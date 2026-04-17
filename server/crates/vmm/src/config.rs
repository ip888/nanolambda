//! VM configuration types

use crate::error::{VmmError, VmmResult};

/// Configuration for a microVM instance
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Memory size in megabytes
    pub memory_mb: u64,
    /// Number of virtual CPUs
    pub vcpus: u8,
    /// Enable kernel-mode interrupt handling (KVM in-kernel APIC)
    pub use_kernel_irqchip: bool,
    /// Path to the kernel image (vmlinux or bzImage)
    pub kernel_path: Option<String>,
    /// Kernel command line parameters
    pub kernel_cmdline: String,
    /// Path to the root filesystem image
    pub rootfs_path: Option<String>,
    /// Enable vsock for host-guest communication
    pub vsock_cid: Option<u64>,
    /// Enable network device
    pub enable_network: bool,
    /// Memory ballooning support
    pub enable_balloon: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            memory_mb: 128,
            vcpus: 1,
            use_kernel_irqchip: true,
            kernel_path: None,
            kernel_cmdline: "console=ttyS0 reboot=k panic=1 pci=off".to_string(),
            rootfs_path: None,
            vsock_cid: None,
            enable_network: false,
            enable_balloon: false,
        }
    }
}

impl VmConfig {
    /// Validate the configuration
    pub fn validate(&self) -> VmmResult<()> {
        if self.memory_mb < 16 {
            return Err(VmmError::InvalidConfig(
                "Minimum memory is 16 MB".to_string(),
            ));
        }
        if self.memory_mb > 32 * 1024 {
            return Err(VmmError::InvalidConfig(
                "Maximum memory is 32 GB".to_string(),
            ));
        }
        if self.vcpus == 0 {
            return Err(VmmError::InvalidConfig(
                "At least 1 vCPU required".to_string(),
            ));
        }
        if self.vcpus > 32 {
            return Err(VmmError::InvalidConfig(
                "Maximum 32 vCPUs supported".to_string(),
            ));
        }
        Ok(())
    }

    /// Create a builder for VmConfig
    pub fn builder() -> VmConfigBuilder {
        VmConfigBuilder::default()
    }
}

/// Builder pattern for VmConfig
/// 
/// Provides a fluent API for constructing VM configurations with validation.
/// 
/// # Example
/// 
/// ```ignore
/// let config = VmConfigBuilder::default()
///     .memory_mb(256)
///     .vcpus(2)
///     .kernel_path("/path/to/vmlinux")
///     .build()?;
/// ```
#[derive(Default)]
pub struct VmConfigBuilder {
    config: VmConfig,
}

impl VmConfigBuilder {
    /// Set the memory size in megabytes.
    pub fn memory_mb(mut self, mb: u64) -> Self {
        self.config.memory_mb = mb;
        self
    }

    /// Set the number of virtual CPUs.
    pub fn vcpus(mut self, count: u8) -> Self {
        self.config.vcpus = count;
        self
    }

    /// Set the path to the kernel image.
    pub fn kernel_path(mut self, path: impl Into<String>) -> Self {
        self.config.kernel_path = Some(path.into());
        self
    }

    /// Set the kernel command line arguments.
    pub fn kernel_cmdline(mut self, cmdline: impl Into<String>) -> Self {
        self.config.kernel_cmdline = cmdline.into();
        self
    }

    /// Set the path to the root filesystem.
    pub fn rootfs_path(mut self, path: impl Into<String>) -> Self {
        self.config.rootfs_path = Some(path.into());
        self
    }

    /// Set the vsock CID (Context ID) for VM communication.
    pub fn vsock_cid(mut self, cid: u64) -> Self {
        self.config.vsock_cid = Some(cid);
        self
    }

    /// Enable or disable network support.
    pub fn enable_network(mut self, enable: bool) -> Self {
        self.config.enable_network = enable;
        self
    }

    /// Enable or disable memory balloon device.
    pub fn enable_balloon(mut self, enable: bool) -> Self {
        self.config.enable_balloon = enable;
        self
    }

    /// Build and validate the configuration.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the configuration is invalid.
    pub fn build(self) -> VmmResult<VmConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VmConfig::default();
        assert_eq!(config.memory_mb, 128);
        assert_eq!(config.vcpus, 1);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = VmConfig::builder()
            .memory_mb(256)
            .vcpus(2)
            .kernel_path("/boot/vmlinux")
            .build()
            .expect("Config should be valid");

        assert_eq!(config.memory_mb, 256);
        assert_eq!(config.vcpus, 2);
        assert_eq!(config.kernel_path, Some("/boot/vmlinux".to_string()));
    }

    #[test]
    fn test_invalid_memory() {
        let result = VmConfig::builder().memory_mb(8).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_vcpus() {
        let result = VmConfig::builder().vcpus(0).build();
        assert!(result.is_err());
    }
}
