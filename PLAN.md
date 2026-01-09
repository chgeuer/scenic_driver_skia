# Scenic Driver Skia Plan

## Goal
Implement a cache-based Scenic script renderer using Rust/Skia via Rustler, with shared backends (Wayland/DRM/raster) and enough primitives to render demo scripts (rects and text).

## Current Status
Completed:
- Script cache in Rust keyed by script id; renderer replays cached ops per redraw.
- `draw_script` resolves cached sub-scripts with push/pop draw-state stack.
- Parser supports `fill_color`, `translate`, `draw_rect`, and `draw_text`.
- Driver submits scripts by id and deletes stale scripts.
- Raster backend validated with `raster_output`; Wayland/DRM share the same render state.
- Demos updated with colored text; assets module with local fonts + aliases.
- Script ingestion tests and lifecycle coverage added.

## Next Steps
1. **Rendering primitives**
   - Add scale/rotate/transform ops and stroke styling.
   - Expand text styling (font, size, align, baseline) beyond placeholders.
   - Add image rendering and image fill support.

2. **Performance**
   - Consider caching Skia `Picture`s per script id for static sub-graphs.
   - Reduce allocations in script parsing and replay.

3. **Testing**
   - Add integration coverage that renders text/image to raster and asserts output metadata.
   - Add a regression test for `draw_script` recursion guard.

4. **Backend polish**
   - Replace deprecated Skia image encode API in the raster backend.
