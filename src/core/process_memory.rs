#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProcessRamUsage {
    pub working_set_bytes: u64,
    pub total_physical_bytes: u64,
}

impl ProcessRamUsage {
    pub(crate) fn usage_percent(self) -> u8 {
        if self.total_physical_bytes == 0 {
            return 0;
        }

        let used = self.working_set_bytes as u128;
        let total = self.total_physical_bytes as u128;
        ((used.saturating_mul(100).saturating_add(total / 2)) / total).min(100) as u8
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn query_process_ram_usage() -> Option<ProcessRamUsage> {
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::GetCurrentProcess;

    let mut memory_status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut memory_status) }.ok()?;

    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    let counters_ptr =
        (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>();
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            counters_ptr,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        )
    }
    .ok()?;

    Some(ProcessRamUsage {
        working_set_bytes: counters.WorkingSetSize as u64,
        total_physical_bytes: memory_status.ullTotalPhys,
    })
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn query_process_ram_usage() -> Option<ProcessRamUsage> {
    None
}

#[cfg(test)]
mod tests {
    use super::ProcessRamUsage;

    #[test]
    fn usage_percent_rounds_to_nearest_integer() {
        let usage = ProcessRamUsage {
            working_set_bytes: 3,
            total_physical_bytes: 8,
        };

        assert_eq!(usage.usage_percent(), 38);
    }

    #[test]
    fn usage_percent_clamps_to_hundred() {
        let usage = ProcessRamUsage {
            working_set_bytes: 999,
            total_physical_bytes: 100,
        };

        assert_eq!(usage.usage_percent(), 100);
    }
}
