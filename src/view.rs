use std::cell::OnceCell;
use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DeclaredClass, MainThreadMarker, Message};
use objc2_core_foundation::{CGRect, CGSize};
use objc2_foundation::{NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::CADisplayLink;
use wgpu::rwh::{
    AppKitWindowHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawWindowHandle, UiKitWindowHandle, WindowHandle,
};

use crate::run_loop::queue_closure;
use crate::wgpu_triangle::Triangle;

#[cfg(feature = "mtkview")]
type View = objc2_metal_kit::MTKView;
#[cfg(all(target_os = "macos", not(feature = "mtkview")))]
type View = objc2_app_kit::NSView;
#[cfg(all(not(target_os = "macos"), not(feature = "mtkview")))]
type View = objc2_ui_kit::UIView;

define_class!(
    // SAFETY:
    // - The superclass View does not have any subclassing requirements.
    // - `Delegate` does not implement `Drop`.
    #[unsafe(super(View))]
    #[name = "View"]
    #[ivars = OnceCell<Triangle<'static>>]
    pub struct WgpuTriangleView;

    unsafe impl NSObjectProtocol for WgpuTriangleView {}

    /// NSView
    #[cfg(target_os = "macos")]
    impl WgpuTriangleView {
        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool {
            cfg!(not(feature = "draw-rect"))
        }

        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            tracing::trace!(live_resize = self.inLiveResize(), "triggered `updateLayer`");
            let triangle = self.ivars().get().expect("initialized");
            triangle.redraw();

            if cfg!(feature = "queue-display") {
                let view = self.retain();
                queue_closure(move || view.setNeedsDisplay(true));
            }
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _rect: CGRect) {
            tracing::trace!(live_resize = self.inLiveResize(), "triggered `drawRect:`");
            let triangle = self.ivars().get().expect("initialized");
            triangle.redraw();

            if cfg!(feature = "queue-display") {
                let view = self.retain();
                queue_closure(move || view.setNeedsDisplay(true));
            }

            // No need to call super, it does nothing on `NSView`.
        }

        #[unsafe(method(frameDidChange:))]
        fn frame_did_change(&self, _notification: &objc2_foundation::NSNotification) {
            let new_size = scaled_view_frame(self);
            tracing::debug!(
                live_resize = self.inLiveResize(),
                ?new_size,
                "triggered `frameDidChange:`"
            );
            let triangle = self.ivars().get().expect("initialized");
            triangle.resize(
                new_size.width as u32,
                new_size.height as u32,
                self.window().unwrap().backingScaleFactor() as f32,
            );
            if cfg!(all(
                feature = "immediate-redraw",
                not(feature = "display-link")
            )) {
                triangle.redraw();
            }
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn changed_backing_properties(&self) {
            let new_size = scaled_view_frame(self);
            tracing::debug!(
                live_resize = self.inLiveResize(),
                ?new_size,
                "triggered `viewDidChangeBackingProperties`"
            );
            let triangle = self.ivars().get().expect("initialized");
            triangle.resize(
                new_size.width as u32,
                new_size.height as u32,
                self.window().unwrap().backingScaleFactor() as f32,
            );
            if cfg!(all(
                feature = "immediate-redraw",
                not(feature = "display-link")
            )) {
                triangle.redraw();
            }
        }
    }

    /// UIView
    #[cfg(not(target_os = "macos"))]
    impl WgpuTriangleView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _rect: CGRect) {
            tracing::trace!("triggered `drawRect:`");
            let triangle = self.ivars().get().expect("initialized");
            triangle.redraw();

            if cfg!(feature = "queue-display") {
                let view = self.retain();
                queue_closure(move || view.setNeedsDisplay());
            }

            // No need to call super, it does nothing on `UIView`.
        }

        // `layoutSubviews` is the recommended way to listen for changes to
        // the view's frame. Also tracks changes to the backing scale factor.
        #[unsafe(method(layoutSubviews))]
        fn layout_subviews(&self) {
            let new_size = scaled_view_frame(self);
            tracing::debug!("triggered `layoutSubviews`, new_size: {:?}", new_size);
            let triangle = self.ivars().get().expect("initialized");
            triangle.resize(
                new_size.width as u32,
                new_size.height as u32,
                self.contentScaleFactor() as f32,
            );
            if cfg!(all(
                feature = "immediate-redraw",
                not(feature = "display-link")
            )) {
                triangle.redraw();
            }

            // Calling super here is not really necessary, as we have no
            // subviews, but we do it anyway just to make sure.
            let _: () = unsafe { objc2::msg_send![super(self), layoutSubviews] };
        }
    }

    /// For DisplayLink
    impl WgpuTriangleView {
        #[unsafe(method(step:))]
        fn step(&self, _sender: &CADisplayLink) {
            tracing::trace!("triggered `step:`");
            if cfg!(feature = "immediate-redraw") {
                let triangle = self.ivars().get().expect("initialized");
                triangle.redraw();
            } else {
                #[cfg(target_os = "macos")]
                self.setNeedsDisplay(true);
                #[cfg(not(target_os = "macos"))]
                self.setNeedsDisplay();
            }
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
        let view_ptr: NonNull<View> = NonNull::from(&**self.0);
        let raw = if cfg!(target_os = "macos") {
            RawWindowHandle::AppKit(AppKitWindowHandle::new(view_ptr.cast()))
        } else {
            RawWindowHandle::UiKit(UiKitWindowHandle::new(view_ptr.cast()))
        };
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for ViewWrapper {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, HandleError> {
        let handle = if cfg!(target_os = "macos") {
            DisplayHandle::appkit()
        } else {
            DisplayHandle::uikit()
        };
        Ok(handle)
    }
}

impl WgpuTriangleView {
    pub fn new(mtm: MainThreadMarker, frame_rect: CGRect) -> Retained<Self> {
        // Create view
        let view = mtm.alloc().set_ivars(OnceCell::new());
        let view: Retained<Self> = unsafe { msg_send![super(view), initWithFrame: frame_rect] };

        // Set up wgpu state
        let size = scaled_view_frame(&view);
        let triangle = pollster::block_on(Triangle::new(
            ViewWrapper(view.retain()),
            size.width as u32,
            size.height as u32,
            1.0,
        ));
        if cfg!(feature = "immediate-redraw") {
            triangle.redraw();
        }
        view.ivars().set(triangle).expect("only initialize once");

        // Listen for changes to the size of the view.
        //
        // This is done automatically on iOS with `layoutSubviews`.
        #[cfg(target_os = "macos")]
        {
            view.setPostsFrameChangedNotifications(true);
            let notification_center = objc2_foundation::NSNotificationCenter::defaultCenter();
            unsafe {
                notification_center.addObserver_selector_name_object(
                    &view,
                    sel!(frameDidChange:),
                    Some(objc2_app_kit::NSViewFrameDidChangeNotification),
                    Some(&view),
                );
            }
        }

        // Ensure that the view calls `drawRect:` after being resized
        #[cfg(not(target_os = "macos"))]
        view.setContentMode(objc2_ui_kit::UIViewContentMode::Redraw);

        if cfg!(feature = "display-link") {
            view.redraw_with_displaylink();
        }

        view
    }

    fn redraw_with_displaylink(&self) {
        let display_link =
            unsafe { CADisplayLink::displayLinkWithTarget_selector(self, sel!(step:)) };
        unsafe {
            display_link.addToRunLoop_forMode(&NSRunLoop::currentRunLoop(), NSRunLoopCommonModes)
        };
    }
}

#[cfg(target_os = "macos")]
fn scaled_view_frame(view: &View) -> CGSize {
    view.convertSizeToBacking(view.frame().size)
}

#[cfg(not(target_os = "macos"))]
fn scaled_view_frame(view: &View) -> CGSize {
    let size = view.frame().size;
    let scale_factor = view.contentScaleFactor();
    CGSize {
        width: size.width * scale_factor,
        height: size.height * scale_factor,
    }
}
