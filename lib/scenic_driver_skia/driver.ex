defmodule ScenicDriverSkia.Driver do
  @moduledoc """
  Minimal Scenic driver that logs viewport callbacks.

  This is scaffolding for the Skia-backed driver. Configure it on a ViewPort:

      drivers: [
        [
          module: ScenicDriverSkia.Driver,
          name: :skia_driver,
          backend: :raster
        ]
      ]
  """

  use Scenic.Driver
  require Logger

  alias ScenicDriverSkia.Native
  alias Scenic.{Script, ViewPort}

  @window_schema [
    title: [type: :string, default: "Scenic Window"],
    resizeable: [type: :boolean, default: false]
  ]

  @opts_schema [
    backend: [type: {:or, [:atom, :string]}, default: :wayland],
    debug: [type: :boolean, default: false],
    raster_output: [type: :string],
    window: [type: :keyword_list, keys: @window_schema, default: []]
  ]

  @impl Scenic.Driver
  def validate_opts(opts) do
    with {:ok, opts} <- NimbleOptions.validate(opts, @opts_schema) do
      {:ok, Keyword.put(opts, :backend, normalize_backend(opts[:backend]))}
    end
  end

  @impl Scenic.Driver
  def init(driver, opts) do
    Logger.info("ScenicDriverSkia.Driver init: #{inspect(opts)}")

    case Native.start(opts[:backend]) do
      :ok ->
        maybe_set_raster_output(opts)
        {:ok, assign(driver, opts: opts, update_count: 0)}

      {:ok, _} ->
        maybe_set_raster_output(opts)
        {:ok, assign(driver, opts: opts, update_count: 0)}

      {:error, reason} ->
        {:stop, reason}

      other ->
        {:stop, {:unexpected_start_result, other}}
    end
  end

  @impl Scenic.Driver
  def reset_scene(driver) do
    Logger.debug("ScenicDriverSkia.Driver reset_scene")
    _ = Native.reset_scene()
    {:ok, driver}
  end

  @impl Scenic.Driver
  def request_input(input, driver) do
    Logger.debug("ScenicDriverSkia.Driver request_input: #{inspect(input)}")
    {:ok, driver}
  end

  @impl Scenic.Driver
  def update_scene(script_ids, %{viewport: vp} = driver) do
    Logger.debug("ScenicDriverSkia.Driver update_scene: #{inspect(script_ids)}")

    updates =
      Enum.reduce(script_ids, [], fn id, acc ->
        case ViewPort.get_script(vp, id) do
          {:ok, script} ->
            binary =
              script
              |> Script.serialize()
              |> IO.iodata_to_binary()

            [{to_string(id), binary} | acc]

          _ ->
            acc
        end
      end)

    case updates do
      [] ->
        :ok

      _ ->
        Native.submit_scripts(updates)
        |> case do
          :ok -> :ok
          {:ok, _} -> :ok
          {:error, reason} -> Logger.warning("submit_scripts failed: #{inspect(reason)}")
          other -> Logger.warning("submit_scripts returned #{inspect(other)}")
        end
    end

    driver = maybe_log_script_count(driver)
    {:ok, driver}
  end

  @impl Scenic.Driver
  def del_scripts(script_ids, driver) do
    Logger.debug("ScenicDriverSkia.Driver del_scripts: #{inspect(script_ids)}")
    Enum.each(script_ids, &Native.del_script(to_string(&1)))
    {:ok, driver}
  end

  @impl Scenic.Driver
  def clear_color(color, driver) do
    Logger.debug("ScenicDriverSkia.Driver clear_color: #{inspect(color)}")
    {:color_rgba, {r, g, b, a}} = Scenic.Color.to_rgba(color)
    _ = Native.set_clear_color({r, g, b, a})
    {:ok, driver}
  end

  @impl GenServer
  def terminate(_reason, _driver) do
    _ = Native.stop()
    :ok
  end

  defp maybe_set_raster_output(opts) do
    case opts[:raster_output] do
      nil ->
        :ok

      path ->
        case Native.set_raster_output(path) do
          :ok -> :ok
          {:ok, _} -> :ok
          {:error, reason} -> Logger.warning("set_raster_output failed: #{inspect(reason)}")
          other -> Logger.warning("set_raster_output returned #{inspect(other)}")
        end
    end
  end

  defp normalize_backend(backend) do
    backend
    |> to_string()
    |> String.downcase()
  end

  defp maybe_log_script_count(%{assigns: %{opts: opts, update_count: count}} = driver) do
    count = count + 1
    driver = assign(driver, :update_count, count)

    if opts[:debug] && rem(count, 60) == 0 do
      case Native.script_count() do
        {:ok, total} ->
          Logger.info("ScenicDriverSkia.Driver cached scripts: #{total}")

        total when is_integer(total) ->
          Logger.info("ScenicDriverSkia.Driver cached scripts: #{total}")

        {:error, reason} ->
          Logger.warning("script_count failed: #{inspect(reason)}")

        other ->
          Logger.warning("script_count returned #{inspect(other)}")
      end
    end

    driver
  end
end
