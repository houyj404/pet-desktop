pub mod audio;
pub mod commands;
pub mod db;
pub mod hit_test;
pub mod state_machine;
pub mod tts;

use db::DbState;
use state_machine::PetStateMachine;
use tauri::Manager; // needed for app.state(), get_webview_window()
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const SNAP_THRESHOLD: i32 = 30;
const SNAP_DEBOUNCE_MS: u64 = 500;

pub fn run() {
    tauri::Builder::default()
        .manage(DbState::new())
        .manage(PetStateMachine::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_tasks,
            commands::add_task,
            commands::complete_task,
            commands::delete_task,
            commands::get_pet_state,
            commands::set_pet_state,
            commands::get_settings,
            commands::update_setting,
            commands::speak_text,
            commands::play_sound,
            commands::pet_pet,
            commands::hour_reached,
            commands::set_hit_rect,
            commands::set_hit_enabled,
        ])
        .setup(|app| {
            // 初始化数据库
            let db = app.state::<DbState>();
            db.init_tables().expect("Failed to init database");

            // Win32 子类化：实现 WM_NCHITTEST 点击穿透
            let win = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "windows")]
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                let handle = win.window_handle().expect("failed to get window handle");
                if let RawWindowHandle::Win32(h) = handle.as_raw() {
                    let hwnd = windows::Win32::Foundation::HWND(h.hwnd.get() as *mut _);
                    hit_test::subclass(hwnd);
                }
            }

            // 贴边吸附：窗口停止移动后自动吸附到屏幕边缘
            let snapping = Arc::new(AtomicBool::new(false));
            let win_clone = win.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::Moved(_pos) = event {
                    if snapping.load(Ordering::Relaxed) {
                        return;
                    }
                    let snapping = snapping.clone();
                    let win = win_clone.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(SNAP_DEBOUNCE_MS));
                        if snapping.load(Ordering::Relaxed) {
                            return;
                        }
                        snap_to_edge(&win, &snapping);
                    });
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn snap_to_edge(win: &tauri::WebviewWindow, snapping: &Arc<AtomicBool>) {
    let pos = match win.outer_position() {
        Ok(p) => p,
        Err(_) => return,
    };
    let size = match win.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };
    let monitor = match win.current_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };

    let scale = monitor.scale_factor();
    let mx = monitor.position().x as f64 / scale;
    let my = monitor.position().y as f64 / scale;
    let mw = monitor.size().width as f64 / scale;
    let mh = monitor.size().height as f64 / scale;

    let wx = pos.x as f64 / scale;
    let wy = pos.y as f64 / scale;
    let ww = size.width as f64 / scale;
    let wh = size.height as f64 / scale;

    let left_dist = (wx - mx).abs();
    let right_dist = ((mx + mw) - (wx + ww)).abs();
    let top_dist = (wy - my).abs();
    let bottom_dist = ((my + mh) - (wy + wh)).abs();

    let mut new_x = wx;
    let mut new_y = wy;
    let mut snapped = false;

    if left_dist < SNAP_THRESHOLD as f64 {
        new_x = mx;
        snapped = true;
    } else if right_dist < SNAP_THRESHOLD as f64 {
        new_x = mx + mw - ww;
        snapped = true;
    }

    if top_dist < SNAP_THRESHOLD as f64 {
        new_y = my;
        snapped = true;
    } else if bottom_dist < SNAP_THRESHOLD as f64 {
        new_y = my + mh - wh;
        snapped = true;
    }

    if snapped {
        snapping.store(true, Ordering::Relaxed);
        let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(new_x, new_y)));
        std::thread::spawn({
            let snapping = snapping.clone();
            move || {
                std::thread::sleep(std::time::Duration::from_millis(100));
                snapping.store(false, Ordering::Relaxed);
            }
        });
    }
}
