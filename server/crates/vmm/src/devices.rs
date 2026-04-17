//! Device management (placeholder for virtio devices)
//!
//! # Planned Devices
//!
//! 1. **virtio-net**: Network device for function networking
//! 2. **virtio-block**: Block device for rootfs
//! 3. **virtio-vsock**: Host-guest communication channel
//! 4. **virtio-balloon**: Memory ballooning for dynamic sizing

use crate::error::{VmmError, VmmResult};

/// Device manager for a VM
pub struct DeviceManager {
    devices: Vec<Box<dyn VirtioDevice>>,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Add a device
    pub fn add_device(&mut self, device: Box<dyn VirtioDevice>) -> VmmResult<()> {
        self.devices.push(device);
        Ok(())
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for virtio devices
pub trait VirtioDevice: Send + Sync {
    /// Get the device type
    fn device_type(&self) -> u32;

    /// Get the device name
    fn name(&self) -> &str;

    /// Activate the device
    fn activate(&mut self) -> VmmResult<()>;

    /// Process a virtqueue notification
    fn process_queue(&mut self, queue_index: u16) -> VmmResult<()>;

    /// Handle device reset
    fn reset(&mut self) -> VmmResult<()>;
}

/// Virtio device types (from virtio spec)
pub mod device_types {
    pub const VIRTIO_NET: u32 = 1;
    pub const VIRTIO_BLOCK: u32 = 2;
    pub const VIRTIO_CONSOLE: u32 = 3;
    pub const VIRTIO_BALLOON: u32 = 5;
    pub const VIRTIO_VSOCK: u32 = 19;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_manager() {
        let manager = DeviceManager::new();
        assert_eq!(manager.device_count(), 0);
    }
}
