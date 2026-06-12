// ══════════════════════════════════════════════════════════════
// 桌面小宠物 — main.rs (Slint 1.16 in-out API)
// ══════════════════════════════════════════════════════════════

mod audio;
mod db;
mod hit_test;
mod state_machine;
mod tts;

slint::include_modules!();

use chrono::{Datelike, Local, Timelike};
use slint::{ComponentHandle, ModelRc};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::db::DbState;
use crate::state_machine::{PetState, PetStateMachine, StateEvent};

struct AppCtx {
    pet: PetStateMachine,
    bubble: Mutex<String>,
    bubble_reset: Mutex<Option<Instant>>,
    tts: Mutex<tts::TtsManager>,
    mute: Mutex<bool>,
    rest: Mutex<bool>,
    last_hour: Mutex<i32>,
    db: DbState,
}

fn main() {
    env_logger::init();
    let ui = MainWindow::new().expect("Failed to create Slint window");
    configure_window(&ui);

    let db = DbState::new();
    db.init_tables().expect("Failed to init database");

    let ctx = Arc::new(AppCtx {
        pet: PetStateMachine::new(),
        bubble: Mutex::new(String::new()),
        bubble_reset: Mutex::new(None),
        tts: Mutex::new(tts::TtsManager::new()),
        mute: Mutex::new(false),
        rest: Mutex::new(false),
        last_hour: Mutex::new(-1),
        db,
    });

    // ── Callback: window_init (one-shot, fires after event loop starts) ──
    let ui_s = ui.clone_strong();
    ui.on_window_init(move || {
        #[cfg(target_os = "windows")]
        style_window(&ui_s);
    });

    // ── Callback: cat_clicked → random bubble ──
    let c3 = ctx.clone();
    ui.on_cat_clicked(move || {
        let msgs = ["喵~","今天也要加油哦!","休息一下吧~","有什么需要帮忙的吗?","摸摸头 >w<","记得喝水哦!","喵呜~"];
        let idx = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % msgs.len() as u128) as usize;
        *c3.bubble.lock().unwrap() = msgs[idx].to_string();
        *c3.bubble_reset.lock().unwrap() = Some(Instant::now());
    });

    // ── Callback: toggle_rest ──
    let c4 = ctx.clone();
    ui.on_toggle_rest(move || {
        let mut rest = c4.rest.lock().unwrap();
        *rest = !*rest;
        let mut b = c4.bubble.lock().unwrap();
        if *rest {
            c4.pet.set_state(PetState::Sleeping, 0, "");
            *b = "进入休息模式".to_string();
        } else {
            c4.pet.set_state(PetState::Idle, 0, "");
            *b = "我回来了!".to_string();
        }
        *c4.bubble_reset.lock().unwrap() = Some(Instant::now());
    });

    // ── Callback: toggle_mute ──
    let c5 = ctx.clone();
    ui.on_toggle_mute(move || {
        let mut muted = c5.mute.lock().unwrap();
        *muted = !*muted;
        let mut b = c5.bubble.lock().unwrap();
        *b = if *muted { "已静音".to_string() } else { "已取消静音".to_string() };
        *c5.bubble_reset.lock().unwrap() = Some(Instant::now());
    });

    // ── Callback: task_tab_changed ──
    let c6 = ctx.clone();
    let ui_w6 = ui.as_weak();
    ui.on_task_tab_changed(move |filter: slint::SharedString| {
        if let Some(ui) = ui_w6.upgrade() {
            let f: String = filter.into();
            refresh_task_list(&ui, &c6.db, &f);
        }
    });

    // ── Callback: task_complete ──
    let c7 = ctx.clone();
    let ui_w7 = ui.as_weak();
    ui.on_task_complete(move |id: i32, title: slint::SharedString| {
        let t: String = title.into();
        if let Some(ui) = ui_w7.upgrade() {
            c7.db.complete_task(id as i64).ok();
            let info = c7.pet.transition(StateEvent::TaskCompleted);
            ui.set_cat_state(state_name(&info.state).into());
            let msg = format!("好棒! {} 完成啦!", t);
            *c7.bubble.lock().unwrap() = msg.clone();
            *c7.bubble_reset.lock().unwrap() = Some(Instant::now());
            if let Ok(mut tt) = c7.tts.lock() { tt.speak(&tts::task_completed_text(&t)); }
            refresh_task_list(&ui, &c7.db, &String::from(ui.get_task_filter()));
        }
    });

    // ── Callback: task_delete ──
    let c8 = ctx.clone();
    let ui_w8 = ui.as_weak();
    ui.on_task_delete(move |id: i32| {
        if let Some(ui) = ui_w8.upgrade() {
            c8.db.delete_task(id as i64).ok();
            refresh_task_list(&ui, &c8.db, &String::from(ui.get_task_filter()));
        }
    });

    // ── Callback: add_saved ──
    let c9 = ctx.clone();
    let ui_w9 = ui.as_weak();
    ui.on_add_saved(move || {
        if let Some(ui) = ui_w9.upgrade() {
            let title: String = ui.get_add_title().into();
            if title.trim().is_empty() { return; }
            let m: String = ui.get_add_month().into();
            let d: String = ui.get_add_day().into();
            let h: String = ui.get_add_hour().into();
            let mi: String = ui.get_add_minute().into();
            let due = format!("{:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:00",
                chrono::Local::now().year(), m.parse::<i32>().unwrap_or(6), d.parse::<i32>().unwrap_or(12),
                h.parse::<i32>().unwrap_or(12), mi.parse::<i32>().unwrap_or(0));
            let remind = ui.get_add_remind();

            c9.db.add_task(&title, &String::from(ui.get_add_desc()), &due, remind as i64).ok();
            let info = c9.pet.transition(StateEvent::TaskAdded);
            ui.set_cat_state(state_name(&info.state).into());

            let msg = format!("收到新任务: {}", title);
            *c9.bubble.lock().unwrap() = msg.clone();
            *c9.bubble_reset.lock().unwrap() = Some(Instant::now());
            if let Ok(mut tt) = c9.tts.lock() { tt.speak(&tts::task_added_text(&title)); }

            refresh_task_list(&ui, &c9.db, &String::from(ui.get_task_filter()));
        }
    });

    // ── Callback: save_settings ──
    let c10 = ctx.clone();
    let ui_w10 = ui.as_weak();
    ui.on_save_settings(move || {
        if let Some(ui) = ui_w10.upgrade() {
            let pairs = [
                ("auto_start", ui.get_set_autostart().to_string()),
                ("edge_snap", ui.get_set_edgesnap().to_string()),
                ("pet_transparency", ui.get_set_opacity().to_string()),
                ("hourly_enabled", ui.get_set_hourly().to_string()),
                ("hourly_start_hour", ui.get_set_h_start().to_string()),
                ("hourly_end_hour", ui.get_set_h_end().to_string()),
                ("voice_enabled", ui.get_set_voice().to_string()),
                ("tts_volume", ui.get_set_volume().to_string()),
            ];
            for (k, v) in pairs {
                c10.db.update_setting(k, &v).ok();
            }
        }
    });

    // ── Callback: exit ──
    ui.on_app_exit(|| std::process::exit(0));

    // ── Load initial data ──
    load_settings(&ui, &ctx);
    refresh_task_list(&ui, &ctx.db, "pending");

    // ── Background thread: periodic state update ──
    let ui_w = ui.as_weak();
    let ctx_t = ctx.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        if let Some(ui) = ui_w.upgrade() {
            // Update cat state
            let info = ctx_t.pet.get_state();
            ui.set_cat_state(state_name(&info.state).into());

            // Update bubble text
            let bubble = ctx_t.bubble.lock().unwrap().clone();
            let reset = ctx_t.bubble_reset.lock().unwrap();
            let show = reset.is_some() && reset.unwrap().elapsed() < Duration::from_secs(3);
            if !show && !bubble.is_empty() {
                *ctx_t.bubble.lock().unwrap() = String::new();
                *ctx_t.bubble_reset.lock().unwrap() = None;
                ui.set_bubble_text("".into());
                ui.set_bubble_visible(false);
            } else if show {
                ui.set_bubble_text(bubble.into());
                ui.set_bubble_visible(true);
            }
        }
    });

    // ── Background thread: hourly check ──
    let ctx_h = ctx.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        hourly_check(&ctx_h);
    });

    // ── Run ──
    if let Err(e) = ui.run() { eprintln!("Error: {:?}", e); }
}

