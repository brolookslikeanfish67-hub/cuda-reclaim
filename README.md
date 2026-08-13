#  cuda-reclaim

`cuda-reclaim` is an ultra-fast, **zero-dependency** Rust command-line utility designed to instantly discover, map, and cleanly purge orphaned PyTorch, vLLM, and CUDA zombie processes holding GPU VRAM hostage after a crashed notebook, training job, or runtime container script.

No more hunting down ghost processes with `ps aux`, manually matching PIDs, or resorting to a full machine reboot just to clear your GPU memory.

---

##  Key Features

* **Zero Dependencies:** Written entirely in raw, optimized Rust using core system calls—no external runtime libraries required.
* **Robust Procfs Interrogation:** Safely extracts Parent PIDs (PPIDs) by anchoring behind erratic process names containing whitespace (e.g., `(python3 script)`).
* **Docker & Container Awareness:** Traces kernel `/proc/{pid}/cgroup` roots to isolate whether a zombie process lives on your native host machine or trapped inside an unmapped Docker/Kubernetes container footprint.
* **Instant Reclamation:** Leverages zero-cost memory mapping to evaluate your active GPU compute landscape and securely sweep ghost PIDs instantly.

---

##  Installation & Building

Since this utility is dependency-free, you only need `cargo` installed alongside your standard NVIDIA Linux driver environment.

### 1. Clone the Repository
```bash
git clone https://github.com
cd cuda-reclaim
```

### 2. Build for Production
```bash
cargo build --release
```
The compiled, self-contained binary will be ready at `./target/release/cuda-reclaim`.

---

##  Usage

Run the utility with administrative or root privileges to ensure it can read kernel system maps across isolated container landscapes and issue clean termination commands:

```bash
sudo ./target/release/cuda-reclaim
```

### Expected Output Structure

```text
Scanning GPU memory space for orphaned AI processes...

  CRITICAL: Found 2 zombie process(es) trapping 14.20 GB of VRAM:
   [PID 14209] [Docker: a1b2c3d4e5f6] python3 training_loop.py - locking 8192 MB
   [PID 28911] [Host]                 vllm_server            - locking 6016 MB

 Initiating absolute purge sequence...
  ✔ Terminated PID 14209
  ✔ Terminated PID 28911

✔ Reclaimed 14.20 GB of GPU VRAM successfully!
```

---

##  How It Works Under the Hood

1. **`nvidia-smi` Pipeline:** Interrogates active hardware state allocations bypassing slow layout parsers by requesting machine-readable CSV fields directly (`pid,process_name,used_memory`).
2. **Deterministic State Audit:** Validates process lineage by traversing raw `/proc/[pid]/stat` targets to evaluate whether the original parent controller session has unexpectedly detached.
3. **Targeted Disruption:** Executes clean, unblockable immediate process interventions across isolated application boundaries to liberate trapped system pipelines.

---

##  License

Distributed under the **MIT License**. See `LICENSE` for more information.
