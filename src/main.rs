use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tau_cpu::{Controller, Policy, ProcessSnapshot, SystemSnapshot, WatchEvent};

#[derive(Parser)]
#[command(
    name = "tau",
    about = "Safe CPU-efficiency monitoring and workload prioritization"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Scan {
        #[arg(long, default_value_t = 1.0)]
        duration: f32,
        #[arg(long, default_value_t = 10)]
        top: usize,
        #[arg(long)]
        json: bool,
    },
    Watch {
        #[arg(long)]
        duration: Option<f32>,
        #[arg(long, default_value_t = 2.0)]
        interval: f32,
        #[arg(long, default_value_t = 85.0)]
        system_threshold: f32,
        #[arg(long, default_value_t = 35.0)]
        process_threshold: f32,
        #[arg(long, default_value_t = 10)]
        background_nice: i32,
        #[arg(long, default_value_t = 30.0)]
        cooldown: f32,
        #[arg(long, default_value_t = 4)]
        max_actions: usize,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Commands::Scan {
            duration,
            top,
            json,
        } => scan(duration, top, json),
        Commands::Watch {
            duration,
            interval,
            system_threshold,
            process_threshold,
            background_nice,
            cooldown,
            max_actions,
            json,
        } => watch(
            duration,
            interval,
            system_threshold,
            process_threshold,
            background_nice,
            cooldown,
            max_actions,
            json,
        ),
    }
}

fn scan(duration: f32, top: usize, json: bool) -> Result<(), Box<dyn Error>> {
    if duration <= 0.0 {
        return Err("duration must be positive".into());
    }
    let mut controller = Controller::new(Policy::default())?;
    let (system, mut processes) = controller.scan(Duration::from_secs_f32(duration));
    processes.sort_by(|left, right| {
        right
            .cpu_percent
            .partial_cmp(&left.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    processes.truncate(top);
    if json {
        println!(
            "{}",
            serde_json::json!({"system": system, "processes": processes})
        );
    } else {
        println!(
            "CPU {:.1}% | Memory {:.1}% | Processes {}",
            system.cpu_percent, system.memory_percent, system.process_count
        );
        for process in processes {
            println!(
                "{:>7} {:>7.1}% {:>7.1}% {}",
                process.pid, process.cpu_percent, process.memory_percent, process.name
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn watch(
    duration: Option<f32>,
    interval: f32,
    system_threshold: f32,
    process_threshold: f32,
    background_nice: i32,
    cooldown: f32,
    max_actions: usize,
    json: bool,
) -> Result<(), Box<dyn Error>> {
    if interval <= 0.0 {
        return Err("interval must be positive".into());
    }
    if duration.is_some_and(|value| value <= 0.0) {
        return Err("duration must be positive".into());
    }
    let policy = Policy {
        interval_seconds: interval,
        system_threshold,
        process_threshold,
        background_nice,
        cooldown_seconds: cooldown,
        max_actions_per_cycle: max_actions,
    };
    let mut controller = Controller::new(policy)?;
    let stop = Arc::new(AtomicBool::new(false));
    let handler_stop = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        handler_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    })?;
    controller.watch(
        duration.map(Duration::from_secs_f32),
        stop.as_ref(),
        |event| print_event(&event, json),
    );
    let restored = controller.restore();
    if !json && !restored.is_empty() {
        eprintln!("restored {} process priorities", restored.len());
    }
    Ok(())
}

fn print_event(event: &WatchEvent, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }
    let actions = if event.actions.is_empty() {
        "no changes".to_string()
    } else {
        event
            .actions
            .iter()
            .map(|action| format!("{}[{}]: {}", action.name, action.pid, action.detail))
            .collect::<Vec<String>>()
            .join(", ")
    };
    println!(
        "CPU {:.1}% | candidates {} | {}",
        event.system.cpu_percent,
        event.candidates.len(),
        actions
    );
}

fn _keep_types(_: &ProcessSnapshot, _: &SystemSnapshot) {}
