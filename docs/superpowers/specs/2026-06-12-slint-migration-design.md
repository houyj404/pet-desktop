# 桌面小宠物 — Slint 重构设计文档

> 版本: 2.0 | 日期: 2026-06-12 | 状态: 待实施

## 1. 概述

### 1.1 项目定位

桌面小宠物 — 一个跨平台（Windows/macOS/Linux）桌面陪伴应用。显示一只橘色卡通猫咪，浮动在桌面上，兼具**任务管理器**和**情绪陪伴**功能。

### 1.2 技术栈迁移

| 维度 | v1 (当前) | v2 (目标) |
|---|---|---|
| UI 框架 | Tauri v2 + WebView2 | Slint (原生渲染) |
| 前端语言 | Vanilla JS/HTML/CSS | Slint 声明式 (.slint) |
| 后端语言 | Rust | Rust (不变) |
| 猫咪渲染 | 纯 CSS DOM 元素 | PNG 图片 + Slint 动画 |
| TTS | PowerShell → SAPI | `tts` crate (平台原生) |
| 音频 | PowerShell → MediaPlayer | `rodio` crate |
| 点击穿透 | Win32 WM_NCHITTEST | 平台抽象层 (Win/Mac/Linux) |
| 数据库 | rusqlite (不变) | rusqlite (不变) |
| 状态机 | Rust state machine (不变) | Rust state machine (不变) |

### 1.3 为什么要迁移

1. **跨平台**: 当前 TTS、音频、点击穿透全部依赖 Windows 专有 API
2. **轻量化**: 移除 WebView 运行时，二进制从 ~8MB 降至 ~5MB，内存占用更低
3. **类型安全**: Slint 编译期检查 UI 属性绑定，消除大量运行时错误
4. **无 JS 疲劳**: 不需要处理 CSS 兼容性、DOM 操作、JS 模块化
5. **图片方案**: 猫咪改为 PNG 图片后，WebView 的 CSS 动画优势不再关键

---

## 2. 架构设计

### 2.1 整体架构图

```
┌─────────────────────────────────────────────┐
│                 main.rs                      │
│  窗口创建 · 平台初始化 · 事件循环             │
├─────────────────────────────────────────────┤
│              ui/main.slint                    │
│  ┌──────────┐ ┌──────────┐ ┌─────────────┐  │
│  │ PetArea  │ │  Modals  │ │ ContextMenu │  │
│  │ CatImage │ │ TaskList │ │   MenuItems │  │
│  │ Bubble   │ │ AddForm  │ │             │  │
│  │ Popup    │ │ Settings │ │             │  │
│  └──────────┘ └──────────┘ └─────────────┘  │
├─────────────────────────────────────────────┤
│                 app.rs                        │
│  Callback 绑定 · 状态协调 · 定时器             │
├────────┬──────────┬───────────┬──────────────┤
│  db.rs │state_    │  tts.rs   │  audio.rs    │
│ SQLite │machine   │ 跨平台TTS │  rodio       │
│        │.rs       │           │              │
├────────┴──────────┴───────────┴──────────────┤
│              hit_test.rs                       │
│  Win: WM_NCHITTEST · Mac: NSView · Linux: X11 │
└─────────────────────────────────────────────┘
```

### 2.2 数据流

```
用户操作 (Slint UI)
  │ callback('add_task', {title, due, ...})
  ▼
app.rs::on_add_task()
  │ db.insert()
  │ pet.transition(TaskAdded)
  │ tts.speak(...)
  │
  ▼
更新 Slint 属性:
  ui.set_cat_state("remind")
  ui.set_bubble_text("收到新任务!")
  ui.set_tasks(task_list)
  │
  ▼
Slint 自动重绘:
  CatImage 切换图片 + 动画
  SpeechBubble 淡入
  TaskList 刷新
```

**核心改进**: 不再使用轮询。状态变化后直接 `set_xxx()` 更新 Slint 属性，UI 自动响应。

---

