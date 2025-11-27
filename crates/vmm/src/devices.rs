#[derive(Debug, Default, Clone)]
pub struct NetworkDevice {}

impl NetworkDevice {
    pub fn bytes_received(&self) -> u64 { 1024 }
    pub fn bytes_transmitted(&self) -> u64 { 2048 }
}

#[derive(Debug, Default, Clone)]
pub struct BlockDevice {}

impl BlockDevice {
    pub fn bytes_read(&self) -> u64 { 4096 }
    pub fn bytes_written(&self) -> u64 { 8192 }
}

impl VmDevices {
    pub fn get_network_device(&self) -> Result<NetworkDevice, crate::VmmError> {
        Ok(NetworkDevice {})
    }
    pub fn get_block_device(&self) -> Result<BlockDevice, crate::VmmError> {
        Ok(BlockDevice {})
    }
}


#[derive(Debug, Default, Clone)]
pub struct VmDevices {}

impl VmDevices {
    pub fn new() -> Self {
        Self {}
    }
}
