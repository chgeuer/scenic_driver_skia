defmodule ScenicDriverSkia.DemoWayland do
  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Primitives
    alias Scenic.Script

    def init(scene, _args, _opts) do
      scene = Scenic.Scene.push_script(scene, build_rrectv_script(), "rrectv_demo")
      graph = build_graph()
      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end

    defp build_graph do
      x1 = 80
      x2 = 880
      x3 = 1680
      y1 = 80
      y2 = 520
      y3 = 960
      label_offset = 190

      Scenic.Graph.build(font_size: 22)
      |> rect({220, 140}, fill: :blue, stroke: {3, :white}, translate: {x1, y1})
      |> text("rect", fill: :white, translate: {x1, y1 + label_offset})
      |> rounded_rectangle({220, 140, 24}, fill: :purple, stroke: {3, :white}, translate: {x2, y1})
      |> text("rrect", fill: :white, translate: {x2, y1 + label_offset})
      |> script("rrectv_demo", translate: {x3, y1})
      |> text("rrectv", fill: :white, translate: {x3, y1 + label_offset})
      |> line({{0, 0}, {220, 140}}, stroke: {4, :white}, translate: {x1, y2})
      |> text("line", fill: :white, translate: {x1, y2 + label_offset})
      |> circle(60, fill: :green, stroke: {3, :white}, translate: {x2 + 110, y2 + 70})
      |> text("circle", fill: :white, translate: {x2, y2 + label_offset})
      |> ellipse({80, 50}, fill: :orange, stroke: {3, :white}, translate: {x3 + 110, y2 + 70})
      |> text("ellipse", fill: :white, translate: {x3, y2 + label_offset})
      |> arc({80, 1.6}, stroke: {6, :white}, translate: {x1 + 110, y3 + 70})
      |> text("arc", fill: :white, translate: {x1, y3 + label_offset})
      |> sector({80, 1.2}, fill: :teal, stroke: {3, :white}, translate: {x2 + 110, y3 + 70})
      |> text("sector", fill: :white, translate: {x2, y3 + label_offset})
      |> text("text", fill: :yellow, font_size: 34, translate: {x3, y3 + 80})
      |> text("text", fill: :white, translate: {x3, y3 + label_offset})
    end

    defp build_rrectv_script do
      Script.start()
      |> Script.fill_color(:navy)
      |> Script.stroke_color(:white)
      |> Script.stroke_width(3)
      |> Script.draw_variable_rounded_rectangle(220, 140, 40, 20, 60, 10, :fill_stroke)
      |> Script.finish()
    end
  end

  def run do
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {2560, 1440},
        default_scene: DemoScene,
        drivers: [
          [
            module: Scenic.Driver.Skia,
            name: :skia_driver,
            backend: :wayland,
            debug: false,
            window: [resizeable: true, title: "Scenic Wayland"]
          ]
        ]
      )

    Process.sleep(:infinity)
  end
end

ScenicDriverSkia.DemoWayland.run()
