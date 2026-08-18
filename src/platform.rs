use std::time::{SystemTime, UNIX_EPOCH};

use sysinfo::{CpuExt, PidExt, ProcessExt, System, SystemExt};

use crate::models::{ProcessSnapshot, SystemSnapshot};

pub struct Monitor {
    system: System,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    pub fn refresh(&mut self) -> (SystemSnapshot, Vec<ProcessSnapshot>) {
        self.system.refresh_all();
        let total_memory = self.system.total_memory();
        let memory_percent = if total_memory == 0 {
            0.0
        } else {
            self.system.used_memory() as f32 * 100.0 / total_memory as f32
        };
        let load = self.system.load_average();
        let system_snapshot = SystemSnapshot {
            cpu_percent: self.system.global_cpu_info().cpu_usage(),
            memory_percent,
            load_average: Some([load.one, load.five, load.fifteen]),
            process_count: self.system.processes().len(),
            captured_at: timestamp(),
        };
        let processes = self
            .system
            .processes()
            .values()
            .map(|process| ProcessSnapshot {
                pid: process.pid().as_u32(),
                name: process.name().to_string(),
                cpu_percent: process.cpu_usage(),
                memory_percent: if total_memory == 0 {
                    0.0
                } else {
                    process.memory() as f32 * 100.0 / total_memory as f32
                },
                status: format!("{:?}", process.status()),
                nice: process_nice(process.pid().as_u32()),
            })
            .collect();
        (system_snapshot, processes)
    }
}

#[cfg(unix)]
fn process_nice(pid: u32) -> Option<i32> {
    let path = format!("/proc/{pid}/stat");
    let content = std::fs::read_to_string(path).ok()?;
    let end = content.rfind(") ")?;
    let fields: Vec<&str> = content[end + 2..].split_whitespace().collect();
    fields.get(16)?.parse().ok()
}

#[cfg(not(unix))]
fn process_nice(_pid: u32) -> Option<i32> {
    None
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(unix)]
pub fn lower_priority(pid: u32, target_nice: i32) -> Result<(bool, String, i32), String> {
    if !(1..=19).contains(&target_nice) {
        return Err("target nice must be between 1 and 19".to_string());
    }
    let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid, target_nice) };
    if result != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok((true, format!("priority set to {target_nice}"), target_nice))
}

#[cfg(not(unix))]
pub fn lower_priority(_pid: u32, _target_nice: i32) -> Result<(bool, String, i32), String> {
    Err("priority control is unavailable on this platform".to_string())
}

#[cfg(unix)]
pub fn restore_priority(pid: u32, original_nice: i32) -> Result<String, String> {
    let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid, original_nice) };
    if result != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(format!("priority restored to {original_nice}"))
}

#[cfg(not(unix))]
pub fn restore_priority(_pid: u32, _original_nice: i32) -> Result<String, String> {
    Err("priority control is unavailable on this platform".to_string())
}