// ── Window config ─────────────────────────────────────────────

fn configure_window(ui: &MainWindow) { position_center(ui); }

fn position_center(ui: &MainWindow) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;
        use windows::Win32::UI::WindowsAndMessaging::SPI_GETWORKAREA;
        use windows::Win32::Foundation::RECT;
        use slint::PhysicalPosition;

        let mut rect = RECT::default();
        let ok = unsafe {
            SystemParametersInfoW(SPI_GETWORKAREA, 0,
                Some(&mut rect as *mut _ as *mut _),
                windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
        };
        if ok.is_ok() {
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            let _ = ui.window().set_position(PhysicalPosition::new(
                ((w - 156) / 2).max(0), ((h - 220) / 2).max(0)));
        } else {
            let _ = ui.window().set_position(PhysicalPosition::new(200, 200));
        }
    }
    #[cfg(not(target_os = "windows"))]
    { let _ = ui.window().set_position(slint::PhysicalPosition::new(200, 200)); }
}

#[cfg(target_os = "windows")]
fn style_window(ui: &MainWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos,
        GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE,
        WS_CAPTION, WS_THICKFRAME, WS_MINIMIZEBOX, WS_MAXIMIZEBOX, WS_SYSMENU,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_APPWINDOW,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let win = ui.window();
    let handle = win.window_handle();
    if let Ok(rwh) = handle.window_handle() {
        if let RawWindowHandle::Win32(win32) = rwh.as_raw() {
            use windows::Win32::Foundation::HWND;
            let hwnd = HWND(win32.hwnd.get() as *mut _);
            unsafe {
                let mut s = GetWindowLongPtrW(hwnd, GWL_STYLE);
                s &= !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0) as isize;
                let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, s);
                let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                ex |= (WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0) as isize;
                ex &= !(WS_EX_APPWINDOW.0 as isize);
                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
                {
                    use windows::Win32::UI::Controls::MARGINS;
                    use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
                    let m = MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 };
                    let _ = DwmExtendFrameIntoClientArea(hwnd, &m);
                }
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0,
                    SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE);
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────

