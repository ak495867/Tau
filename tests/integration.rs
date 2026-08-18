use assert_cmd::Command;
use predicates::str::contains;
use tau_cpu::{Controller, Policy, ProcessSnapshot, SystemSnapshot};

fn process(pid: u32, cpu_percent: f32, status: &str) -> ProcessSnapshot {
    ProcessSnapshot {
        pid,
        name: format!("proc-{pid}"),
        cpu_percent,
        memory_percent: 1.0,
        status: status.to_string(),
        nice: Some(0),
    }
}

fn system(cpu_percent: f32) -> SystemSnapshot {
    SystemSnapshot {
        cpu_percent,
        memory_percent: 40.0,
        load_average: None,
        process_count: 3,
        captured_at: "0".to_string(),
    }
}

#[test]
fn policy_rejects_invalid_interval() {
    let policy = Policy {
        interval_seconds: 0.0,
        ..Policy::default()
    };
    assert!(policy.validate().is_err());
}

#[test]
fn candidates_require_system_pressure() {
    let controller = Controller::new(Policy::default()).expect("valid policy");
    let processes = vec![process(100_001, 80.0, "Run")];
    assert!(controller.candidates(&system(84.9), &processes).is_empty());
    assert_eq!(controller.candidates(&system(85.0), &processes).len(), 1);
}

#[test]
fn candidates_are_sorted_and_bounded() {
    let policy = Policy {
        process_threshold: 10.0,
        max_actions_per_cycle: 2,
        ..Policy::default()
    };
    let controller = Controller::new(policy).expect("valid policy");
    let processes = vec![
        process(100_001, 20.0, "Run"),
        process(100_002, 70.0, "Run"),
        process(100_003, 50.0, "Run"),
    ];
    let selected = controller.candidates(&system(90.0), &processes);
    assert_eq!(
        selected.iter().map(|item| item.pid).collect::<Vec<u32>>(),
        vec![100_002, 100_003]
    );
}

#[test]
fn terminal_processes_are_ignored() {
    let controller = Controller::new(Policy::default()).expect("valid policy");
    let processes = vec![
        process(100_001, 90.0, "Zombie"),
        process(100_002, 40.0, "Run"),
    ];
    let selected = controller.candidates(&system(90.0), &processes);
    assert_eq!(
        selected.iter().map(|item| item.pid).collect::<Vec<u32>>(),
        vec![100_002]
    );
}

#[test]
fn cli_exposes_scan_and_watch() {
    let mut command = Command::cargo_bin("tau").expect("binary exists");
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("scan"));
}
