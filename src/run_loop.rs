use std::cell::Cell;

use block2::Block;
use core_foundation::{
    base::CFTypeRef,
    runloop::{kCFRunLoopDefaultMode, CFRunLoopGetMain, CFRunLoopRef},
};
use objc2_foundation::MainThreadMarker;

pub fn queue_closure(closure: impl FnOnce() + 'static) {
    extern "C" {
        fn CFRunLoopPerformBlock(rl: CFRunLoopRef, mode: CFTypeRef, block: &Block<dyn Fn()>);
    }

    // Convert `FnOnce()` to `Block<dyn Fn()>`.
    let closure = Cell::new(Some(closure));
    let block = block2::RcBlock::new(move || {
        if let Some(closure) = closure.take() {
            closure()
        } else {
            tracing::error!("tried to execute queued closure on main thread twice");
        }
    });

    let _mtm = MainThreadMarker::new().unwrap();
    // SAFETY: We're on the main thread, so when adding the closure, it will
    // be run on the same thread.
    let run_loop = unsafe { CFRunLoopGetMain() };

    let mode = unsafe { kCFRunLoopDefaultMode as CFTypeRef };
    // SAFETY: The runloop is valid, the mode is a `CFStringRef`, and the block is `'static`.
    unsafe { CFRunLoopPerformBlock(run_loop, mode, &block) }
}
