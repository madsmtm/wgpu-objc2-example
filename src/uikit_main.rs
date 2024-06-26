use std::cell::OnceCell;
use std::ffi::{c_char, c_int};
use std::ptr::NonNull;

use objc2::rc::{Allocated, Retained};
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSStringFromClass};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationMain, UIScreen, UIViewController, UIWindow,
};

use crate::view::WgpuTriangleView;

declare_class!(
    #[derive(Debug)]
    struct ViewController;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is used for UI stuff.
    // - `ViewController` does not implement `Drop`.
    unsafe impl ClassType for ViewController {
        type Super = UIViewController;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "ViewController";
    }

    impl DeclaredClass for ViewController {
        type Ivars = ();
    }

    unsafe impl NSObjectProtocol for ViewController {}
);

impl ViewController {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(());
        unsafe { msg_send_id![super(this), init] }
    }
}

#[derive(Debug)]
struct Ivars {
    window: OnceCell<Retained<UIWindow>>,
}

declare_class!(
    #[derive(Debug)]
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

    unsafe impl Delegate {
        #[method_id(init)]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(Ivars {
                window: OnceCell::new(),
            });
            unsafe { msg_send_id![super(this), init] }
        }
    }

    unsafe impl UIApplicationDelegate for Delegate {
        #[method(applicationDidFinishLaunching:)]
        fn did_finish_launching(&self, _application: &UIApplication) {
            self.setup();
        }
    }
);

impl Delegate {
    fn setup(&self) {
        let mtm = MainThreadMarker::from(self);

        #[allow(deprecated)] // Unsure how else we should do this when setting up?
        let frame = UIScreen::mainScreen(mtm).bounds();

        let window = unsafe { UIWindow::initWithFrame(mtm.alloc(), frame) };
        eprintln!(
            "frame: {:?}, bounds: {:?}",
            window.frame().size,
            window.bounds().size
        );

        let view_controller = ViewController::new(mtm);
        view_controller.setView(Some(&WgpuTriangleView::new(mtm, frame)));

        window.setRootViewController(Some(&view_controller));

        window.makeKeyAndVisible();

        self.ivars()
            .window
            .set(window)
            .expect("can only initialize once");
    }
}

pub fn main(mtm: MainThreadMarker) {
    // These functions are in crt_externs.h.
    extern "C" {
        fn _NSGetArgc() -> *mut c_int;
        fn _NSGetArgv() -> *mut *mut *mut c_char;
    }

    let _ = mtm;
    unsafe {
        UIApplicationMain(
            *_NSGetArgc(),
            NonNull::new(*_NSGetArgv()).unwrap(),
            None,
            Some(NSStringFromClass(Delegate::class()).as_ref()),
        )
    };
}
