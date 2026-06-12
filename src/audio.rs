// ══════════════════════════════════════════════════════════════
// 桌面小宠物 — audio.rs
// 跨平台音频播放 via `rodio` crate
// ══════════════════════════════════════════════════════════════

use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use std::time::Duration;

pub struct AudioManager;

impl AudioManager {
    /// Play a simple beep at the given frequency (Hz) for duration (ms).
    /// Runs on a background thread, fire-and-forget.
    pub fn play_beep(freq: f32, duration_ms: u64) {
        std::thread::spawn(move || {
            if let Ok((_stream, handle)) = OutputStream::try_default() {
                if let Ok(sink) = Sink::try_new(&handle) {
                    let source = SineWave::new(freq)
                        .take_duration(Duration::from_millis(duration_ms))
                        .amplify(0.3);
                    sink.append(source);
                    sink.sleep_until_end();
                }
            }
        });
    }

    /// Play a WAV file from the given path. Runs on a background thread.
    pub fn play_wav(path: &str) {
        let path = path.to_string();
        std::thread::spawn(move || {
            if let Ok((_stream, handle)) = OutputStream::try_default() {
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(source) = rodio::Decoder::new(std::io::BufReader::new(file)) {
                        if let Ok(sink) = Sink::try_new(&handle) {
                            sink.append(source);
                            sink.sleep_until_end();
                        }
                    }
                }
            }
        });
    }
}
