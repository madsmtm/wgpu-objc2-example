#![deny(unsafe_op_in_unsafe_fn)]
use objc2_foundation::MainThreadMarker;

#[cfg(target_os = "macos")]
mod appkit_main;
#[cfg(not(target_os = "macos"))]
mod uikit_main;
mod view;
mod wgpu_triangle;

fn main() {
    let mtm = MainThreadMarker::new().unwrap();

    #[cfg(target_os = "macos")]
    appkit_main::main(mtm);
    #[cfg(not(target_os = "macos"))]
    uikit_main::main(mtm);
}