fn refresh_task_list(ui: &MainWindow, db: &DbState, filter: &str) {
    ui.set_task_filter(filter.into());
    if let Ok(tasks) = db.get_tasks(filter) {
        let items: Vec<TaskItem> = tasks.iter().map(|t| TaskItem {
            id: t.id as i32,
            title: t.title.clone().into(),
            due_label: format_due(&t.due_time).into(),
            is_completed: t.is_completed,
            is_urgent: !t.is_completed && is_overdue(&t.due_time),
        }).collect();
        ui.set_task_list(ModelRc::from(Rc::new(slint::VecModel::from(items))));
    }
}

fn load_settings(ui: &MainWindow, ctx: &AppCtx) {
    if let Ok(settings) = ctx.db.get_settings() {
        for s in settings {
            match s.key.as_str() {
                "auto_start" => ui.set_set_autostart(s.value == "true"),
                "edge_snap" => ui.set_set_edgesnap(s.value == "true"),
                "pet_transparency" => ui.set_set_opacity(s.value.parse().unwrap_or(100)),
                "hourly_enabled" => ui.set_set_hourly(s.value == "true"),
                "hourly_start_hour" => ui.set_set_h_start(s.value.parse().unwrap_or(7)),
                "hourly_end_hour" => ui.set_set_h_end(s.value.parse().unwrap_or(22)),
                "voice_enabled" => ui.set_set_voice(s.value == "true"),
                "tts_volume" => ui.set_set_volume(s.value.parse().unwrap_or(80)),
                _ => {}
            }
        }
    }
}

fn state_name(s: &PetState) -> String {
    match s {
        PetState::Idle => "idle", PetState::Remind => "remind",
        PetState::Warning => "warning", PetState::Sad => "sad",
        PetState::Recover => "recover", PetState::Happy => "happy",
        PetState::Sleeping => "sleeping",
    }.to_string()
}

fn format_due(s: &str) -> String {
    use chrono::NaiveDateTime;
    let Ok(due) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") else { return String::new(); };
    let diff = due.signed_duration_since(chrono::Local::now().naive_local());
    if diff.num_seconds() < 0 { "已过期".into() }
    else if diff.num_hours() < 1 { format!("{}分钟", diff.num_minutes()) }
    else if diff.num_days() < 1 { format!("{}小时", diff.num_hours()) }
    else { format!("{}/{}", due.month(), due.day()) }
}

fn is_overdue(s: &str) -> bool {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|due| due < chrono::Local::now().naive_local())
        .unwrap_or(false)
}

fn hourly_check(ctx: &AppCtx) {
    use chrono::{Local, Timelike};
    let now = Local::now();
    if now.minute() != 0 { return; }
    let hour = now.hour() as i32;
    let mut last = ctx.last_hour.lock().unwrap();
    if hour == *last { return; }
    let enabled = ctx.db.get_setting("hourly_enabled").unwrap_or_else(|| "true".into());
    if enabled != "true" { return; }
    let start: i32 = ctx.db.get_setting("hourly_start_hour").unwrap_or_else(|| "7".into()).parse().unwrap_or(7);
    let end: i32 = ctx.db.get_setting("hourly_end_hour").unwrap_or_else(|| "22".into()).parse().unwrap_or(22);
    if hour < start || hour > end { return; }
    *last = hour;

    let greetings = [(6,"早安! 新的一天开始啦"),(7,"早安! 新的一天开始啦"),(8,"早安! 新的一天开始啦"),
        (9,"上午好! 继续加油"),(10,"上午好! 继续加油"),(11,"快中午了~"),(12,"中午啦! 该吃饭了"),
        (13,"午休一下吧"),(14,"下午好! 继续加油"),(15,"下午好! 继续加油"),(16,"快下班了~"),
        (17,"傍晚好"),(18,"晚上好~"),(19,"晚上好~"),(20,"晚上好~ 放松一下吧"),
        (21,"不早了~"),(22,"很晚了，早点休息吧"),(23,"深夜了，注意休息")];
    let msg = greetings.iter().find(|(h,_)| *h == hour).map(|(_,m)| *m).unwrap_or("整点了");
    *ctx.bubble.lock().unwrap() = msg.to_string();
    *ctx.bubble_reset.lock().unwrap() = Some(Instant::now());

    let voice = ctx.db.get_setting("voice_enabled").unwrap_or_else(|| "true".into());
    if voice == "true" && !*ctx.mute.lock().unwrap() {
        if let Ok(mut t) = ctx.tts.lock() { t.speak(&tts::hourly_text(hour as u32)); }
    }
}
