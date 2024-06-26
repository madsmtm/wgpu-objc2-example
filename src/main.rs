#![deny(unsafe_op_in_unsafe_fn)]
use objc2_foundation::MainThreadMarker;
use tracing_subscriber::filter::EnvFilter;

#[cfg(target_os = "macos")]
mod appkit_main;
#[cfg(not(target_os = "macos"))]
mod uikit_main;
mod view;
mod wgpu_triangle;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("wgpu_objc2_example=info".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    let mtm = MainThreadMarker::new().unwrap();

    #[cfg(target_os = "macos")]
    appkit_main::main(mtm);
    #[cfg(not(target_os = "macos"))]
    uikit_main::main(mtm);
}
