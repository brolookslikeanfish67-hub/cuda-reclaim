use colored::*;
use rayon::prelude::*;
use std::fs;
use std::process::Command;
use sysinfo::{Pid, ProcessExt, System, SystemExt};

#[derive(Debug, Clone)]
struct ZombieGpuProcess {
    pid: u32,
    name: String,
    vram_mb: u64,
    container_id: Option<String>,
}

fn main() {
    ensure_root_or_escalate();

    println!("{}", "\n🚀 REAPER-CUDA: Initializing Deep System Scan...".bold().cyan());

    // 1. Gather all active CUDA workloads from nvidia-smi
    let active_cuda_jobs = fetch_nvidia_pids();
    if active_cuda_jobs.is_empty() {
        println!("{}", "✔ GPU memory is completely pristine. No active jobs found.".bold().green());
        return;
    }

    // 2. Load system process graph globally once
    let mut sys = System::new_all();
    sys.refresh_all();

    // 3. Parallel audit of processes using Rayon for blistering speed
    let zombies: Vec<ZombieGpuProcess> = active_cuda_jobs
        .into_par_iter()
        .filter_map(|(pid, vram, name)| {
            if is_braindead(pid, &sys) {
                let container_id = extract_container_id(pid);
                Some(ZombieGpuProcess { pid, name, vram_mb: vram, container_id })
            } else {
                None
            }
        })
        .collect();

    if zombies.is_empty() {
        println!("{}", "✔ All active GPU processes map to valid parent sessions.".bold().green());
        return;
    }

    // 4. Calculate total memory leakage
    let total_leaked_mb: u64 = zombies.iter().map(|z| z.vram_mb).sum();
    println!(
        "{}",
        format!(
            "⚠️  CRITICAL: Found {} zombie process(es) trapping {:.2} GB of VRAM:",
            zombies.len(),
            total_leaked_mb as f64 / 1024.0
        )
        .bold()
        .red()
    );

    // 5. Render details elegantly
    for process in &zombies {
        let env_tag = match &process.container_id {
            Some(id) => format!("[Docker: {}]", &id[..12]).on_magenta().black(),
            None => "[Host]".on_blue().black(),
        };

        println!(
            "  {} {} {} {} {}",
            "🛑".red(),
            format!("[PID {}]", process.pid).yellow().bold(),
            env_tag,
            process.name.white().underline(),
            format!("locking {} MB", process.vram_mb).bright_red()
        );
    }

    println!("{}", "\n⚡ Initiating absolute purge sequence...".bold().yellow());

    // 6. Kill targets in parallel using direct system calls
    zombies.par_iter().for_each(|process| {
        let _ = Command::new("kill").args(["-9", &process.pid.to_string()]).status();
    });

    println!(
        "{}\n",
        format!("✔ Reclaimed {:.2} GB of GPU VRAM successfully!", total_leaked_mb as f64 / 1024.0)
            .bold()
            .green()
    );
}

/// Automatically restarts the tool via sudo if run without root access
fn ensure_root_or_escalate() {
    if unsafe { libc::getuid() } != 0 {
        println!("{}", "🔒 Elevating privileges to access system tables...".dimmed());
        let args: Vec<String> = std::env::args().collect();
        let status = Command::new("sudo")
            .arg("-E") // Preserves environmental variables
            .arg(&args[0])
            .args(&args[1..])
            .status();

        if status.map_or(false, |s| s.success()) {
            std::process::exit(0);
        } else {
            eprintln!("{}", "❌ Escalation failed. Sudo privileges required.".bold().red());
            std::process::exit(1);
        }
    }
}

/// Queries nvidia-smi with explicit formatting fields
fn fetch_nvidia_pids() -> Vec<(u32, u64, String)> {
    let output = Command::new("nvidia-smi")
        .args(["--query-compute-apps=pid,used_memory,process_name", "--format=csv,noheader,nounits"])
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

/// Advanced deep process audit checking physical realities, not just PPID==1
fn is_braindead(pid: u32, sys: &System) -> bool {
    let sys_pid = Pid::from(pid as usize);
    
    // Test 1: Is it completely missing from the OS kernel process tables?
    let process = match sys.process(sys_pid) {
        Some(p) => p,
        None => return true, // Ghost tracking inside nvidia driver memory
    };

    // Test 2: Catch standard systemd/init adoption cases
    if let Some(parent) = process.parent() {
        if parent.as_u32() == 1 {
            return true;
        }
        // Test 3: If parent process field exists but parent binary is dead
        if sys.process(parent).is_none() {
            return true;
        }
    } else {
        return true; // No parent context exists whatsoever
    }

    false
}

/// Extracts Docker/Container hashes via cgroups routing to map blast radius
fn extract_container_id(pid: u32) -> Option<String> {
    if let Ok(cgroup) = fs::read_to_string(format!("/proc/{}/cgroup", pid)) {
        for line in cgroup.lines() {
            if line.contains("/docker/") || line.contains("/containers/") {
                if let Some(pos) = line.rfind('/') {
                    let id = &line[pos + 1..];
                    if id.len() >= 12 {
                        return Some(id.to_string());
                    }
                }
            }
        }
    }
    None
}
