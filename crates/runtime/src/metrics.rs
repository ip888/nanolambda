use std::fs;
use std::time::{Duration, SystemTime};

/// Process memory and CPU metrics collected from /proc filesystem
#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub pid: u32,
    pub rss_bytes: u64,      // Resident Set Size (RAM used)
    pub vms_bytes: u64,      // Virtual Memory Size
    pub rss_peak_bytes: u64, // Peak RSS (VmHWM)
    pub vms_peak_bytes: u64, // Peak VMS (VmPeak)
    pub cpu_utime: u64,      // User mode CPU time (jiffies)
    pub cpu_stime: u64,      // System mode CPU time (jiffies)
    pub threads: u32,
    pub timestamp: SystemTime,
}

impl ProcessMetrics {
    /// Collect metrics from /proc/{pid}/status and /proc/{pid}/stat
    pub fn from_pid(pid: u32) -> Result<Self, MetricsError> {
        let mut metrics = Self::from_status_file(pid)?;
        
        // Supplement with CPU data from stat file
        if let Ok(stat_data) = Self::parse_stat_file(pid) {
            metrics.cpu_utime = stat_data.0;
            metrics.cpu_stime = stat_data.1;
        }
        
        metrics.timestamp = SystemTime::now();
        Ok(metrics)
    }
    
    /// Parse /proc/{pid}/status for memory information
    fn from_status_file(pid: u32) -> Result<Self, MetricsError> {
        let path = format!("/proc/{}/status", pid);
        let content = fs::read_to_string(&path)
            .map_err(|e| MetricsError::ReadError(path.clone(), e.to_string()))?;
        
        Self::parse_status(&content, pid)
    }
    
    /// Parse content of /proc/{pid}/status file
    fn parse_status(content: &str, pid: u32) -> Result<Self, MetricsError> {
        let mut rss_bytes = 0;
        let mut vms_bytes = 0;
        let mut rss_peak_bytes = 0;
        let mut vms_peak_bytes = 0;
        let mut threads = 1;
        
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("VmSize:") {
                vms_bytes = parse_kb_value(value)?;
            } else if let Some(value) = line.strip_prefix("VmRSS:") {
                rss_bytes = parse_kb_value(value)?;
            } else if let Some(value) = line.strip_prefix("VmPeak:") {
                vms_peak_bytes = parse_kb_value(value)?;
            } else if let Some(value) = line.strip_prefix("VmHWM:") {
                rss_peak_bytes = parse_kb_value(value)?;
            } else if let Some(value) = line.strip_prefix("Threads:") {
                threads = value.trim().parse().unwrap_or(1);
            }
        }
        
        Ok(ProcessMetrics {
            pid,
            rss_bytes,
            vms_bytes,
            rss_peak_bytes,
            vms_peak_bytes,
            cpu_utime: 0,
            cpu_stime: 0,
            threads,
            timestamp: SystemTime::now(),
        })
    }
    
    /// Parse /proc/{pid}/stat for CPU time (returns utime, stime)
    fn parse_stat_file(pid: u32) -> Result<(u64, u64), MetricsError> {
        let path = format!("/proc/{}/stat", pid);
        let content = fs::read_to_string(&path)
            .map_err(|e| MetricsError::ReadError(path.clone(), e.to_string()))?;
        
        // Split by whitespace, handling process names with spaces in parentheses
        let parts: Vec<&str> = content.split_whitespace().collect();
        
        if parts.len() < 15 {
            return Err(MetricsError::ParseError(
                "Not enough fields in /proc/stat".to_string(),
            ));
        }
        
        // Fields 14 and 15 are utime and stime (0-indexed: 13 and 14)
        let utime = parts[13]
            .parse()
            .map_err(|_| MetricsError::ParseError("Invalid utime".to_string()))?;
        let stime = parts[14]
            .parse()
            .map_err(|_| MetricsError::ParseError("Invalid stime".to_string()))?;
        
        Ok((utime, stime))
    }
    
    /// Calculate CPU percentage between two metrics snapshots
    pub fn cpu_percent(&self, previous: &ProcessMetrics) -> f64 {
        // Calculate time delta in seconds
        let time_delta = self
            .timestamp
            .duration_since(previous.timestamp)
            .unwrap_or(Duration::from_secs(1))
            .as_secs_f64();
        
        if time_delta < 0.001 {
            return 0.0; // Too short interval
        }
        
        // Calculate CPU time delta (in jiffies, typically 100 per second)
        let cpu_delta = (self.cpu_utime + self.cpu_stime)
            .saturating_sub(previous.cpu_utime + previous.cpu_stime);
        
        // Convert jiffies to seconds (assuming 100 Hz)
        let cpu_seconds = cpu_delta as f64 / 100.0;
        
        // CPU percentage = (CPU time / wall time) * 100
        (cpu_seconds / time_delta) * 100.0
    }
    
    /// Get RSS in megabytes
    pub fn memory_mb(&self) -> f64 {
        self.rss_bytes as f64 / 1024.0 / 1024.0
    }
    
    /// Get peak RSS in megabytes
    pub fn peak_memory_mb(&self) -> f64 {
        self.rss_peak_bytes as f64 / 1024.0 / 1024.0
    }
    
    /// Get virtual memory size in megabytes
    pub fn vms_mb(&self) -> f64 {
        self.vms_bytes as f64 / 1024.0 / 1024.0
    }
}