## 3. 项目结构

```
pet-desktop/
├── Cargo.toml
├── build.rs                       # slint-build 编译 .slint → Rust
├── assets/
│   ├── cat_idle.png               # 7 张猫咪状态图 (PNG, 透明背景)
│   ├── cat_remind.png
│   ├── cat_warning.png
│   ├── cat_sad.png
│   ├── cat_recover.png
│   ├── cat_happy.png
│   └── cat_sleeping.png
├── ui/
│   └── main.slint                 # 全部 UI 定义 (~500行)
├── src/
│   ├── main.rs                    # 入口 · 窗口创建 · 事件循环 (~80行)
│   ├── app.rs                     # Callback 绑定 · 状态协调 (~250行)
│   ├── state_machine.rs           # 宠物状态机 (复用 v1)
│   ├── db.rs                      # SQLite CRUD (复用 v1, 去 Tauri)
│   ├── tts.rs                     # 跨平台 TTS (tts crate)
│   ├── audio.rs                   # 跨平台音频 (rodio crate)
│   └── hit_test.rs                # 点击穿透平台抽象
```

### 3.1 文件职责

| 文件 | 职责 | 行数估算 |
|---|---|---|
| `main.rs` | Slint 窗口创建、透明/置顶/无边框配置、事件循环启动 | ~80 |
| `app.rs` | 全局状态持有、所有 Slint callback 实现、定时器管理 | ~250 |
| `ui/main.slint` | 声明式 UI: 组件树 + 属性绑定 + 动画 + callback 声明 | ~500 |
| `state_machine.rs` | 7 状态 × 11 事件状态机, `Mutex<PetStateInfo>` | ~135 (不变) |
| `db.rs` | rusqlite, `Mutex<Connection>`, Task CRUD, Settings CRUD | ~175 (微调) |
| `tts.rs` | `tts` crate 封装, 跨平台 speak/volume/rate | ~60 |
| `audio.rs` | `rodio` crate 封装, WAV/BEEP 播放 | ~50 |
| `hit_test.rs` | 平台抽象: Win WM_NCHITTEST / Mac NSView / Linux X11 | ~150 |

---

## 4. Slint UI 设计

### 4.1 组件树

```
MainWindow (Window)
├── PetArea (always visible, bottom-right)
│   ├── SpeechBubble (conditional: bubble_text != "")
│   │   └── Text
│   ├── CatImage (state-driven image + animation)
│   │   ├── Image (PNG by state)
│   │   ├── ZzzOverlay (visible when sleeping)
│   │   └── SparkleOverlay (visible when happy)
│   └── TaskPopup (conditional: hover on cat)
│       ├── Text "待办事项"
│       ├── VerticalBox (max 3 task items)
│       └── HorizontalBox (buttons: + 添加, 全部)
│
├── TaskModal (conditional: task_modal_open)
│   ├── ModalHeader "我的任务" + CloseButton
│   ├── TabBar (未完成 | 已完成 | 全部)
│   ├── ListView (scrollable task items)
│   └── ModalFooter (+ 添加任务 button)
│
├── AddTaskModal (conditional: add_modal_open)
│   ├── ModalHeader "添加任务" + CloseButton
│   ├── FormBody
│   │   ├── TextInput (任务名称)
│   │   ├── DateTimeInput (截止时间)  ← 注意: Slint 无原生 datetime picker
│   │   ├── TextArea (备注)
│   │   └── ComboBox (提前提醒: 5/15/30/60分钟)
│   └── ModalFooter (取消 | 保存)
│
├── SettingsModal (conditional: settings_modal_open)
│   ├── ModalHeader "设置" + CloseButton
│   ├── TabBar (常规 | 提醒 | 宠物)
│   └── PanelStack (3 panels)
│       ├── GeneralPanel: auto_start switch, edge_snap switch, opacity slider
│       ├── ReminderPanel: hourly switch, start/end SpinBox
│       └── PetPanel: voice switch, volume slider
│
└── ContextMenu (conditional: context_menu_open)
    ├── MenuItem "添加任务"
    ├── MenuItem "查看全部任务"
    ├── Separator
    ├── MenuItem "休息模式"
    ├── MenuItem "静音"
    ├── Separator
    ├── MenuItem "设置"
    └── MenuItem "退出"
```

