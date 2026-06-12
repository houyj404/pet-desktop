// ══════════════════════════════════════════════════════════════
// Win32 WM_NCHITTEST 子类化 — 实现像素级点击穿透
// ══════════════════════════════════════════════════════════════

use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE,
    GWL_WNDPROC, HTCLIENT, HTTRANSPARENT, WM_NCHITTEST,
    WS_EX_LAYERED, WS_EX_TRANSPARENT,
};
use windows::Win32::Graphics::Gdi::ScreenToClient;

struct HitTestState {
    enabled: bool,
    pet_rect: RECT,
}

static ORIG_WNDPROC: OnceLock<isize> = OnceLock::new();
static HIT_STATE: OnceLock<Mutex<HitTestState>> = OnceLock::new();
static HWND_GLOBAL: OnceLock<isize> = OnceLock::new();

fn state() -> &'static Mutex<HitTestState> {
    HIT_STATE.get_or_init(|| {
        Mutex::new(HitTestState {
            enabled: true,
            pet_rect: RECT { left: 10, top: 10, right: 130, bottom: 170 },
        })
    })
}

/// 动态切换 WS_EX_TRANSPARENT 标志
unsafe fn set_window_transparent(hwnd: HWND, transparent: bool) {
    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if transparent {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize);
    } else {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_TRANSPARENT.0 as isize));
    }
}

unsafe extern "system" fn hit_test_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let orig = *ORIG_WNDPROC.get().unwrap();

    if msg == WM_NCHITTEST {
        let st = state().lock().unwrap();
        if !st.enabled {
            drop(st);
            set_window_transparent(hwnd, false);
            return LRESULT(HTCLIENT as isize);
        }

        // 屏幕坐标 → 窗口客户区坐标
        let mut pt = POINT {
            x: (lparam.0 & 0xFFFF) as i32,
            y: (lparam.0 >> 16) as i32,
        };
        let _ = ScreenToClient(hwnd, &mut pt);

        let over_pet = pt.x >= st.pet_rect.left
            && pt.x <= st.pet_rect.right
            && pt.y >= st.pet_rect.top
            && pt.y <= st.pet_rect.bottom;
        drop(st);

        if over_pet {
            // 鼠标在宠物上：移除穿透标志，窗口接收事件
            set_window_transparent(hwnd, false);
            return LRESULT(HTCLIENT as isize);
        } else {
            // 鼠标在透明区域：添加穿透标志，事件透传到后面窗口
            set_window_transparent(hwnd, true);
            return LRESULT(HTTRANSPARENT as isize);
        }
    }

    CallWindowProcW(Some(std::mem::transmute(orig)), hwnd, msg, wparam, lparam)
}

pub fn subclass(hwnd: HWND) {
    unsafe {
        let orig = GetWindowLongPtrW(hwnd, GWL_WNDPROC);
        let _ = ORIG_WNDPROC.set(orig);
        let _ = HWND_GLOBAL.set(hwnd.0 as isize);

        // 初始设置为穿透状态（启动时透明区域不拦截鼠标）
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize | WS_EX_LAYERED.0 as isize);

        SetWindowLongPtrW(hwnd, GWL_WNDPROC, hit_test_wndproc as *const () as isize);
    }
    log::info!("Hit-test WndProc subclassed");
}

pub fn set_pet_rect(x: i32, y: i32, w: i32, h: i32) {
    let mut st = state().lock().unwrap();
    st.pet_rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
    log::info!("Hit-test pet rect: {:?}", st.pet_rect);
}

pub fn set_enabled(on: bool) {
    let mut st = state().lock().unwrap();
    st.enabled = on;
    drop(st);
    // 切换时立即更新窗口穿透状态
    if let Some(&raw) = HWND_GLOBAL.get() {
        let hwnd = HWND(raw as *mut _);
        unsafe {
            set_window_transparent(hwnd, on);
        }
    }
    log::info!("Hit-test enabled: {}", on);
}
