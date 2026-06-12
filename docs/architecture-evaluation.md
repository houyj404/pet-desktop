项目架构与可选方案评估
=================================

概述
----
本项目采用 Tauri (Rust 后端) + 前端静态页面（HTML/CSS/JS / Slint）与 SQLite 的实现，目标是一个常驻、轻量、动画丰富的桌面“宠物”应用。此文档总结当前实现的优缺点并给出可替代方案与建议。

当前实现优点
-------------
- 资源占用低：Tauri + Rust 相比 Electron 更轻量，适合常驻应用。 
- 本地能力强：Rust 容易集成系统 API（窗口控制、文件、TTS 等）。
- 前端灵活：HTML/CSS 动画能实现丰富表情与行为；Slint 可用于更原生 UI。
- 数据持久：SQLite 足够任务管理场景，事务与查询简单可靠。

当前实现限制
--------------
- 构建门槛：Rust/Tauri 在 Windows 上需要 MSVC，构建对新手不友好。
- 跨平台差异：当前 TTS/audio 实现为 Windows 专用（PowerShell），其他平台需另行实现。
- 开发生态：若团队以 JS 为主，Rust 层迭代可能较慢。

替代方案对比（何时选择）
-----------------------
- Electron (JS/React/Vue/Svelte)
  - 优点：开发速度快、生态丰富、第三方库多。适合团队以 JS 为主或需要快速原型。  
  - 缺点：体积与内存较大，不适合长期常驻的低资源场景。

- Flutter (Dart)
  - 优点：跨平台一致渲染，动画/性能优秀，适合高度自定义视觉效果。  
  - 缺点：打包体积大，桌面支持成熟度次于移动。

- Slint + Rust（原生 UI）
  - 优点：更小的内存占用、无浏览器运行时，使用 Rust 构建完整应用。  
  - 缺点：生态与组件较少，迁移成本存在。

- .NET (WPF / Avalonia)
  - 优点：Windows 原生体验或跨平台（Avalonia），C# 社区成熟。  
  - 缺点：依赖 .NET 技能栈；WPF 限 Windows。

子系统替代与改进建议
--------------------
- TTS/音频：抽象出接口层（strategy pattern），提供 Windows SAPI、macOS AVSpeech、Linux espeak 或云 TTS 的可插拔实现。  
- 存储：保持 SQLite，如需高并发或 KV 语义可考虑 `sled` 或 LMDB，但目前无需更换。  
- 打包：Tauri 保持较小安装包，若迁移 Electron 需接受体积增长。

建议路线（优先级）
-----------------
1. 保持现有栈（Tauri + Rust + 前端 CSS/Slint），因为它已较好满足“轻量+原生能力+动画”的目标。  
2. 抽离并实现跨平台的音频/tts 适配层，先做接口并保留 Windows 实现。  
3. 若目标是进一步降低内存并移除浏览器依赖，考虑把 UI 从 HTML 迁移到 Slint（逐步替换）。  
4. 若团队以 JS 为主或优先快速迭代，可考虑 Electron 作为替代，并评估体积/性能权衡。

后续工作建议
--------------
- 我可以为你：
  - 实现 `tts` 抽象层并提交 PR（含 Windows/macro stub），或
  - 提供 Slint POC（对比内存与启动时间），或
  - 准备迁移到 Electron 的工作清单与代价估算。

文件位置：docs/architecture-evaluation.md
