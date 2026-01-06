# Agent Guidelines

This repository contains a Rust project. Please follow these conventions when modifying files in this repo:

- Run `cargo fmt` before committing changes.
- Run `cargo check` to ensure the code compiles.
- Prefer small, focused modules. Keep rendering logic backend-agnostic and isolate backend/windowing concerns in their own modules.
- Avoid adding unnecessary dependencies; prefer the standard library where practical.
- Document test commands you run in your final summary, noting any failures or external blockers.

These instructions apply to all files in this repository.

Backends:
- `SCENIC_BACKEND=wayland` (default) renders through a Wayland window.
- `SCENIC_BACKEND=kms` or `SCENIC_BACKEND=drm` renders directly on Linux DRM/KMS hardware (tested with AMD GPUs).
  - Override the DRM device path with `SCENIC_DRM_CARD` (defaults to `/dev/dri/card0`).
