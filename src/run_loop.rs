use std::cell::Cell;

use objc2::MainThreadMarker;
use objc2_core_foundation::{kCFRunLoopDefaultMode, CFRunLoop};

pub fn queue_closure(closure: impl FnOnce() + 'static) {
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
    let run_loop = CFRunLoop::main().unwrap();

    let mode = unsafe { kCFRunLoopDefaultMode };
    // SAFETY: The runloop is valid, and the block is `'static`.
    //
    // Additionally, we're on the main thread, so when adding the closure, it
    // will be run on the same thread.
    unsafe { run_loop.perform_block(mode.map(|mode| &**mode), Some(&block)) }
}
