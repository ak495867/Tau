use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::models::{ActionResult, Policy, ProcessSnapshot, SystemSnapshot, WatchEvent};
use crate::platform::{lower_priority, restore_priority, Monitor};

pub struct Controller {
    monitor: Monitor,
    policy: Policy,
    current_pid: u32,
    last_action_at: HashMap<u32, Instant>,
    original_nice: HashMap<u32, i32>,
}

impl Controller {
    pub fn new(policy: Policy) -> Result<Self, String> {
        policy.validate()?;
        Ok(Self {
            monitor: Monitor::new(),
            policy,
            current_pid: std::process::id(),
            last_action_at: HashMap::new(),
            original_nice: HashMap::new(),
        })
    }

    pub fn scan(&mut self, duration: Duration) -> (SystemSnapshot, Vec<ProcessSnapshot>) {
        thread::sleep(duration);
        self.monitor.refresh()
    }

    pub fn candidates(
        &self,
        system: &SystemSnapshot,
        processes: &[ProcessSnapshot],
    ) -> Vec<ProcessSnapshot> {
        if system.cpu_percent < self.policy.system_threshold {
            return Vec::new();
        }
        let mut selected: Vec<ProcessSnapshot> = processes
            .iter()
            .filter(|process| {
                process.pid != self.current_pid
                    && process.cpu_percent >= self.policy.process_threshold
                    && process.status != "Zombie"
                    && process.status != "Dead"
            })
            .cloned()
            .collect();
        selected.sort_by(|left, right| {
            right
                .cpu_percent
                .partial_cmp(&left.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        selected.truncate(self.policy.max_actions_per_cycle);
        selected
    }

    pub fn apply(
        &mut self,
        system: &SystemSnapshot,
        processes: &[ProcessSnapshot],
    ) -> Vec<ActionResult> {
        let now = Instant::now();
        let mut actions = Vec::new();
        for process in self.candidates(system, processes) {
            if let Some(previous) = self.last_action_at.get(&process.pid) {
                if now.duration_since(*previous).as_secs_f32() < self.policy.cooldown_seconds {
                    continue;
                }
            }
            let Some(current_nice) = process.nice else {
                continue;
            };
            if current_nice >= self.policy.background_nice {
                continue;
            }
            self.original_nice
                .entry(process.pid)
                .or_insert(current_nice);
            let result = lower_priority(process.pid, self.policy.background_nice);
            self.last_action_at.insert(process.pid, now);
            match result {
                Ok((changed, detail, _)) => actions.push(ActionResult {
                    pid: process.pid,
                    name: process.name,
                    changed,
                    detail,
                }),
                Err(detail) => actions.push(ActionResult {
                    pid: process.pid,
                    name: process.name,
                    changed: false,
                    detail,
                }),
            }
        }
        actions
    }

    pub fn watch<F>(&mut self, duration: Option<Duration>, stop: &AtomicBool, mut on_event: F)
    where
        F: FnMut(WatchEvent),
    {
        let started = Instant::now();
        while !stop.load(Ordering::Relaxed)
            && duration.map_or(true, |value| started.elapsed() < value)
        {
            thread::sleep(Duration::from_secs_f32(self.policy.interval_seconds));
            let (system, processes) = self.monitor.refresh();
            let candidates = self.candidates(&system, &processes);
            let actions = self.apply(&system, &processes);
            on_event(WatchEvent {
                system,
                candidates,
                actions,
            });
        }
    }

    pub fn restore(&mut self) -> Vec<ActionResult> {
        let entries: Vec<(u32, i32)> = self
            .original_nice
            .iter()
            .map(|(pid, nice)| (*pid, *nice))
            .collect();
        let mut actions = Vec::new();
        for (pid, nice) in entries {
            let result = restore_priority(pid, nice);
            actions.push(ActionResult {
                pid,
                name: pid.to_string(),
                changed: result.is_ok(),
                detail: result.unwrap_or_else(|error| error),
            });
            self.original_nice.remove(&pid);
            self.last_action_at.remove(&pid);
        }
        actions
    }
}
