pub mod audio;
pub mod commands;
pub mod db;
pub mod state_machine;
pub mod tts;

use db::DbState;
use state_machine::PetStateMachine;
use tauri::Manager; // needed for app.state()

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
        ])
        .setup(|app| {
            let db = app.state::<DbState>();
            db.init_tables().expect("Failed to init database");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
