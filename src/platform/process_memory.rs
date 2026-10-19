// platform/process_memory.rs — Working set atual e pico do processo (BF-002).

/// Memória do processo em bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessMemory {
    pub working_set: u64,
    pub peak_working_set: u64,
    pub private_bytes: u64,
}

#[cfg(target_os = "windows")]
pub fn current() -> Option<ProcessMemory> {
    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, cb: u32) -> i32;
    }

    let mut counters = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        ..Default::default()
    };
    // SAFETY: pseudo-handle do próprio processo e struct com o layout de
    // PROCESS_MEMORY_COUNTERS.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    (ok != 0).then(|| ProcessMemory {
        working_set: counters.working_set_size as u64,
        peak_working_set: counters.peak_working_set_size as u64,
        private_bytes: counters.pagefile_usage as u64,
    })
}

#[cfg(not(target_os = "windows"))]
pub fn current() -> Option<ProcessMemory> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "windows")]
    fn reports_a_plausible_working_set() {
        let memory = super::current().expect("memória do processo");
        assert!(memory.working_set > 1024 * 1024);
        assert!(memory.peak_working_set >= memory.working_set);
    }
}
