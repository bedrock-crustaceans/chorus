use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub struct ProcessStats {
    pub resident_bytes: u64,
    pub virtual_bytes: u64,
    /// Only available on Linux, sysinfo cannot enumerate a process' threads elsewhere.
    pub threads: Option<usize>,
}

/// Reads the memory usage and thread count of the running process.
pub fn process_stats() -> Option<ProcessStats> {
    let pid = Pid::from_u32(std::process::id());

    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing().with_memory().with_tasks());

    let process = system.process(pid)?;

    Some(ProcessStats {
        resident_bytes: process.memory(),
        virtual_bytes: process.virtual_memory(),
        threads: process.tasks().map(|tasks| tasks.len()),
    })
}