### 4.2 窗口模式切换

窗口有两种模式，通过 `expanded` 属性切换：

```slint
export component MainWindow inherits Window {
    property <bool> expanded: false;

    width: expanded ? 500px : 140px;
    height: expanded ? 520px : 180px;
    animate width { duration: 250ms; easing: ease-in-out; }
    animate height { duration: 250ms; easing: ease-in-out; }

    // expanded 时禁止穿透，pet mode 时启用穿透
    // (穿透逻辑在 Rust 侧通过 hit_test 模块控制)
}
```

**注意**: `datetime-local` 在 Slint 中没有原生控件。替代方案：
- 使用 `TextInput` + `SpinBox` 组合手动构建日期时间选择器
- 或者提供一个 TextInput 接受 `YYYY-MM-DD HH:MM` 格式的手动输入
- 推荐方案：4 个 SpinBox（月/日/时/分）+ TextInput（标题），简单可靠

### 4.3 猫咪图片状态切换

```slint
component CatCharacter {
    in property <string> cat_state: "idle";
    in property <bool> sleeping: false;
    in property <bool> happy: false;

    Image {
        source: root.cat_state == "idle" ? @image-url("assets/cat_idle.png")
              : root.cat_state == "remind" ? @image-url("assets/cat_remind.png")
              : root.cat_state == "warning" ? @image-url("assets/cat_warning.png")
              : root.cat_state == "sad" ? @image-url("assets/cat_sad.png")
              : root.cat_state == "recover" ? @image-url("assets/cat_recover.png")
              : root.cat_state == "happy" ? @image-url("assets/cat_happy.png")
              : @image-url("assets/cat_sleeping.png");
        width: 120px;
        height: 160px;
        animate opacity { duration: 300ms; }
    }
}
```

### 4.4 猫咪动画效果

在 Slint 中，每个状态对应的动画通过 `animate` 属性实现：

| 状态 | 动画效果 | Slint 实现 |
|---|---|---|
| idle | 轻微上下浮动 | `animate transform { ... }` translate-y 循环 |
| remind | 抬头 + 身体微倾 | translate-y(-4px) + rotate(-3deg) |
| warning | 水平抖动 | translate-x 震荡, 快节奏 |
| sad | 下沉 + 灰暗 | translate-y(+4px), opacity 降低 |
| recover | 翻转回正 | rotate-y 180→0, easing: ease-out |
| happy | 弹跳 | translate-y(-12px) 三次, ease-out |
| sleeping | 缓慢呼吸缩放 | scale-y 1.0↔0.97, 慢节奏 |

动画通过 Slint 内置的 `animate` 声明，**无需额外代码**。

### 4.5 Property 清单 (暴露给 Rust 的接口)

```slint
// ── 猫咪状态 ──
in-out property <string> cat_state: "idle";
in-out property <string> bubble_text: "";
in-out property <bool> sleeping: false;
in-out property <bool> happy: false;

// ── 任务弹窗 ──
in-out property <bool> popup_open: false;
in-out property <[TaskItem]> popup_tasks: [];

// ── 模态框开关 ──
in-out property <bool> task_modal_open: false;
in-out property <bool> add_modal_open: false;
in-out property <bool> settings_modal_open: false;

// ── 任务列表 ──
in-out property <[TaskItem]> task_list: [];
in-out property <string> task_filter: "pending";

// ── 添加任务表单 ──
in-out property <string> add_title: "";
in-out property <string> add_date: "";   // YYYY-MM-DD
in-out property <int> add_hour: 12;
in-out property <int> add_minute: 0;
in-out property <string> add_desc: "";
in-out property <int> add_remind: 15;

// ── 设置 ──
in-out property <bool> set_autostart: true;
in-out property <bool> set_edgesnap: true;
in-out property <int> set_opacity: 100;      // 30-100
in-out property <bool> set_hourly: true;
in-out property <int> set_h_start: 7;
in-out property <int> set_h_end: 22;
in-out property <bool> set_voice: true;
in-out property <int> set_volume: 80;

// ── 右键菜单 ──
in-out property <bool> context_menu_open: false;
in-out property <int> menu_x: 0;
in-out property <int> menu_y: 0;

// ── Callbacks (Slint → Rust) ──
callback add_task(string, string, int, int, string, int);
callback complete_task(int, string);
callback delete_task(int);
callback pet_pet();
callback update_setting(string, string);
callback hour_check();
callback menu_action(string);
callback toggle_rest_mode();
```

