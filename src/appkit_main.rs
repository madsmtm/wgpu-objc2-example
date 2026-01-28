use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType,
    NSStackView, NSStackViewDistribution, NSUserInterfaceLayoutOrientation, NSWindow,
    NSWindowStyleMask,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};

use crate::view::WgpuTriangleView;

#[derive(Debug)]
struct Ivars {
    window: OnceCell<Retained<NSWindow>>,
}

define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is used for UI stuff.
    // - `Delegate` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "Delegate"]
    #[ivars = Ivars]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSApplicationDelegate for Delegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            tracing::info!("applicationDidFinishLaunching:");
            self.setup();
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn should_terminate_after_last_window_closed(&self, _sender: &NSApplication) -> bool {
            tracing::info!("applicationShouldTerminateAfterLastWindowClosed:");
            true
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(Ivars {
            window: OnceCell::new(),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn setup(&self) {
        let mtm = MainThreadMarker::from(self);

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        #[allow(deprecated)] // New method `activate` is only available on macOS 14.0
        app.activateIgnoringOtherApps(false); // Useful when the application is not bundled

        let window = {
            let content_rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(1024., 768.));
            let style = NSWindowStyleMask::Closable
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Titled;
            let backing_store_type = NSBackingStoreType::Buffered;
            let flag = false;
            unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc(),
                    content_rect,
                    style,
                    backing_store_type,
                    flag,
                )
            }
        };
        // Important for memory safety!
        unsafe { window.setReleasedWhenClosed(false) };

        if cfg!(feature = "two-triangles") {
            // Frame will be resized by NSStackView automatically
            let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
            let view = NSStackView::new(mtm);
            view.addArrangedSubview(&WgpuTriangleView::new(mtm, frame));
            view.addArrangedSubview(&WgpuTriangleView::new(mtm, frame));
            view.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
            view.setDistribution(NSStackViewDistribution::FillEqually);
            window.setContentView(Some(&view));
        } else {
            let frame = window.contentView().expect("window content view").frame();
            let view = WgpuTriangleView::new(mtm, frame);
            window.setContentView(Some(&view));
        }

        window.center();
        window.makeKeyAndOrderFront(None);

        self.ivars()
            .window
            .set(window)
            .expect("can only initialize once");
    }
}

pub fn main(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    let delegate = Delegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
}
