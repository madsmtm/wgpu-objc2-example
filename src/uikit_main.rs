use std::cell::OnceCell;

use objc2::rc::{Allocated, Retained};
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIScreen, UIStackView, UIStackViewDistribution,
    UIViewController, UIWindow,
};

use crate::view::WgpuTriangleView;

define_class!(
    // SAFETY:
    // - The superclass UIViewController does not have any subclassing requirements.
    // - `ViewController` does not implement `Drop`.
    #[unsafe(super(UIViewController))]
    #[name = "ViewController"]
    #[derive(Debug)]
    struct ViewController;

    unsafe impl NSObjectProtocol for ViewController {}
);

impl ViewController {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

#[derive(Debug)]
struct Ivars {
    window: OnceCell<Retained<UIWindow>>,
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
    #[derive(Debug)]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    /// Called by UIApplicationMain
    impl Delegate {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(Ivars {
                window: OnceCell::new(),
            });
            unsafe { msg_send![super(this), init] }
        }
    }

    unsafe impl UIApplicationDelegate for Delegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _application: &UIApplication) {
            tracing::info!("applicationDidFinishLaunching:");
            self.setup();
        }
    }
);

impl Delegate {
    fn setup(&self) {
        let mtm = MainThreadMarker::from(self);

        #[allow(deprecated)] // Unsure how else we should do this when setting up?
        let frame = UIScreen::mainScreen(mtm).bounds();

        #[allow(deprecated)]
        let window = UIWindow::initWithFrame(mtm.alloc(), frame);
        eprintln!(
            "frame: {:?}, bounds: {:?}",
            window.frame().size,
            window.bounds().size
        );

        let view_controller = ViewController::new(mtm);

        if cfg!(feature = "two-triangles") {
            // Frame will be resized by NSStackView automatically
            let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
            let view = UIStackView::new(mtm);
            view.addArrangedSubview(&WgpuTriangleView::new(mtm, frame));
            view.addArrangedSubview(&WgpuTriangleView::new(mtm, frame));
            // view.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
            view.setDistribution(UIStackViewDistribution::FillEqually);
            view_controller.setView(Some(&view));
        } else {
            view_controller.setView(Some(&WgpuTriangleView::new(mtm, frame)));
        }

        window.setRootViewController(Some(&view_controller));

        window.makeKeyAndVisible();

        self.ivars()
            .window
            .set(window)
            .expect("can only initialize once");
    }
}

pub fn main(mtm: MainThreadMarker) {
    UIApplication::main(None, Some(&NSString::from_class(Delegate::class())), mtm)
}
