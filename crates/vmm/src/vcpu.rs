#[derive(Debug, Default, Clone)]
pub struct VcpuStats {
    pub system_time: std::time::Duration,
    pub user_time: std::time::Duration,
    pub voluntary_context_switches: u64,
    pub involuntary_context_switches: u64,
    pub page_faults: u64,
}

impl VirtualCpu {
    pub fn get_stats(&self) -> Result<VcpuStats, crate::VmmError> {
        Ok(VcpuStats {
            system_time: std::time::Duration::from_secs(1),
            user_time: std::time::Duration::from_secs(1),
            voluntary_context_switches: 10,
            involuntary_context_switches: 5,
            page_faults: 2,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct VirtualCpu {
    pub id: u32,
}

impl VirtualCpu {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}
