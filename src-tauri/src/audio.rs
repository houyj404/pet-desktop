#[cfg(not(target_os = "windows"))]
use log::info;
use std::sync::Mutex;

/// Audio manager -- plays WAV files via PowerShell on Windows.
pub struct AudioManager {
    enabled: Mutex<bool>,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            enabled: Mutex::new(true),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock().unwrap() = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    /// Play a WAV sound file asynchronously via PowerShell.
    pub fn play_file(&self, path: &str) {
        if !self.is_enabled() {
            return;
        }
        let path = path.to_string();
        std::thread::spawn(move || {
            play_wav(&path);
        });
    }

    /// Play a system beep via PowerShell.
    pub fn play_beep(&self, freq: u32, duration_ms: u32) {
        if !self.is_enabled() {
            return;
        }
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            {
                let script = format!(
                    "[console]::Beep({freq},{dur})",
                    freq = freq,
                    dur = duration_ms,
                );
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &script])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
            #[cfg(not(target_os = "windows"))]
            {
                info!("[Audio] beep {}Hz {}ms", freq, duration_ms);
            }
        });
    }
}

#[cfg(target_os = "windows")]
fn play_wav(path: &str) {
    let script = format!(
        "Add-Type -AssemblyName PresentationCore; $m = New-Object System.Windows.Media.MediaPlayer; $m.Open([uri]::new('{}')); $m.Play(); Start-Sleep -Milliseconds 500",
        path.replace('\'', "''"),
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(not(target_os = "windows"))]
fn play_wav(path: &str) {
    info!("[Audio] would play: {}", path);
}
