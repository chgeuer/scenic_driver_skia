defmodule Scenic.Driver.Skia.InputEventsTest do
  use ExUnit.Case, async: false

  alias Scenic.Driver.Skia.Native
  alias Scenic.Driver.Skia.TestSupport.ViewPort, as: ViewPortHelper
  alias Scenic.ViewPort

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

  defp wait_for_frame!(attempts_remaining) do
    case Native.get_raster_frame() do
      {:ok, {width, height, frame}} ->
        {width, height, frame}

      _ when attempts_remaining > 0 ->
        Process.sleep(50)
        wait_for_frame!(attempts_remaining - 1)

      other ->
        flunk("timed out waiting for raster frame: #{inspect(other)}")
    end
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
