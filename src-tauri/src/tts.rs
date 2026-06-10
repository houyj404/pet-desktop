#[cfg(not(target_os = "windows"))]
use log::info;

/// TTS via Windows PowerShell System.Speech -- zero extra crate deps.
pub struct TtsManager {
    enabled: bool,
    volume: u16,
    rate: i32,
}

impl TtsManager {
    pub fn new() -> Self {
        Self {
            enabled: true,
            volume: 80,
            rate: 0,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_volume(&mut self, volume: u16) {
        self.volume = volume.min(100);
    }

    pub fn set_rate(&mut self, rate: i32) {
        self.rate = rate.clamp(-10, 10);
    }

    /// Speak text using PowerShell SAPI on a background thread.
    pub fn speak(&self, text: &str) {
        if !self.enabled {
            return;
        }
        let text = text.to_string();
        let volume = self.volume;
        let rate = self.rate;

        std::thread::spawn(move || {
            speak_via_powershell(&text, volume, rate);
        });
    }
}

#[cfg(target_os = "windows")]
fn speak_via_powershell(text: &str, volume: u16, rate: i32) {
    let escaped = text.replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.Volume={vol}; $s.Rate={rate}; $s.Speak('{txt}')",
        vol = volume,
        rate = rate,
        txt = escaped,
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(not(target_os = "windows"))]
fn speak_via_powershell(text: &str, _volume: u16, _rate: i32) {
    info!("[TTS] would speak: {}", text);
}

/// Generate hour-dependent TTS text.
pub fn hourly_text(hour: u32) -> String {
    match hour {
        5..=8   => "早安，新的一天开始啦!".into(),
        9..=10  => format!("{}点了，加油哦!", hour),
        11..=13 => "中午啦，该吃饭了!".into(),
        14..=17 => format!("{}点了，记得休息一下~", hour),
        18..=20 => format!("{}点了，晚上好!", hour),
        21..=23 => "很晚了，注意休息哦!".into(),
        _       => format!("{}点了", hour),
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
