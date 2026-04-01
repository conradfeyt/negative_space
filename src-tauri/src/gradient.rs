// gradient.rs — Native macOS clipped sidebar gradient layer.
//
// This path renders the sidebar gradient as an NSImageView inside the main
// window's contentView (below the WKWebView), then updates the image view frame
// continuously so the sampled region tracks screen coordinates.
//
// Why this approach exists:
// - It preserves clipping to app bounds (no desktop leak)
// - It avoids low-frequency moved events by accepting explicit high-rate updates
//   from the frontend rAF loop via `update_native_background_position`
//
// High-rate native frame updates keep the sidebar gradient visually stable
// during window drag while preserving clipping to app bounds.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSImage, NSImageView, NSView, NSWindow, NSWindowOrderingMode};
use objc2_foundation::{NSData, NSPoint, NSRect, NSSize};
use std::sync::Mutex;

/// Raw pointer wrapper. SAFETY: All AppKit access is on the main thread.
struct RawPtr(*mut AnyObject);
unsafe impl Send for RawPtr {}
unsafe impl Sync for RawPtr {}

struct NativeGradientState {
    view: RawPtr,
    main_window: RawPtr,
    origin_x: f64,
    origin_y: f64,
    screen_w: f64,
    screen_h: f64,
}

static GRADIENT_STATE: Mutex<Option<NativeGradientState>> = Mutex::new(None);

/// Focus hooks are currently no-ops for clipped mode (the gradient is attached
/// to the main window and naturally hidden when the app is unfocused/occluded).
pub fn show_background() {
    // No-op in clipped mode.
}

pub fn hide_background() {
    // No-op in clipped mode.
}

/// Remove any existing native gradient subview and clear cached state.
pub fn teardown_background() {
    let mut state = GRADIENT_STATE.lock().unwrap();
    if let Some(existing) = state.take() {
        unsafe {
            let view: &NSView = &*(existing.view.0 as *const NSView);
            view.removeFromSuperview();
        }
    }
}

/// Update the in-window gradient view's frame based on the main window position
/// in logical screen coordinates.
#[tauri::command]
pub fn update_native_background_position(window_x: f64, window_y: f64) -> Result<(), String> {
    let state = GRADIENT_STATE.lock().unwrap();
    let Some(ref s) = *state else {
        return Ok(());
    };

    unsafe {
        let view: &NSImageView = &*(s.view.0 as *const NSImageView);
        let main_win: &NSWindow = &*(s.main_window.0 as *const NSWindow);
        let content_h = main_win
            .contentView()
            .map(|cv| cv.frame().size.height)
            .unwrap_or(s.screen_h);

        // X uses the straightforward inverse offset.
        let ox = -(window_x - s.origin_x);
        // Y converts top-left screen coordinates into contentView's bottom-left
        // coordinate space. This formula previously removed the 2x parallax bug.
        let oy = content_h - s.screen_h + (window_y - s.origin_y);

        view.setFrame(NSRect::new(
            NSPoint::new(ox, oy),
            NSSize::new(s.screen_w, s.screen_h),
        ));
    }

    Ok(())
}

/// Tauri command: receive the sidebar gradient JPEG and install it as a native
/// NSImageView subview below the WKWebView.
#[tauri::command]
pub fn set_native_background(
    app: tauri::AppHandle,
    sidebar_jpeg: String,
    origin_x: f64,
    origin_y: f64,
    screen_w: f64,
    screen_h: f64,
) -> Result<(), String> {
    use base64::Engine;
    use tauri::Manager;

    let sidebar_bytes = base64::engine::general_purpose::STANDARD
        .decode(&sidebar_jpeg)
        .map_err(|e| format!("Failed to decode sidebar JPEG: {}", e))?;

    let window = app.get_webview_window("main").ok_or("No main window")?;
    let ns_window_ptr = window.ns_window().map_err(|e| format!("{}", e))? as *mut AnyObject;

    unsafe {
        teardown_background();

        let main_win: &NSWindow = &*(ns_window_ptr as *const NSWindow);
        let content_view = main_win.contentView().ok_or("No content view")?;
        let mtm = MainThreadMarker::new().ok_or("Not on main thread")?;

        // Build NSImage from JPEG payload.
        let ns_data = NSData::with_bytes(&sidebar_bytes);
        let ns_image = NSImage::initWithData(NSImage::alloc(), &ns_data)
            .ok_or("Failed to create NSImage from JPEG")?;

        // Create the native image view sized to the full virtual screen.
        let image_view = NSImageView::new(mtm);
        image_view.setImage(Some(&ns_image));
        image_view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(screen_w, screen_h)));
        image_view.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::empty());

        // Transparent window/webview so native gradient can show through.
        main_win.setBackgroundColor(Some(&objc2_app_kit::NSColor::clearColor()));
        content_view.setClipsToBounds(false);

        // Insert below existing content (WKWebView is typically first subview).
        let subviews = content_view.subviews();
        if subviews.len() > 0 {
            let first: &NSView = &subviews.objectAtIndex(0);
            content_view.addSubview_positioned_relativeTo(
                &image_view,
                NSWindowOrderingMode::Below,
                Some(first),
            );
        } else {
            content_view.addSubview(&image_view);
        }

        // Initial alignment from current window position.
        let pos = window.outer_position().map_err(|e| format!("{}", e))?;
        let dpr = main_win.backingScaleFactor();
        let win_x = pos.x as f64 / dpr;
        let win_y = pos.y as f64 / dpr;
        let content_h = content_view.frame().size.height;
        let ox = -(win_x - origin_x);
        let oy = content_h - screen_h + (win_y - origin_y);
        image_view.setFrame(NSRect::new(
            NSPoint::new(ox, oy),
            NSSize::new(screen_w, screen_h),
        ));

        // Store state for high-frequency follow-up updates.
        let view_raw = Retained::into_raw(image_view) as *mut AnyObject;
        let mut state = GRADIENT_STATE.lock().unwrap();
        *state = Some(NativeGradientState {
            view: RawPtr(view_raw),
            main_window: RawPtr(ns_window_ptr),
            origin_x,
            origin_y,
            screen_w,
            screen_h,
        });
    }

    Ok(())
}
