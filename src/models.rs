use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub status: String,
    pub nice: Option<i32>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SystemSnapshot {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub load_average: Option<[f64; 3]>,
    pub process_count: usize,
    pub captured_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Policy {
    pub interval_seconds: f32,
    pub system_threshold: f32,
    pub process_threshold: f32,
    pub background_nice: i32,
    pub cooldown_seconds: f32,
    pub max_actions_per_cycle: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            interval_seconds: 2.0,
            system_threshold: 85.0,
            process_threshold: 35.0,
            background_nice: 10,
            cooldown_seconds: 30.0,
            max_actions_per_cycle: 4,
        }
    }
}

impl Policy {
    pub fn validate(&self) -> Result<(), String> {
        if self.interval_seconds <= 0.0 {
            return Err("interval must be positive".to_string());
        }
        if !(0.0..=100.0).contains(&self.system_threshold) {
            return Err("system threshold must be between 0 and 100".to_string());
        }
        if !(0.0..=100.0).contains(&self.process_threshold) {
            return Err("process threshold must be between 0 and 100".to_string());
        }
        if !(1..=19).contains(&self.background_nice) {
            return Err("background nice must be between 1 and 19".to_string());
        }
        if self.cooldown_seconds < 0.0 {
            return Err("cooldown cannot be negative".to_string());
        }
        if self.max_actions_per_cycle == 0 {
            return Err("max actions must be positive".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ActionResult {
    pub pid: u32,
    pub name: String,
    pub changed: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct WatchEvent {
    pub system: SystemSnapshot,
    pub candidates: Vec<ProcessSnapshot>,
    pub actions: Vec<ActionResult>,
}
