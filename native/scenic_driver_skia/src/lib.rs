mod backend;
mod kms_backend;
mod raster_backend;
mod renderer;

use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use backend::UserEvent;
use renderer::RenderState;

enum StopSignal {
    Wayland(winit::event_loop::EventLoopProxy<UserEvent>),
    Drm(Arc<AtomicBool>),
    Raster(Arc<AtomicBool>),
}

struct DriverHandle {
    stop: StopSignal,
    text: Arc<Mutex<String>>,
    render_state: Arc<Mutex<RenderState>>,
    raster_output: Option<Arc<Mutex<Option<String>>>>,
    dirty: Option<Arc<AtomicBool>>,
    running: Arc<AtomicBool>,
    thread: thread::JoinHandle<()>,
}

static DRIVER: OnceLock<Mutex<Option<DriverHandle>>> = OnceLock::new();

fn driver_state() -> &'static Mutex<Option<DriverHandle>> {
    DRIVER.get_or_init(|| Mutex::new(None))
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn start(backend: Option<String>) -> Result<(), String> {
    let backend = backend
        .map(|b| b.to_lowercase())
        .map(|b| if b == "kms" { String::from("drm") } else { b })
        .unwrap_or_else(|| String::from("wayland"));

    let mut state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;

    if let Some(handle) = state.as_ref() {
        match &handle.stop {
            StopSignal::Wayland(proxy) => {
                if handle.running.load(Ordering::Relaxed) {
                    return Err("renderer already running".to_string());
                }
                handle.running.store(true, Ordering::Relaxed);
                let result = proxy
                    .send_event(UserEvent::Start)
                    .map_err(|err| format!("failed to signal renderer: {err}"));
                if result.is_err() {
                    handle.running.store(false, Ordering::Relaxed);
                }
                return result;
            }
            StopSignal::Drm(_) | StopSignal::Raster(_) => {
                return Err("renderer already running".to_string());
            }
        }
    }

    let thread_name = format!("scenic-driver-{backend}");
    let text = Arc::new(Mutex::new(String::from("Hello, Wayland")));
    let render_state = Arc::new(Mutex::new(RenderState::default()));
    let running = Arc::new(AtomicBool::new(true));
    let handle = if backend == "drm" {
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(false));
        let text_for_thread = Arc::clone(&text);
        let state_for_thread = Arc::clone(&render_state);
        let dirty_for_thread = Arc::clone(&dirty);
        let stop_for_thread = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                kms_backend::run(
                    stop_for_thread,
                    text_for_thread,
                    dirty_for_thread,
                    state_for_thread,
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Drm(stop),
            text,
            render_state,
            raster_output: None,
            dirty: Some(dirty),
            running,
            thread,
        }
    } else if backend == "raster" {
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(false));
        let state_for_thread = Arc::clone(&render_state);
        let dirty_for_thread = Arc::clone(&dirty);
        let stop_for_thread = Arc::clone(&stop);
        let raster_output = Arc::new(Mutex::new(None));
        let output_for_thread = Arc::clone(&raster_output);
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                raster_backend::run(
                    stop_for_thread,
                    dirty_for_thread,
                    state_for_thread,
                    output_for_thread,
                )
            })
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        DriverHandle {
            stop: StopSignal::Raster(stop),
            text,
            render_state,
            raster_output: Some(raster_output),
            dirty: Some(dirty),
            running,
            thread,
        }
    } else {
        let (proxy_tx, proxy_rx) = mpsc::channel();
        let initial_text = text
            .lock()
            .map_err(|_| "driver state lock poisoned".to_string())?
            .clone();
        let running_for_thread = Arc::clone(&running);
        let initial_state = render_state
            .lock()
            .map_err(|_| "driver state lock poisoned".to_string())?
            .clone();
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || backend::run(proxy_tx, initial_text, running_for_thread, initial_state))
            .map_err(|err| format!("failed to spawn renderer thread: {err}"))?;
        let proxy = proxy_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "renderer did not initialize in time".to_string())?;
        DriverHandle {
            stop: StopSignal::Wayland(proxy),
            text,
            render_state,
            raster_output: None,
            dirty: None,
            running,
            thread,
        }
    };

    *state = Some(handle);

    Ok(())
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn stop() -> Result<(), String> {
    let mut state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;

    if !handle.running.load(Ordering::Relaxed) {
        return Ok(());
    }

    let signal_result = match &handle.stop {
        StopSignal::Wayland(proxy) => proxy
            .send_event(UserEvent::Stop)
            .map_err(|err| format!("failed to signal renderer: {err}")),
        StopSignal::Drm(stop) => {
            stop.store(true, Ordering::Relaxed);
            Ok(())
        }
        StopSignal::Raster(stop) => {
            stop.store(true, Ordering::Relaxed);
            Ok(())
        }
    };
    handle.running.store(false, Ordering::Relaxed);

    match &handle.stop {
        StopSignal::Wayland(_) => signal_result,
        StopSignal::Drm(_) | StopSignal::Raster(_) => {
            let handle = state.take().expect("handle checked");
            handle
                .thread
                .join()
                .map_err(|_| "renderer thread panicked".to_string())?;
            signal_result
        }
    }
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_text(text: String) -> Result<(), String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;

    {
        let mut stored = handle
            .text
            .lock()
            .map_err(|_| "text state lock poisoned".to_string())?;
        *stored = text.clone();
    }

    match &handle.stop {
        StopSignal::Wayland(proxy) => proxy
            .send_event(UserEvent::SetText(text))
            .map_err(|err| format!("failed to signal renderer: {err}")),
        StopSignal::Drm(_) | StopSignal::Raster(_) => {
            if let Some(dirty) = &handle.dirty {
                dirty.store(true, Ordering::Relaxed);
            }
            Ok(())
        }
    }
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn reset_scene() -> Result<(), String> {
    update_render_state(|state| {
        state.rect = None;
        state.translate = (0.0, 0.0);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_clear_color(color: (u8, u8, u8, u8)) -> Result<(), String> {
    update_render_state(|state| {
        state.clear_color = skia_safe::Color::from_argb(color.3, color.0, color.1, color.2);
        Ok(())
    })
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn submit_script(script: rustler::Binary) -> Result<(), String> {
    update_render_state(|state| {
        parse_script(script.as_slice(), state)?;
        Ok(())
    })?;
    Ok(())
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn set_raster_output(path: String) -> Result<(), String> {
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;
    let output = handle
        .raster_output
        .as_ref()
        .ok_or_else(|| "raster backend not active".to_string())?;

    let mut slot = output
        .lock()
        .map_err(|_| "raster output lock poisoned".to_string())?;
    *slot = Some(path);

    if let Some(dirty) = &handle.dirty {
        dirty.store(true, Ordering::Relaxed);
    }

    Ok(())
}

fn update_render_state<F>(mut update: F) -> Result<(), String>
where
    F: FnMut(&mut RenderState) -> Result<(), String>,
{
    let state = driver_state()
        .lock()
        .map_err(|_| "driver state lock poisoned".to_string())?;
    let handle = state
        .as_ref()
        .ok_or_else(|| "renderer not running".to_string())?;

    let mut render_state = handle
        .render_state
        .lock()
        .map_err(|_| "render state lock poisoned".to_string())?;
    update(&mut render_state)?;
    let render_state_snapshot = *render_state;
    drop(render_state);

    match &handle.stop {
        StopSignal::Wayland(proxy) => proxy
            .send_event(UserEvent::SetRenderState(render_state_snapshot))
            .map_err(|err| format!("failed to signal renderer: {err}")),
        StopSignal::Drm(_) | StopSignal::Raster(_) => {
            if let Some(dirty) = &handle.dirty {
                dirty.store(true, Ordering::Relaxed);
            }
            Ok(())
        }
    }
}

fn parse_script(script: &[u8], state: &mut RenderState) -> Result<(), String> {
    let mut rest = script;
    let mut translate = state.translate;
    let mut translate_stack: Vec<(f32, f32)> = Vec::new();
    while rest.len() >= 2 {
        let (op, remaining) = rest.split_at(2);
        let opcode = u16::from_be_bytes([op[0], op[1]]);
        rest = remaining;
        match opcode {
            0x0f => {
                if rest.len() < 2 {
                    return Err("draw_script opcode truncated".to_string());
                }
                let (len_bytes, tail) = rest.split_at(2);
                let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                let pad = (4 - (len % 4)) % 4;
                let total = len + pad;
                if tail.len() < total {
                    return Err("draw_script payload truncated".to_string());
                }
                rest = &tail[total..];
            }
            0x40 => {
                if rest.len() < 2 {
                    return Err("push_state opcode truncated".to_string());
                }
                translate_stack.push(translate);
                rest = &rest[2..];
            }
            0x41 => {
                if rest.len() < 2 {
                    return Err("pop_state opcode truncated".to_string());
                }
                translate = translate_stack.pop().unwrap_or((0.0, 0.0));
                rest = &rest[2..];
            }
            0x42 => {
                if rest.len() < 2 {
                    return Err("pop_push_state opcode truncated".to_string());
                }
                translate = translate_stack.pop().unwrap_or((0.0, 0.0));
                translate_stack.push(translate);
                rest = &rest[2..];
            }
            0x60 => {
                if rest.len() < 6 {
                    return Err("fill_color opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (rgba, tail) = tail.split_at(4);
                state.fill_color = skia_safe::Color::from_argb(rgba[3], rgba[0], rgba[1], rgba[2]);
                rest = tail;
            }
            0x53 => {
                if rest.len() < 10 {
                    return Err("translate opcode truncated".to_string());
                }
                let (_reserved, tail) = rest.split_at(2);
                let (x_bytes, tail) = tail.split_at(4);
                let (y_bytes, tail) = tail.split_at(4);
                let x = f32::from_bits(u32::from_be_bytes([
                    x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3],
                ]));
                let y = f32::from_bits(u32::from_be_bytes([
                    y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3],
                ]));
                translate = (x, y);
                rest = tail;
            }
            0x04 => {
                if rest.len() < 10 {
                    return Err("draw_rect opcode truncated".to_string());
                }
                let (flag_bytes, tail) = rest.split_at(2);
                let flag = u16::from_be_bytes([flag_bytes[0], flag_bytes[1]]);
                let (w_bytes, tail) = tail.split_at(4);
                let (h_bytes, tail) = tail.split_at(4);
                let width = f32::from_bits(u32::from_be_bytes([
                    w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3],
                ]));
                let height = f32::from_bits(u32::from_be_bytes([
                    h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3],
                ]));
                if flag & 0x01 == 0x01 {
                    state.rect = Some(skia_safe::Rect::from_xywh(
                        translate.0,
                        translate.1,
                        width,
                        height,
                    ));
                }
                rest = tail;
            }
            _ => {
                return Err(format!("unsupported opcode: 0x{opcode:02x}"));
            }
        }
    }
    state.translate = translate;
    Ok(())
}

rustler::init!("Elixir.ScenicDriverSkia.Native");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fill_and_rect() {
        let script: [u8; 20] = [
            0x00, 0x60, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x04, 0x00, 0x01, 0x42, 0x20,
            0x00, 0x00, 0x41, 0xA0, 0x00, 0x00,
        ];
        let mut state = RenderState::default();
        state.rect = None;
        parse_script(&script, &mut state).expect("parse_script failed");

        assert_eq!(
            state.fill_color,
            skia_safe::Color::from_argb(0xFF, 0xFF, 0x00, 0x00)
        );
        assert_eq!(
            state.rect,
            Some(skia_safe::Rect::from_xywh(0.0, 0.0, 40.0, 20.0))
        );
    }

    #[test]
    fn parse_rejects_truncated_fill_color() {
        let script: [u8; 4] = [0x00, 0x60, 0x00, 0x00];
        let mut state = RenderState::default();
        let err = parse_script(&script, &mut state).unwrap_err();
        assert!(err.contains("fill_color opcode truncated"));
    }

    #[test]
    fn parse_rejects_truncated_rect() {
        let script: [u8; 6] = [0x00, 0x04, 0x00, 0x01, 0x00, 0x00];
        let mut state = RenderState::default();
        let err = parse_script(&script, &mut state).unwrap_err();
        assert!(err.contains("draw_rect opcode truncated"));
    }

    #[test]
    fn parse_rejects_unknown_opcode() {
        let script: [u8; 2] = [0x12, 0x34];
        let mut state = RenderState::default();
        let err = parse_script(&script, &mut state).unwrap_err();
        assert!(err.contains("unsupported opcode"));
    }

    #[test]
    fn parse_translate_affects_rect() {
        let script: [u8; 40] = [
            0x00, 0x40, 0x00, 0x00, 0x00, 0x53, 0x00, 0x00, 0x42, 0x48, 0x00, 0x00, 0x42, 0x70,
            0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x04, 0x00, 0x01,
            0x41, 0x20, 0x00, 0x00, 0x41, 0xA0, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00,
        ];
        let mut state = RenderState::default();
        parse_script(&script, &mut state).expect("parse_script failed");

        assert_eq!(
            state.rect,
            Some(skia_safe::Rect::from_xywh(50.0, 60.0, 10.0, 20.0))
        );
    }

    #[test]
    fn parse_skips_draw_script() {
        let mut script: Vec<u8> = vec![0x00, 0x0f, 0x00, 0x04];
        script.extend_from_slice(b"root");
        script.extend_from_slice(&[
            0x00, 0x60, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x04, 0x00, 0x01, 0x41, 0x20,
            0x00, 0x00, 0x41, 0xA0, 0x00, 0x00,
        ]);

        let mut state = RenderState::default();
        parse_script(&script, &mut state).expect("parse_script failed");
        assert!(state.rect.is_some());
    }
}
