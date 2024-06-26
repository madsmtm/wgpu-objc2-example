use std::cell::OnceCell;
use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, sel, ClassType, DeclaredClass};
use objc2_app_kit::{NSView, NSViewFrameDidChangeNotification};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSNotificationCenter, NSObjectProtocol, NSRect,
};
use wgpu::rwh::{
    AppKitWindowHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawWindowHandle, WindowHandle,
};

use crate::wgpu_triangle::Triangle;

declare_class!(
    pub struct WgpuTriangleView;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is used for UI stuff.
    // - `Delegate` does not implement `Drop`.
    unsafe impl ClassType for WgpuTriangleView {
        type Super = NSView;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "View";
    }

    impl DeclaredClass for WgpuTriangleView {
        type Ivars = OnceCell<Triangle<'static>>;
    }

    unsafe impl NSObjectProtocol for WgpuTriangleView {}

    unsafe impl WgpuTriangleView {
        #[method(wantsUpdateLayer)]
        fn wants_update_layer(&self) -> bool {
            cfg!(not(feature = "draw-rect"))
        }

        #[method(updateLayer)]
        fn update_layer(&self) {
            eprintln!("updateLayer");
            let triangle = self.ivars().get().expect("initialized");
            triangle.redraw();
        }

        #[method(drawRect:)]
        fn draw_rect(&self, _rect: NSRect) {
            eprintln!("drawRect:");
            let triangle = self.ivars().get().expect("initialized");
            triangle.redraw();
        }

        #[method(frameDidChange:)]
        fn frame_did_change(&self, _notification: &NSNotification) {
            eprintln!("frameDidChange:");
            let new_size = unsafe { self.convertSizeToBacking(self.frame().size) };

            let triangle = self.ivars().get().expect("initialized");
            triangle.resize(new_size.width as u32, new_size.height as u32);
            #[cfg(feature = "hacky-redraw")]
            triangle.redraw();
        }
    }
);

// Helper for passing the view to `create_surface`.
struct ViewWrapper(Retained<WgpuTriangleView>);

// SAFETY: We only use WGPU from the main thread, which we know because
// `WgpuTriangleView` which is exposed from this module is `!Send + !Sync`.
unsafe impl Send for ViewWrapper {}
unsafe impl Sync for ViewWrapper {}

impl HasWindowHandle for ViewWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let view_ptr: NonNull<NSView> = NonNull::from(&**self.0);
        let raw = RawWindowHandle::AppKit(AppKitWindowHandle::new(view_ptr.cast()));
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for ViewWrapper {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::appkit())
    }
}

impl WgpuTriangleView {
    pub fn new(mtm: MainThreadMarker, frame_rect: NSRect) -> Retained<Self> {
        let view = mtm.alloc().set_ivars(OnceCell::new());
        let view: Retained<Self> = unsafe { msg_send_id![super(view), initWithFrame: frame_rect] };

        let size = unsafe { view.convertSizeToBacking(view.frame().size) };
        let triangle = pollster::block_on(Triangle::new(
            ViewWrapper(view.retain()),
            size.width as u32,
            size.height as u32,
        ));
        #[cfg(feature = "hacky-redraw")]
        triangle.redraw();
        view.ivars().set(triangle).expect("only initialize once");

        view.setPostsFrameChangedNotifications(true);
        let notification_center = unsafe { NSNotificationCenter::defaultCenter() };
        unsafe {
            notification_center.addObserver_selector_name_object(
                &view,
                sel!(frameDidChange:),
                Some(NSViewFrameDidChangeNotification),
                Some(&view),
            );
        }

        view
    }
}
