// ══════════════════════════════════════════════════════════════
// 桌面小宠物 — tts.rs
// 跨平台 TTS via `tts` crate
//   Windows: SAPI
//   macOS:   NSSpeechSynthesizer
//   Linux:   speech-dispatcher
// ══════════════════════════════════════════════════════════════

use tts::Tts;

pub struct TtsManager {
    tts: Option<Tts>,
    enabled: bool,
}

impl TtsManager {
    pub fn new() -> Self {
        let tts = Tts::default().ok();
        if tts.is_none() {
            log::warn!("TTS engine not available on this platform");
        }
        Self {
            tts,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Speak text (fire-and-forget on the same thread; `tts` crate handles async).
    pub fn speak(&mut self, text: &str) {
        if !self.enabled {
            return;
        }
        if let Some(ref mut tts) = self.tts {
            // `speak(text, interrupt)` — true = stop current speech
            if let Err(e) = tts.speak(text, false) {
                log::error!("TTS speak failed: {}", e);
            }
        }
    }

    pub fn set_volume(&mut self, vol: f32) {
        if let Some(ref mut tts) = self.tts {
            let _ = tts.set_volume(vol);
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        if let Some(ref mut tts) = self.tts {
            let _ = tts.set_rate(rate);
        }
    }
}

// ── Pre-built speech texts ───────────────────────────────────

pub fn hourly_text(hour: u32) -> String {
    match hour {
        5..=8 => "早安，新的一天开始啦!".into(),
        9..=10 => format!("{}点了，加油哦!", hour),
        11..=13 => "中午啦，该吃饭了!".into(),
        14..=17 => format!("{}点了，记得休息一下~", hour),
        18..=20 => format!("{}点了，晚上好!", hour),
        21..=23 => "很晚了，注意休息哦!".into(),
        _ => format!("{}点了", hour),
    }
}

pub fn task_added_text(title: &str) -> String {
    format!("收到新任务: {}，我会提醒你的", title)
}

pub fn task_due_soon_text(title: &str) -> String {
    format!("{} 还有一个小时截止哦", title)
}

pub fn task_completed_text(title: &str) -> String {
    format!("好棒! {} 已经完成啦!", title)
}