---

## 5. Rust 模块设计

### 5.1 `main.rs` — 入口

```rust
fn main() {
    // 1. 创建 Slint 窗口
    let ui = MainWindow::new()?;

    // 2. 配置窗口属性 (通过 winit backend)
    //    - transparent: true
    //    - always_on_top: true
    //    - decorations: false
    //    - resizable: false
    //    - skip_taskbar: true (Windows)

    // 3. 初始化数据库
    let db = DbState::new();
    db.init_tables()?;

    // 4. 初始化 TTS 引擎
    let tts = TtsManager::new();

    // 5. 初始化状态机
    let pet = PetStateMachine::new();

    // 6. 设置点击穿透 (平台相关)
    #[cfg(target_os = "windows")]
    hit_test::init(&window);
    // Mac/Linux: TODO

    // 7. 绑定所有 callback (委托给 app.rs)
    app::bind_callbacks(&ui, db, pet, tts);

    // 8. 运行事件循环
    ui.run()?;
}
```

### 5.2 `app.rs` — Callback 绑定与状态协调

```rust
pub fn bind_callbacks(
    ui: &MainWindow,
    db: DbState,
    pet: PetStateMachine,
    mut tts: TtsManager,
) {
    let ui_weak = ui.as_weak();

    // ── 任务 CRUD ──
    ui.on_add_task(move |title, date, hour, minute, desc, remind| {
        let due = format!("{} {:02}:{:02}:00", date, hour, minute);
        let id = db.add_task(&title, &desc, &due, remind as i64)?;
        let info = pet.transition(StateEvent::TaskAdded);

        let ui = ui_weak.upgrade()?;
        ui.set_cat_state(state_to_string(&info.state));
        ui.set_bubble_text(info.message);
        ui.set_add_modal_open(false);

        // TTS
        tts.speak(&format!("收到新任务: {}", title));

        refresh_task_list(&ui, &db, &ui.get_task_filter());
    });

    ui.on_complete_task(move |id, title| {
        db.complete_task(id)?;
        let info = pet.transition(StateEvent::TaskCompleted);

        let ui = ui_weak.upgrade()?;
        ui.set_cat_state(state_to_string(&info.state));
        ui.set_bubble_text(format!("好棒! {} 完成啦!", title));
        tts.speak(&format!("好棒! {} 完成啦!", title));

        refresh_task_list(&ui, &db, &ui.get_task_filter());
    });

    // ... 其他 callback 类似模式

    // ── 定时器 ──
    // 2 秒刷新宠物状态
    let ui_w = ui_weak.clone();
    let pet_w = pet.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if let Some(ui) = ui_w.upgrade() {
                let info = pet_w.get_state();
                ui.set_cat_state(state_to_string(&info.state));
            }
        }
    });

    // 整点提醒 (每分钟检查)
    // ... (类似 v1 逻辑)
}
```

### 5.3 `state_machine.rs` — 完全复用

当前 v1 实现中的 `state_machine.rs` 与 UI 框架零耦合，直接迁入 v2。不需修改。

结构体: `PetState`, `PetStateInfo`, `StateEvent`, `PetStateMachine`
方法: `new()`, `get_state()`, `set_state()`, `transition()`

