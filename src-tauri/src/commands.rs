use crate::db::{DbState, Setting, Task};
use crate::state_machine::{PetStateMachine, PetStateInfo, StateEvent};
use crate::tts;
use crate::hit_test;
use log::info;
use tauri::Manager;

// ── Task commands ────────────────────────────────────────────

#[tauri::command]
pub fn get_tasks(state: tauri::State<DbState>, filter: String) -> Result<Vec<Task>, String> {
    state.get_tasks(&filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_task(
    state: tauri::State<DbState>,
    pet: tauri::State<PetStateMachine>,
    title: String,
    description: String,
    due_time: String,
    remind_minutes: i64,
    voice_enabled: bool,
) -> Result<i64, String> {
    let id = state
        .add_task(&title, &description, &due_time, remind_minutes)
        .map_err(|e| e.to_string())?;

    // Trigger state machine: new task
    let info = pet.transition(StateEvent::TaskAdded);
    info!("Task added: {} -> state {:?}", title, info.state);

    // TTS feedback
    if voice_enabled {
        let tts_mgr = tts::TtsManager::new();
        tts_mgr.speak(&tts::task_added_text(&title));
    }

    Ok(id)
}

#[tauri::command]
pub fn complete_task(
    state: tauri::State<DbState>,
    pet: tauri::State<PetStateMachine>,
    id: i64,
    title: String,
    voice_enabled: bool,
) -> Result<(), String> {
    state.complete_task(id).map_err(|e| e.to_string())?;

    let info = pet.transition(StateEvent::TaskCompleted);
    info!("Task completed: {} -> state {:?}", title, info.state);

    if voice_enabled {
        let tts_mgr = tts::TtsManager::new();
        tts_mgr.speak(&tts::task_completed_text(&title));
    }

    Ok(())
}

#[tauri::command]
pub fn delete_task(state: tauri::State<DbState>, id: i64) -> Result<(), String> {
    state.delete_task(id).map_err(|e| e.to_string())
}

// ── Pet state commands ───────────────────────────────────────

#[tauri::command]
pub fn get_pet_state(pet: tauri::State<PetStateMachine>) -> PetStateInfo {
    pet.get_state()
}

#[tauri::command]
pub fn set_pet_state(
    pet: tauri::State<PetStateMachine>,
    state_name: String,
    sadness_level: u8,
    message: String,
) {
    use crate::state_machine::PetState;
    let s = match state_name.as_str() {
        "IDLE"     => PetState::Idle,
        "REMIND"   => PetState::Remind,
        "WARNING"  => PetState::Warning,
        "SAD"      => PetState::Sad,
        "RECOVER"  => PetState::Recover,
        "HAPPY"    => PetState::Happy,
        "SLEEPING" => PetState::Sleeping,
        _          => PetState::Idle,
    };
    pet.set_state(s, sadness_level, &message);
}

#[tauri::command]
pub fn pet_pet(pet: tauri::State<PetStateMachine>) -> PetStateInfo {
    pet.transition(StateEvent::PetPet)
}

#[tauri::command]
pub fn hour_reached(pet: tauri::State<PetStateMachine>, hour: u32, voice_enabled: bool) -> PetStateInfo {
    let info = pet.transition(StateEvent::HourReached);
    if voice_enabled {
        let tts_mgr = tts::TtsManager::new();
        tts_mgr.speak(&tts::hourly_text(hour));
    }
    info
}

// ── Settings commands ────────────────────────────────────────

#[tauri::command]
pub fn get_settings(state: tauri::State<DbState>) -> Result<Vec<Setting>, String> {
    state.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_setting(state: tauri::State<DbState>, key: String, value: String) -> Result<(), String> {
    state.update_setting(&key, &value).map_err(|e| e.to_string())
}

// ── TTS / Audio commands ─────────────────────────────────────

#[tauri::command]
pub fn speak_text(text: String, volume: u16, rate: i32) {
    let mut tts_mgr = tts::TtsManager::new();
    tts_mgr.set_volume(volume);
    tts_mgr.set_rate(rate);
    tts_mgr.speak(&text);
}

#[tauri::command]
pub fn play_sound(path: String) {
    let audio = crate::audio::AudioManager::new();
    audio.play_file(&path);
}

// ── Hit-test commands ──────────────────────────────────────

#[tauri::command]
pub fn set_hit_rect(x: i32, y: i32, w: i32, h: i32) {
    hit_test::set_pet_rect(x, y, w, h);
}

#[tauri::command]
pub fn set_hit_enabled(on: bool) {
    hit_test::set_enabled(on);
}

 
