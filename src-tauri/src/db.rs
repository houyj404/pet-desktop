use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ── Data models ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub due_time: String,
    pub remind_minutes: i64,
    pub is_completed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

// ── Database state (thread-safe wrapper) ─────────────────────

pub struct DbState {
    conn: Mutex<Connection>,
}

impl DbState {
    pub fn new() -> Self {
        let conn = Connection::open("pet_data.db").expect("Failed to open database");
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tasks (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                title       TEXT    NOT NULL,
                description TEXT    DEFAULT '',
                due_time    TEXT    NOT NULL,
                remind_minutes INTEGER DEFAULT 15,
                is_completed INTEGER DEFAULT 0,
                created_at  TEXT DEFAULT (datetime('now','localtime')),
                updated_at  TEXT DEFAULT (datetime('now','localtime'))
            );

            CREATE TABLE IF NOT EXISTS pet_state (
                id              INTEGER PRIMARY KEY CHECK (id = 1),
                current_state   TEXT DEFAULT 'IDLE',
                sadness_level   INTEGER DEFAULT 0,
                last_position_x INTEGER DEFAULT 0,
                last_position_y INTEGER DEFAULT 0,
                rest_mode_until TEXT DEFAULT NULL,
                updated_at      TEXT DEFAULT (datetime('now','localtime'))
            );

            CREATE TABLE IF NOT EXISTS settings (
                key        TEXT PRIMARY KEY,
                value      TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now','localtime'))
            );

            INSERT OR IGNORE INTO pet_state (id) VALUES (1);

            INSERT OR IGNORE INTO settings (key, value) VALUES
                ('voice_enabled',     'true'),
                ('voice_mode',        'tts'),
                ('tts_volume',        '80'),
                ('tts_rate',          '0'),
                ('hourly_enabled',    'true'),
                ('hourly_start_hour', '7'),
                ('hourly_end_hour',   '22'),
                ('auto_start',        'true'),
                ('pet_transparency',  '100'),
                ('edge_snap',         'true');
            ",
        )?;
        Ok(())
    }

    // ── Task CRUD ────────────────────────────────────────────

    pub fn get_tasks(&self, filter: &str) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let sql = match filter {
            "completed" => {
                "SELECT id,title,description,due_time,remind_minutes,is_completed,created_at,updated_at
                 FROM tasks WHERE is_completed=1 ORDER BY due_time"
            }
            "pending" => {
                "SELECT id,title,description,due_time,remind_minutes,is_completed,created_at,updated_at
                 FROM tasks WHERE is_completed=0 ORDER BY due_time"
            }
            _ => {
                "SELECT id,title,description,due_time,remind_minutes,is_completed,created_at,updated_at
                 FROM tasks ORDER BY due_time"
            }
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                due_time: row.get(3)?,
                remind_minutes: row.get(4)?,
                is_completed: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>>>()?)
    }

    pub fn add_task(
        &self,
        title: &str,
        description: &str,
        due_time: &str,
        remind_minutes: i64,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, description, due_time, remind_minutes) VALUES (?1,?2,?3,?4)",
            rusqlite::params![title, description, due_time, remind_minutes],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn complete_task(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET is_completed=1, updated_at=datetime('now','localtime') WHERE id=?1",
            [id],
        )?;
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tasks WHERE id=?1", [id])?;
        Ok(())
    }

    // ── Settings ─────────────────────────────────────────────

    pub fn get_settings(&self) -> Result<Vec<Setting>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok(Setting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>>>()?)
    }

    pub fn update_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1,?2,datetime('now','localtime'))",
            [key, value],
        )?;
        Ok(())
    }
}