### 5.4 `db.rs` — 微调复用

当前 `db.rs` 仅依赖 `rusqlite`，与 Tauri 无关。迁移时：
- 移除 `tauri::State` 包装 → 改为普通 `Arc<DbState>` 或直接传引用
- 其余代码不变

### 5.5 `tts.rs` — 重写

```rust
use tts::{Tts, Error};

pub struct TtsManager {
    tts: Option<Tts>,
}

impl TtsManager {
    pub fn new() -> Self {
        Self {
            tts: Tts::default().ok(),
        }
    }

    pub fn speak(&mut self, text: &str) {
        if let Some(ref mut tts) = self.tts {
            let _ = tts.speak(text, false); // false = don't interrupt
        }
    }

    pub fn set_volume(&mut self, vol: f32) { /* ... */ }
    pub fn set_rate(&mut self, rate: f32) { /* ... */ }
}
```

**`tts` crate 平台适配:**
- Windows: COM → SAPI (不依赖 PowerShell!)
- macOS: NSSpeechSynthesizer
- Linux: speech-dispatcher (需安装 `speech-dispatcher` 包)

### 5.6 `audio.rs` — 重写

```rust
use rodio::{OutputStream, Sink, source::SineWave};

pub struct AudioManager;

impl AudioManager {
    pub fn play_beep(freq: f32, duration_ms: u64) {
        std::thread::spawn(|| {
            let (_stream, handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&handle).unwrap();
            let source = SineWave::new(freq)
                .take_duration(Duration::from_millis(duration_ms))
                .amplify(0.5);
            sink.append(source);
            sink.sleep_until_end();
        });
    }

    pub fn play_wav(path: &str) {
        // rodio 直接解码 WAV
    }
}
```

### 5.7 `hit_test.rs` — 平台抽象

```
hit_test 模块职责:
  初始化时: 获取窗口原生句柄, 注册平台回调
  运行时:   根据 expanded 状态切换穿透模式

  ┌──────────────────────────────────────────────────┐
  │              HitTestTrait (公共接口)              │
  │  init(window) | set_pet_rect(x,y,w,h) |          │
  │  set_enabled(bool)                                │
  ├────────────────┬────────────────┬────────────────┤
  │ win/impl.rs    │ mac/impl.rs    │ linux/impl.rs  │
  │ WM_NCHITTEST   │ ignoresMouse   │ X11 input      │
  │ subclassing    │ Events toggle  │ shape ext      │
  └────────────────┴────────────────┴────────────────┘
```

**设计要点:**
- 编译期条件编译 (`#[cfg(target_os = "...")]`)
- 公共 `HitTestState` 结构体 (pet_rect + enabled)
- 窗口 expanded 时调用 `set_enabled(false)` → 全窗可交互
- 窗口 pet mode 时调用 `set_enabled(true)` → 仅猫咪区域可交互

当前 v1 的 Windows 实现可以直接作为 `win/impl.rs`。

---

## 6. 依赖清单 (Cargo.toml)

```toml
[package]
name = "pet-desktop"
version = "0.2.0"
edition = "2021"

[[bin]]
name = "pet-desktop"
path = "src/main.rs"

[build-dependencies]
slint-build = "1.8"

[dependencies]
slint = "1.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
log = "0.4"
env_logger = "0.11"
tts = "0.25"                              # 跨平台 TTS
rodio = "0.17"                            # 跨平台音频

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.61", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Gdi",
] }
raw-window-handle = "0.6"

[target.'cfg(target_os = "macos")'.dependencies]
objc = "0.2"

# 注意: 移除的依赖
# tauri, tauri-build, tokio, env_logger? (保留 env_logger)
```

---

## 7. 迁移策略

### 7.1 可复用模块 (直接迁入)

