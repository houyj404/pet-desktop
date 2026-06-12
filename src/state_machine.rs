use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ── Pet states ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PetState {
    Idle,
    Remind,
    Warning,
    Sad,
    Recover,
    Happy,
    Sleeping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetStateInfo {
    pub state: PetState,
    pub sadness_level: u8,
    pub message: String,
}

// ── Events that drive transitions ────────────────────────────

#[derive(Debug, Clone)]
pub enum StateEvent {
    TaskAdded,
    TaskNearDue,      // <= 24h && > 1h
    TaskUrgent,       // <= 1h
    TaskExpired,      // past due
    TaskCompleted,
    PetPet,           // long-press / 摸头
    RestModeOn,
    RestModeOff,
    HourReached,
    IdleTimeout,      // user inactive
    UserReturned,
}

// ── Thread-safe state machine ────────────────────────────────

pub struct PetStateMachine {
    inner: Mutex<PetStateInfo>,
}

impl PetStateMachine {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PetStateInfo {
                state: PetState::Idle,
                sadness_level: 0,
                message: String::new(),
            }),
        }
    }

    pub fn get_state(&self) -> PetStateInfo {
        self.inner.lock().unwrap().clone()
    }

    pub fn set_state(&self, state: PetState, sadness: u8, message: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.state = state;
        inner.sadness_level = sadness;
        inner.message = message.to_string();
    }

    pub fn transition(&self, event: StateEvent) -> PetStateInfo {
        let mut inner = self.inner.lock().unwrap();
        use PetState::*;
        use StateEvent::*;

        let (new_state, msg) = match (&inner.state, event) {
            // ── IDLE transitions ──────────────────────────────
            (Idle, TaskAdded) => (Idle, "收到一个新任务!".into()),
            (Idle, TaskNearDue) => (Remind, "有任务快到期了哦~".into()),
            (Idle, TaskUrgent) => (Warning, "紧急! 任务马上到期!".into()),
            (Idle, TaskExpired) => (Sad, "任务过期了... 好难过".into()),
            (Idle, TaskCompleted) => (Happy, "好棒! 任务完成啦!".into()),
            (Idle, HourReached) => (Idle, format!("现在是整点~")),
            (Idle, IdleTimeout) => (Sleeping, "Zzz...".into()),
            (Idle, RestModeOn) => (Sleeping, "休息一下~".into()),

            // ── REMIND transitions ────────────────────────────
            (Remind, TaskUrgent) => (Warning, "来不及了! 快!".into()),
            (Remind, TaskExpired) => (Sad, "过期了... 背过身去".into()),
            (Remind, TaskCompleted) => (Happy, "太棒了!".into()),
            (Remind, TaskNearDue) => (Remind, "还有任务快到期~".into()),

            // ── WARNING transitions ───────────────────────────
            (Warning, TaskExpired) => {
                inner.sadness_level = (inner.sadness_level + 20).min(100);
                (Sad, "过期了... 好伤心".into())
            }
            (Warning, TaskCompleted) => (Happy, "好险! 赶上了!".into()),

            // ── SAD transitions ───────────────────────────────
            (Sad, PetPet) => (Recover, "嗯? 谢谢摸头...".into()),
            (Sad, TaskCompleted) => (Happy, "终于完成了!".into()),
            (Sad, TaskExpired) => {
                inner.sadness_level = (inner.sadness_level + 10).min(100);
                (Sad, "越来越多任务过期了...".into())
            }

            // ── RECOVER transitions ───────────────────────────
            (Recover, _) => {
                inner.sadness_level = inner.sadness_level.saturating_sub(30);
                (Idle, "恢复精神了!".into())
            }

            // ── HAPPY transitions ─────────────────────────────
            (Happy, _) => (Idle, String::new()),

            // ── SLEEPING transitions ──────────────────────────
            (Sleeping, UserReturned) => (Idle, "你回来啦!".into()),
            (Sleeping, RestModeOff) => (Idle, "醒了!".into()),
            (Sleeping, TaskUrgent) => (Warning, "紧急任务! 醒了!".into()),

            // ── fallback: stay in current state ───────────────
            _ => (inner.state.clone(), inner.message.clone()),
        };

        inner.state = new_state.clone();
        inner.message = msg.clone();

        PetStateInfo {
            state: new_state,
            sadness_level: inner.sadness_level,
            message: msg,
        }
    }
}
