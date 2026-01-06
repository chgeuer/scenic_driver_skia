defmodule ScenicDriverSkia do
  @moduledoc """
  Scenic driver wrapper that delegates rendering to a Rust NIF implemented with Rustler.
  """

  alias ScenicDriverSkia.Native

  @doc """
  Start the renderer with the provided backend. Accepts `:wayland`, `:kms`, or `:drm`.
  """
  @spec start(:wayland | :kms | :drm | String.t()) :: :ok | {:error, term()}
  def start(backend \ :wayland) when is_atom(backend) or is_binary(backend) do
    backend
    |> to_string()
    |> Native.start()
    |> normalize_start_result()
  end

  defp normalize_start_result(:ok), do: :ok
  defp normalize_start_result({:ok, _}), do: :ok
  defp normalize_start_result({:error, _} = error), do: error
  defp normalize_start_result(other), do: {:error, {:unexpected_result, other}}
end
