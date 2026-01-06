mod backend;
mod kms_backend;
mod renderer;

fn main() {
    match std::env::var("SCENIC_BACKEND").unwrap_or_else(|_| "wayland".into()) {
        backend if backend.eq_ignore_ascii_case("kms") || backend.eq_ignore_ascii_case("drm") => {
            kms_backend::run()
        }
        _ => backend::run(),
    }
}
