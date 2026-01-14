defmodule Scenic.Driver.Skia.InputEventsTest do
  use ExUnit.Case, async: false

  alias Scenic.Driver.Skia.Native
  alias Scenic.Driver.Skia.TestSupport.ViewPort, as: ViewPortHelper
  alias Scenic.ViewPort

  defmodule RasterScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({140, 80}, fill: :red, translate: {20, 20})

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  test "drains input events while raster backend is running" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)
    ensure_renderer_stopped()

    case Native.start("raster", nil, "Scenic Window", false, nil, true, false) do
      :ok -> :ok
      {:ok, _} -> :ok
      other -> flunk("start returned #{inspect(other)}")
    end

    on_exit(fn ->
      _ = Native.stop()
    end)

    case Native.set_input_mask(0x01) do
      :ok -> :ok
      {:ok, _} -> :ok
      other -> flunk("set_input_mask returned #{inspect(other)}")
    end

    case Native.drain_input_events() do
      [] -> :ok
      {:ok, []} -> :ok
      other -> flunk("drain_input_events returned #{inspect(other)}")
    end
  end

  test "raster output matches viewport size" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    viewport_size = {321, 123}

    vp = ViewPortHelper.start(size: viewport_size)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {width, height, frame} = wait_for_frame!(40)
    assert {width, height} == viewport_size
    assert byte_size(frame) == width * height * 3
  end

  test "raster output contains drawn content" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {200, 120}, scene: RasterScene)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {_width, _height, frame} =
      wait_for_frame!(40, fn {_w, _h, data} ->
        Enum.any?(:binary.bin_to_list(data), &(&1 > 0))
      end)

    assert Enum.any?(:binary.bin_to_list(frame), &(&1 > 0))
  end

  test "cursor visibility toggles are accepted while renderer is running" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)
    ensure_renderer_stopped()

    case Native.start("raster", nil, "Scenic Window", false, nil, true, false) do
      :ok -> :ok
      {:ok, _} -> :ok
      other -> flunk("start returned #{inspect(other)}")
    end

    on_exit(fn ->
      _ = Native.stop()
    end)

    assert :ok = Scenic.Driver.Skia.hide_cursor()
    assert :ok = Scenic.Driver.Skia.show_cursor()
  end

  defp wait_for_frame!(attempts_remaining),
    do: wait_for_frame!(attempts_remaining, fn _ -> true end)

  defp wait_for_frame!(attempts_remaining, predicate) do
    case Native.get_raster_frame() do
      {:ok, {width, height, frame}} = ok ->
        if predicate.({width, height, frame}) do
          {width, height, frame}
        else
          retry_frame(ok, attempts_remaining, predicate)
        end

      other ->
        retry_frame(other, attempts_remaining, predicate)
    end
  end

  defp retry_frame(_last_result, attempts_remaining, predicate) when attempts_remaining > 0 do
    Process.sleep(50)
    wait_for_frame!(attempts_remaining - 1, predicate)
  end

  defp retry_frame(last_result, _attempts_remaining, _predicate) do
    flunk("timed out waiting for raster frame: #{inspect(last_result)}")
  end

  defp ensure_renderer_stopped do
    case Native.stop() do
      :ok -> :ok
      {:ok, _} -> :ok
      {:error, "renderer not running"} -> :ok
      {:error, _} -> :ok
      _ -> :ok
    end
  end
end
