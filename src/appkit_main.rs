use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType,
    NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
};

use crate::view::WgpuTriangleView;

#[derive(Debug)]
struct Ivars {
    window: OnceCell<Retained<NSWindow>>,
}

declare_class!(
    struct Delegate;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is used for UI stuff.
    // - `Delegate` does not implement `Drop`.
    unsafe impl ClassType for Delegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "Delegate";
    }

    impl DeclaredClass for Delegate {
        type Ivars = Ivars;
    }

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSApplicationDelegate for Delegate {
        #[method(applicationDidFinishLaunching:)]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            self.setup();
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(Ivars {
            window: OnceCell::new(),
        });
        unsafe { msg_send_id![super(this), init] }
    }

    fn setup(&self) {
        let mtm = MainThreadMarker::from(self);

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        unsafe { app.activate() }; // Useful when the application is not bundled

        let window = {
            let content_rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(1024., 768.));
            let style = NSWindowStyleMask::Closable
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Titled;
            let backing_store_type = NSBackingStoreType::NSBackingStoreBuffered;
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

        let view = WgpuTriangleView::new(
            mtm,
            window.contentView().expect("window content view").frame(),
        );

        window.setContentView(Some(&view));

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
    unsafe { app.run() };
}
