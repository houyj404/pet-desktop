// ══════════════════════════════════════════════════════════════
// 桌面小宠物 — hit_test.rs
// 平台级点击穿透: 仅猫咪区域接收鼠标事件, 其余区域穿透
//
//   Windows: WM_NCHITTEST subclassing (WS_EX_TRANSPARENT toggle)
//   macOS:   NSWindow.setIgnoresMouseEvents (TODO)
//   Linux:   X11 input shape / wlr-layer-shell (TODO)
// ══════════════════════════════════════════════════════════════

#[cfg(target_os = "windows")]
mod platform {
    use std::sync::Mutex;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE,
        GWL_WNDPROC, HTCLIENT, HTTRANSPARENT, WM_NCHITTEST,
        WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    struct HitState {
        enabled: bool,
        rect: RECT,
    }

    static mut ORIG_PROC: isize = 0;
    static mut HWND_STATIC: isize = 0;
    static HIT: Mutex<HitState> = Mutex::new(HitState {
        enabled: true,
        rect: RECT { left: 10, top: 10, right: 130, bottom: 170 },
    });

    pub fn init(window: &dyn HasWindowHandle) {
        let raw = window.window_handle().unwrap();
        if let RawWindowHandle::Win32(h) = raw.as_raw() {
            unsafe {
                let hwnd = HWND(h.hwnd.get() as *mut _);
                let orig = GetWindowLongPtrW(hwnd, GWL_WNDPROC);
                ORIG_PROC = orig;
                HWND_STATIC = hwnd.0 as isize;

                // Set WS_EX_LAYERED + WS_EX_TRANSPARENT initially
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    style | WS_EX_TRANSPARENT.0 as isize | WS_EX_LAYERED.0 as isize,
                );
                SetWindowLongPtrW(hwnd, GWL_WNDPROC, wndproc as *const () as isize);
            }
            log::info!("Hit-test subclass installed");
        }
    }

    pub fn set_rect(x: i32, y: i32, w: i32, h: i32) {
        let mut st = HIT.lock().unwrap();
        st.rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
    }

    pub fn set_enabled(on: bool) {
        let mut st = HIT.lock().unwrap();
        st.enabled = on;
        drop(st);
        unsafe {
            if HWND_STATIC != 0 {
                let hwnd = HWND(HWND_STATIC as *mut _);
                toggle_transparent(hwnd, on);
            }
        }
    }

    unsafe fn toggle_transparent(hwnd: HWND, transparent: bool) {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if transparent {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize);
        } else {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_TRANSPARENT.0 as isize));
        }
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_NCHITTEST {
            let st = HIT.lock().unwrap();
            if !st.enabled {
                drop(st);
                // Full interactive mode
                toggle_transparent(hwnd, false);
                return LRESULT(HTCLIENT as isize);
            }

            let mut pt = POINT {
                x: (lparam.0 & 0xFFFF) as i32,
                y: (lparam.0 >> 16) as i32,
            };
            let _ = ScreenToClient(hwnd, &mut pt);

            let hit = pt.x >= st.rect.left
                && pt.x <= st.rect.right
                && pt.y >= st.rect.top
                && pt.y <= st.rect.bottom;
            drop(st);

            if hit {
                toggle_transparent(hwnd, false);
                LRESULT(HTCLIENT as isize)
            } else {
                toggle_transparent(hwnd, true);
                LRESULT(HTTRANSPARENT as isize)
            }
        } else {
            CallWindowProcW(
                Some(std::mem::transmute(ORIG_PROC)),
                hwnd,
                msg,
                wparam,
                lparam,
            )
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub fn init(_window: &dyn raw_window_handle::HasWindowHandle) {
        log::warn!("Hit-test not yet implemented on this platform");
    }
    pub fn set_rect(_x: i32, _y: i32, _w: i32, _h: i32) {}
    pub fn set_enabled(_on: bool) {}
}

// ── Public API ───────────────────────────────────────────────

pub use platform::*;
