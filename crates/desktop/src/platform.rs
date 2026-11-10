//! Native operations. Permission checks never replace per-session consent.

pub const PRODUCT_NAME: &str = "YT Remote 3.0";

/// Call only following the student's explicit ALLOW SCREEN & KEYBOARD action,
/// before creating capture/input objects or approving network transmission.
/// A missing OS grant fails closed; another explicit attempt is required.
pub fn ensure_capture_permissions() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::ensure_capture_permissions()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

/// Minimize the local viewer on its UI thread, preserving its taskbar/Dock entry.
pub fn minimize(window: &minifb::Window) -> Result<(), String> {
    use minifb::HasWindowHandle;
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::RawWindowHandle;
        #[link(name = "user32")]
        unsafe extern "system" {
            fn ShowWindow(window: *mut std::ffi::c_void, command: i32) -> i32;
            fn IsWindow(window: *mut std::ffi::c_void) -> i32;
        }
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err("The viewer does not have a Windows window handle.".into());
        };
        let hwnd = handle.hwnd.get() as *mut std::ffi::c_void;
        // SAFETY: the borrowed minifb handle remains alive through this call.
        unsafe {
            if IsWindow(hwnd) == 0 {
                return Err("The viewer window is no longer available.".into());
            }
            // SW_MINIMIZE. ShowWindow's result is previous visibility, not success.
            ShowWindow(hwnd, 6);
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        macos::minimize(handle.as_raw())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = handle;
        Err("Native minimize is supported only on Windows and macOS.".into())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::ptr;

    use objc2::{msg_send, runtime::AnyObject};
    use raw_window_handle::RawWindowHandle;

    type CFTypeRef = *const c_void;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
    }
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
        static kAXTrustedCheckOptionPrompt: CFTypeRef;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFBooleanTrue: CFTypeRef;
        fn CFDictionaryCreate(
            allocator: CFTypeRef,
            keys: *const CFTypeRef,
            values: *const CFTypeRef,
            count: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFTypeRef;
        fn CFRelease(value: CFTypeRef);
    }
    unsafe extern "C" {
        fn pthread_main_np() -> i32;
    }

    pub(super) fn ensure_capture_permissions() -> Result<(), String> {
        // SAFETY: these public OS APIs query/request this process's grants only.
        // No capture, event injection, TCC database changes or grant bypass occurs.
        unsafe {
            if !CGPreflightScreenCaptureAccess() {
                let _ = CGRequestScreenCaptureAccess();
                if !CGPreflightScreenCaptureAccess() {
                    return Err("Screen Recording permission is required. Open System Settings > Privacy & Security > Screen Recording (or Screen & System Audio Recording), enable YT Remote 3.0, and restart the app if macOS asks. Then connect again and explicitly allow sharing.".into());
                }
            }
            if !AXIsProcessTrusted() {
                let key = kAXTrustedCheckOptionPrompt;
                let value = kCFBooleanTrue;
                // Both entries are process-lifetime OS constants; null callbacks
                // deliberately avoid unnecessary ownership of those constants.
                let options =
                    CFDictionaryCreate(ptr::null(), &key, &value, 1, ptr::null(), ptr::null());
                if options.is_null() {
                    return Err("Unable to request Accessibility access. Open System Settings > Privacy & Security > Accessibility and enable YT Remote 3.0, then explicitly allow sharing again.".into());
                }
                // The system prompt is asynchronous; its return is not approval.
                let _ = AXIsProcessTrustedWithOptions(options);
                CFRelease(options);
                if !AXIsProcessTrusted() {
                    return Err("Accessibility permission is required for remote keyboard and mouse control. Open System Settings > Privacy & Security > Accessibility, enable YT Remote 3.0, then connect again and explicitly allow sharing. Restart the app if macOS asks.".into());
                }
            }
            // Third and last, explicitly requested by the product owner. This
            // requests the OS grant only: no event tap, global key listener or
            // keystroke recorder is installed by this application.
            if !CGPreflightListenEventAccess() {
                let _ = CGRequestListenEventAccess();
                if !CGPreflightListenEventAccess() {
                    return Err("Input Monitoring permission is required by this app's permission setup. Open System Settings > Privacy & Security > Input Monitoring, enable YT Remote 3.0, and restart if macOS asks. Then connect again and explicitly allow sharing.".into());
                }
            }
        }
        Ok(())
    }

    pub(super) fn minimize(handle: RawWindowHandle) -> Result<(), String> {
        // AppKit operations must run on the main thread.
        if unsafe { pthread_main_np() } == 0 {
            return Err("Minimize must run on the app's main UI thread.".into());
        }
        let RawWindowHandle::AppKit(handle) = handle else {
            return Err("The viewer does not have a macOS view handle.".into());
        };
        // SAFETY: minifb 0.28's HasWindowHandle supplies its live NSView, not
        // NSWindow. Ask the NSView for its owning NSWindow using public AppKit.
        unsafe {
            let view = handle.ns_view.as_ptr().cast::<AnyObject>();
            let window: *mut AnyObject = msg_send![view, window];
            if window.is_null() {
                return Err("The viewer is not attached to a macOS window.".into());
            }
            let _: () = msg_send![window, miniaturize: ptr::null::<AnyObject>()];
        }
        Ok(())
    }
}
