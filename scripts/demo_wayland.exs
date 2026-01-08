defmodule ScenicDriverSkia.DemoWayland do
  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({200, 120}, fill: :blue, translate: {50, 50})

      scene = Scenic.Scene.push_graph(scene, graph)
      {:ok, scene}
    end
  end

  def run do
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {400, 300},
        default_scene: DemoScene,
        drivers: [[module: ScenicDriverSkia.Driver, name: :skia_driver, backend: :wayland]]
      )

    Process.sleep(:infinity)
  end
end

ScenicDriverSkia.DemoWayland.run()
