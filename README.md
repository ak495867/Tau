# Tau

Tau is a Rust command-line toolkit for observing CPU pressure and reducing software-level contention from unusually heavy background workloads. It uses conservative process-priority changes so foreground work can receive more scheduling preference while the operating system retains control of thermal and power safeguards.

Tau does not disable thermal throttling, change firmware settings, overclock hardware, force unsafe voltages, bypass operating-system protections, suspend arbitrary processes, or promise that every device can operate at maximum speed. It is intended to reduce avoidable software contention, not to override physical limits.

## Features

| Capability | Description |
| --- | --- |
| CPU scan | Captures a short CPU and process snapshot and prints the highest observed process consumers. |
| Adaptive watch | Monitors system CPU pressure and identifies heavy processes only when configurable thresholds are reached. |
| Conservative priority control | Raises the niceness value of selected Unix processes without requesting elevated priority. |
| Cooldown protection | Avoids repeatedly changing the same process during a short interval. |
| Automatic restoration | Restores process priorities changed during the current watch session when the session exits. |
| JSON output | Supports machine-readable output for scans and watch events. |
| Rust implementation | Provides a compiled binary with a small runtime footprint and no Python runtime dependency. |

## Requirements

Tau requires Rust and Cargo. The project is configured for the stable Rust 2021 edition. Priority control is implemented on Unix systems through the operating system’s process-priority interface. Other platforms can still compile and observe metrics, but priority control reports that it is unavailable.

## Installation

Clone the repository, build the release binary, and optionally install it into Cargo’s user binary directory:

```bash
git clone https://github.com/ak495867/Tau.git
cd Tau
cargo build --release
cargo install --path .
```

The compiled binary is available at `target/release/tau`.

## Usage

A one-second scan reports current system load and the processes with the highest observed CPU percentages:

```bash
cargo run --release -- scan
```

A JSON scan is useful for scripts and dashboards:

```bash
cargo run --release -- scan --duration 2 --top 15 --json
```

The adaptive watcher changes priority only when total CPU usage is above the system threshold and an individual process is above the process threshold:

```bash
cargo run --release -- watch --duration 300
```

A more conservative policy can be configured explicitly:

```bash
cargo run --release -- watch \
  --interval 3 \
  --system-threshold 90 \
  --process-threshold 50 \
  --background-nice 8 \
  --cooldown 45 \
  --max-actions 2
```

Use JSON output for event streaming:

```bash
cargo run --release -- watch --duration 60 --json
```

Press Ctrl+C to stop watch mode. Tau restores priorities changed during that watch session before exiting.

## Safety model

Tau only requests a lower scheduling priority for candidate processes. Its default target niceness is 10, and accepted values range from 1 through 19. Tau excludes its own process, avoids zombie and dead process states, limits the number of actions per cycle, and restores priorities it changed when the watcher exits.

Operating-system permissions still apply. On Unix systems, changing another user’s process priority may require elevated privileges, and Tau reports permission failures instead of attempting to bypass them. Run Tau with the smallest permissions appropriate for the processes you intend to observe.

CPU percentage measurements are observations over a sampling interval. They are not a guarantee of total energy savings, thermal headroom, benchmark performance, or sustained clock frequency. Results depend on the operating system, scheduler, workload, device cooling, power source, and firmware configuration.

## Project layout

| Path | Purpose |
| --- | --- |
| `src/models.rs` | Metric, policy, action, and watch-event models. |
| `src/platform.rs` | System observation and platform process-priority operations. |
| `src/controller.rs` | Sampling, threshold evaluation, cooldown handling, and restoration. |
| `src/main.rs` | Command-line parsing and human-readable or JSON output. |
| `tests/` | Rust integration tests for policy, candidate selection, and CLI behavior. |
| `LICENSE` | MIT license file. |

## Development

Run the test suite with:

```bash
cargo test
```

Run the formatter and linter with:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

The Rust source files intentionally contain no comments or doc comments. Keep future changes consistent with that repository convention. User-facing explanations belong in Markdown files rather than inside Rust modules.

## Limitations

Tau is a user-space scheduling assistant, not a kernel scheduler, cgroup manager, power governor, thermal controller, or hardware driver. It cannot guarantee that a device will avoid thermal throttling, and it cannot make a CPU deliver more physical performance than its cooling and power envelope permits. For strict CPU quotas, container isolation, or service-level resource governance, use the native controls provided by the target operating system or orchestration platform.

## License

Tau is released under the MIT License.
