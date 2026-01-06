defmodule ScenicDriverSkia.Native do
  use Rustler, otp_app: :scenic_driver_skia, crate: "scenic_driver_skia"

  @doc false
  def start(_backend), do: :erlang.nif_error(:nif_not_loaded)
end
