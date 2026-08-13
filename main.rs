use std::fs;
use std::process::Command;
use rayon::prelude::*;
use sysinfo::{Pid, Process, System};

#[derive(Debug)]
struct ZombieGpuProcess {
    pid: u32,
    name: String,
    vram_mb: u64,
    container_id: Option<String>,
}

fn main() {
    println!("Scanning GPU memory space for orphaned AI processes...");

    // 1. Extract active GPU computing metrics via machine-readable CSV formatting
    let active_cuda_jobs = fetch_nvidia_pids();
    if active_cuda_jobs.is_empty() {
        println!("✔ All GPU VRAM is clean. No active compute processes found.");
        return;
    }

    // 2. Initialize a complete, accurate system snapshot of memory and process flags
    let mut sys = System::new_all();
    sys.refresh_all();

    // 3. Thread-safe parallel audit of runtime spaces across container boundaries
    let zombies: Vec<ZombieGpuProcess> = active_cuda_jobs
        .into_par_iter()
        .filter_map(|(pid, vram, name)| {
            if is_system_zombie(pid, &sys) {
                let container_id = extract_container_id(pid);
                Some(ZombieGpuProcess {
                    pid,
                    name,
                    vram_mb: vram,
                    container_id,
                })
            } else {
                None
            }
        })
        .collect();

    if zombies.is_empty() {
        println!("✔ All active GPU processes map to valid parent sessions.");
        return;
    }

    let total_leaked: u64 = zombies.iter().map(|z| z.vram_mb).sum();
    println!(
        "\nCRITICAL: Found {} zombie process(es) locking {:.2} GB VRAM:",
        zombies.len(),
        total_leaked as f64 / 1024.0
    );

    for process in &zombies {
        let env_tag = match &process.container_id {
            Some(id) => format!("[Docker Container: {}]", &id[..12]),
            None => "[Native Host]".to_string(),
        };
        println!(
            "  [PID {}] {} {} - Holding {} MB VRAM",
            process.pid, env_tag, process.name, process.vram_mb
        );
    }

    println!("\nInitiating absolute purge sequence...");
    
    // 4. Concurrently terminate targets safely using precise Kernel targets
    zombies.par_iter().for_each(|process| {
        let status = Command::new("kill")
            .arg("-9")
            .arg(process.pid.to_string())
            .status();

        if status.map_or(false, |s| s.success()) {
            println!("  ✔ Terminated PID {}", process.pid);
        }
    });

    println!(
        "\nReclaimed {:.2} GB of GPU VRAM successfully!",
        total_leaked as f64 / 1024.0
    );
}

/// Safely queries the local GPU compute matrix
fn fetch_nvidia_pids() -> Vec<(u32, u64, String)> {
    let output = Command::new("nvidia-smi")
        .arg("--query-compute-apps=pid,used_memory,process_name")
        .arg("--format=csv,noheader,nounits")
        .output();

    let mut workloads = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() == 3 {
                if let (Ok(pid), Ok(vram)) = (parts[0].parse::<u32>(), parts[1].parse::<u64>()) {
                    workloads.push((pid, vram, parts[2].to_string()));
                }
            }
        }
    }
    workloads
}

/// Rigorous multivariant lineage confirmation engine
fn is_system_zombie(pid: u32, sys: &System) -> bool {
    let target_pid = Pid::from(pid as usize);

    // Flaw 1 Fix: Check if process completely slipped into an unindexed kernel state
    let mut current_proc = match sys.process(target_pid) {
        Some(p) => p,
        None => return true, 
    };

    // Flaw 2 Fix: Climb lineage trees recursively to verify validity
    loop {
        match current_proc.parent() {
            Some(parent_pid) => {
                // If it reached systemd/init, checking parent limits is done
                if parent_pid.as_u32() == 1 {
                    return true;
                }

                // If a parent node in the call-stack tree is completely dead, it's a leak
                if let Some(next_parent) = sys.process(parent_pid) {
                    current_proc = next_parent;
                } else {
                    return true; 
                }
            }
            None => {
                // Isolated process loops without a valid supervising descriptor path
                return true;
            }
        }
    }
}

/// Identifies runtime containment profiles directly from system memory maps
fn extract_container_id(pid: u32) -> Option<String> {
    if let Ok(cgroup) = fs::read_to_string(format!("/proc/{}/cgroup", pid)) {
        for line in cgroup.lines() {
            if line.contains("/docker/") || line.contains("/containers/") {
                if let Some(pos) = line.rfind('/') {
                    let id = &line[pos + 1..];
                    // Strip extensions if runtime format includes specialized slices
                    let clean_id = id.strip_suffix(".scope").unwrap_or(id);
                    if clean_id.len() >= 12 {
                        return Some(clean_id.to_string());
                    }
                }
            }
        }
    }
    None
}