| 模块 | 条件 | 改动 |
|---|---|---|
| `state_machine.rs` | 100% 复用 | 无 |
| `db.rs` | 95% 复用 | 移除 Tauri State 参数, 改为 `Arc` 或直接引用 |
| `hit_test.rs` (Windows 部分) | 90% 复用 | 适配 Slint/winit 窗口句柄获取方式 |

### 7.2 需重写模块

| 模块 | 原因 |
|---|---|
| `main.rs` | 全新: Slint 窗口创建替代 Tauri builder |
| `app.rs` | 全新: Slint callback 替代 Tauri commands |
| `ui/main.slint` | 全新: 声明式 UI 替代 HTML/CSS/JS |
| `tts.rs` | 重写: `tts` crate 替代 PowerShell |
| `audio.rs` | 重写: `rodio` crate 替代 PowerShell |

### 7.3 可删除文件

迁移完成后, 以下文件可删除:
- `src/index.html`, `src/main.js`, `src/styles.css`, `src/popup.html`
- `src-tauri/` 整个目录
- `CLAUDE.md` (更新)
- `package.json`, `node_modules/`

### 7.4 实施顺序

```
Phase 1: 骨架搭建
  1. Cargo.toml (新依赖)
  2. build.rs
  3. main.rs (窗口创建)

Phase 2: 核心模块迁入
  4. state_machine.rs (直接复制)
  5. db.rs (微调复制)
  6. tts.rs (重写)
  7. audio.rs (重写)

Phase 3: UI 构建
  8. ui/main.slint (猫咪 + 弹窗 + 设置)
  9. app.rs (callback 绑定)
  10. hit_test.rs (Windows 适配)

Phase 4: 集成 & 调试
  11. 端到端验证
  12. 清理旧文件
```

---

## 8. 风险与注意事项

### 8.1 Slint 的限制

| 限制 | 影响 | 应对 |
|---|---|---|
| 无原生 datetime picker | 添加任务表单 | 用 SpinBox × 4 (月/日/时/分) |
| 无富文本输入 | 备注字段 | TextArea 原生支持, 足够 |
| 动画不如 CSS 丰富 | 猫咪效果 | 图片本身已含表情, 动画只需 transform |
| Slint 1.x API 可能变化 | 升级风险 | 锁定 minor version |
| Linux TTS 需额外安装 | 部署 | 文档说明需 `apt install speech-dispatcher` |

### 8.2 点击穿透跨平台

这是整个项目中最复杂的部分。当前优先级：
- **Windows**: 完美支持 (WM_NCHITTEST 已有实现)
- **macOS**: 可实现 (NSTrackingArea)
- **Linux**: 部分支持 (取决于 Wayland/X11 和合成器)

macOS 和 Linux 穿透可在 Windows 版本稳定后再补。

### 8.3 构建要求

- Windows: MSVC Build Tools 或 MinGW
- macOS: Xcode Command Line Tools
- Linux: `build-essential`, `libspeechd-dev`, `librust-alsa-dev`
- 所有平台: `cargo` + Rust 1.75+

---

## 9. 附录: 前端 v1 → Slint v2 映射

| v1 概念 | v2 等效 |
|---|---|
| `document.querySelector('#cat')` | `CatCharacter { cat_state: ... }` |
| `cat.classList.add('state-happy')` | `ui.set_cat_state("happy")` |
| `invoke('add_task', { ... })` | `callback add_task(...)` → `app.rs` |
| `setInterval(pollPetState, 2000)` | `std::thread::spawn` + `ui.set_cat_state(...)` |
| `DOM.bubbleText.textContent = text` | `ui.set_bubble_text(text)` |
| `DOM.taskModal.classList.add('hidden')` | `ui.set_task_modal_open(false)` |
| CSS `@keyframes idle-bob` | Slint `animate transform { ... }` |
| `window.__TAURI__.core.invoke('set_hit_enabled', ...)` | `hit_test::set_enabled(true/false)` |
| PowerShell `System.Speech` | `tts` crate via platform TTS API |
| PowerShell `MediaPlayer` | `rodio` crate |