/// Parse a value like "12345 kB" to bytes
fn parse_kb_value(s: &str) -> Result<u64, MetricsError> {
    let trimmed = s.trim();
    let kb_str = trimmed
        .strip_suffix(" kB")
        .or_else(|| trimmed.strip_suffix("kB"))
        .unwrap_or(trimmed);
    
    kb_str
        .trim()
        .parse::<u64>()
        .map(|kb| kb * 1024)
        .map_err(|_| MetricsError::ParseError(format!("Invalid kB value: {}", s)))
}

#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Failed to read {0}: {1}")]
    ReadError(String, String),
    
    #[error("Failed to parse metrics: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_status() {
        let content = r#"Name:	python3
Umask:	0022
State:	R (running)
Tgid:	12345
Ngid:	0
Pid:	12345
VmPeak:	   15000 kB
VmSize:	   12345 kB
VmLck:	       0 kB
VmPin:	       0 kB
VmHWM:	    8000 kB
VmRSS:	    6789 kB
VmData:	    4567 kB
VmStk:	     132 kB
Threads:	2
"#;
        
        let metrics = ProcessMetrics::parse_status(content, 12345).unwrap();
        
        assert_eq!(metrics.pid, 12345);
        assert_eq!(metrics.vms_bytes, 12345 * 1024);
        assert_eq!(metrics.rss_bytes, 6789 * 1024);
        assert_eq!(metrics.vms_peak_bytes, 15000 * 1024);
        assert_eq!(metrics.rss_peak_bytes, 8000 * 1024);
        assert_eq!(metrics.threads, 2);
    }
    
    #[test]
    fn test_parse_kb_value() {
        assert_eq!(parse_kb_value("12345 kB").unwrap(), 12345 * 1024);
        assert_eq!(parse_kb_value("  6789 kB  ").unwrap(), 6789 * 1024);
        assert_eq!(parse_kb_value("100kB").unwrap(), 100 * 1024);
    }
    
    #[test]
    fn test_cpu_percent_calculation() {
        let prev = ProcessMetrics {
            pid: 123,
            rss_bytes: 0,
            vms_bytes: 0,
            rss_peak_bytes: 0,
            vms_peak_bytes: 0,
            cpu_utime: 100,
            cpu_stime: 50,
            threads: 1,
            timestamp: SystemTime::now() - Duration::from_secs(1),
        };
        
        let current = ProcessMetrics {
            pid: 123,
            rss_bytes: 0,
            vms_bytes: 0,
            rss_peak_bytes: 0,
            vms_peak_bytes: 0,
            cpu_utime: 200,
            cpu_stime: 100,
            threads: 1,
            timestamp: SystemTime::now(),
        };
        
        let cpu_pct = current.cpu_percent(&prev);
        
        // 150 jiffies / 100 (Hz) = 1.5 seconds of CPU time
        // Over 1 second wall time = 150% CPU (can be >100% on multi-core)
        assert!(cpu_pct > 100.0 && cpu_pct < 200.0);
    }
    
    #[test]
    fn test_memory_mb_conversion() {
        let metrics = ProcessMetrics {
            pid: 123,
            rss_bytes: 64 * 1024 * 1024, // 64 MB
            vms_bytes: 128 * 1024 * 1024, // 128 MB
            rss_peak_bytes: 96 * 1024 * 1024, // 96 MB
            vms_peak_bytes: 0,
            cpu_utime: 0,
            cpu_stime: 0,
            threads: 1,
            timestamp: SystemTime::now(),
        };
        
        assert_eq!(metrics.memory_mb(), 64.0);
        assert_eq!(metrics.peak_memory_mb(), 96.0);
        assert_eq!(metrics.vms_mb(), 128.0);
    }
    
    #[test]
    fn test_from_pid_current_process() {
        // Test with current process
        let pid = std::process::id();
        let metrics = ProcessMetrics::from_pid(pid);
        
        // Should succeed for current process
        assert!(metrics.is_ok());
        
        if let Ok(m) = metrics {
            assert_eq!(m.pid, pid);
            assert!(m.rss_bytes > 0);
            assert!(m.vms_bytes > 0);
        }
    }
    
    #[test]
    fn test_from_pid_invalid() {
        // Test with invalid PID
        let result = ProcessMetrics::from_pid(999999);
        assert!(result.is_err());
    }
}
