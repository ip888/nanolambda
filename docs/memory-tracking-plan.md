# Memory Tracking Implementation Plan

## Overview

Implement real-time process memory tracking using the Linux /proc filesystem to replace placeholder memory values with actual measurements.

## Current State

Currently, memory tracking returns hardcoded placeholder values:
- Memory: 64 MB (placeholder)
- No RSS/VMS tracking
- No CPU usage monitoring
- No peak memory tracking

## Goals

1. **Real Memory Metrics** from /proc filesystem
   - RSS (Resident Set Size) - actual RAM used
   - VMS (Virtual Memory Size) - total virtual memory
   - Peak memory usage
   
2. **CPU Usage Tracking**
   - CPU time consumed by process
   - CPU percentage

3. **Process Health Monitoring**
   - Process alive/dead status
   - Age of process
   - Resource limits

## Linux /proc Filesystem Structure

For a process with PID, relevant files:
- `/proc/{pid}/stat` - Process status (CPU time, memory)
- `/proc/{pid}/statm` - Memory statistics (pages)
- `/proc/{pid}/status` - Human-readable status (includes VmRSS, VmSize)

### /proc/{pid}/status Format
```
Name:   python3
VmSize:     12345 kB  # Virtual memory size
VmRSS:       6789 kB  # Resident set size (physical memory)
VmPeak:     15000 kB  # Peak virtual memory
VmHWM:       8000 kB  # Peak resident set size
Threads:        1
```

### /proc/{pid}/stat Format
Space-separated values including:
- Position 14: utime (CPU time in user mode)
- Position 15: stime (CPU time in kernel mode)
- Position 23: vsize (virtual memory size in bytes)
- Position 24: rss (resident set size in pages)

## Implementation Plan

### 1. Create Process Metrics Module

```rust
// crates/runtime/src/metrics.rs
pub struct ProcessMetrics {
    pub pid: u32,
    pub rss_bytes: u64,      // Resident Set Size (RAM used)
    pub vms_bytes: u64,      // Virtual Memory Size
    pub rss_peak_bytes: u64, // Peak RSS
    pub vms_peak_bytes: u64, // Peak VMS
    pub cpu_utime: u64,      // User mode CPU time (jiffies)
    pub cpu_stime: u64,      // System mode CPU time (jiffies)
    pub threads: u32,
    pub timestamp: SystemTime,
}

impl ProcessMetrics {
    pub fn from_pid(pid: u32) -> Result<Self>;
    pub fn from_status_file(pid: u32) -> Result<Self>;
    pub fn from_stat_file(pid: u32) -> Result<Self>;
    pub fn cpu_percent(&self, previous: &ProcessMetrics) -> f64;
    pub fn memory_mb(&self) -> f64;
}
```

### 2. Update ProcessPool to Track Metrics

```rust
// crates/runtime/src/pool.rs
pub struct WarmProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _code_hash: String,
    stats: ProcessStats,
    metrics: Option<ProcessMetrics>,  // NEW
    last_metrics: Option<ProcessMetrics>,  // NEW: for delta calculations
}

impl WarmProcess {
    pub fn update_metrics(&mut self) -> Result<()> {
        let pid = self.child.id();
        let new_metrics = ProcessMetrics::from_pid(pid)?;
        self.last_metrics = self.metrics.take();
        self.metrics = Some(new_metrics);
        Ok(())
    }
    
    pub fn get_memory_mb(&self) -> u64 {
        self.metrics
            .as_ref()
            .map(|m| (m.rss_bytes / 1024 / 1024) as u64)
            .unwrap_or(64) // Fallback
    }
    
    pub fn get_cpu_percent(&self) -> f64 {
        match (&self.metrics, &self.last_metrics) {
            (Some(current), Some(previous)) => current.cpu_percent(previous),
            _ => 0.0,
        }
    }
}
```

### 3. Update Executor to Return Real Metrics

```rust
// crates/runtime/src/executor.rs
pub struct ExecutionResult {
    pub output: Value,
    pub execution_ms: u64,
    pub memory_mb: u64,      // Now real RSS!
    pub memory_peak_mb: u64, // NEW
    pub cpu_percent: f64,    // NEW
    pub cold_start: bool,
}
```

### 4. Periodic Metric Collection

Add background task to collect metrics periodically:

```rust
impl ProcessPool {
    pub fn start_metrics_collector(&self) -> tokio::task::JoinHandle<()> {
        let pool = Arc::clone(&self.processes);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                if let Ok(mut processes) = pool.lock() {
                    for process in processes.values_mut() {
                        if let Err(e) = process.update_metrics() {
                            warn!("Failed to update metrics: {}", e);
                        }
                    }
                }
            }
        })
    }
}
```

## Parsing Strategy

### Option 1: Parse /proc/{pid}/status (Easier)
- Human-readable format
- Includes peak values
- Easier to parse
- Slightly slower (string parsing)

### Option 2: Parse /proc/{pid}/stat (Faster)
- Binary-friendly format
- Faster to parse
- No peak values
- Requires page size calculation

**Recommendation**: Use `/proc/{pid}/status` for simplicity and peak values.

## Error Handling

1. **Process Dies**: Return last known metrics
2. **Permission Denied**: Log warning, use fallback
3. **File Not Found**: Process terminated, remove from pool
4. **Parse Error**: Log error, use previous metrics

## Testing Strategy

```rust
#[test]
fn test_parse_proc_status() {
    let content = r#"
Name:   python3
VmSize:     12345 kB
VmRSS:       6789 kB
VmPeak:     15000 kB
VmHWM:       8000 kB
Threads:        1
"#;
    let metrics = ProcessMetrics::parse_status(content).unwrap();
    assert_eq!(metrics.vms_bytes, 12345 * 1024);
    assert_eq!(metrics.rss_bytes, 6789 * 1024);
}

#[test]
fn test_cpu_percent_calculation() {
    let prev = ProcessMetrics {
        cpu_utime: 100,
        cpu_stime: 50,
        timestamp: SystemTime::now() - Duration::from_secs(1),
        ..Default::default()
    };
    
    let current = ProcessMetrics {
        cpu_utime: 200,
        cpu_stime: 100,
        timestamp: SystemTime::now(),
        ..Default::default()
    };
    
    let cpu_pct = current.cpu_percent(&prev);
    assert!(cpu_pct > 0.0);
}
```

## Integration Points

### 1. Runtime Executor
```rust
let result = executor.execute(code, payload).await?;
println!("Memory used: {} MB", result.memory_mb);
println!("Peak memory: {} MB", result.memory_peak_mb);
println!("CPU usage: {:.1}%", result.cpu_percent);
```

### 2. API Responses
```json
{
  "result": { ... },
  "metrics": {
    "execution_ms": 15,
    "memory_mb": 45,
    "memory_peak_mb": 52,
    "cpu_percent": 3.5,
    "cold_start": false
  }
}
```

### 3. Monitoring/Logs
```
[INFO] Function execution complete: name=my-func, time=15ms, mem=45MB, cpu=3.5%
```

## Performance Considerations

1. **Caching**: Update metrics every 100ms, not every invocation
2. **Async Reading**: Use tokio::fs for non-blocking /proc reads
3. **Batch Collection**: Update all process metrics in single pass
4. **Lazy Evaluation**: Only read /proc when metrics requested

## Dependencies

No new dependencies needed! Standard library is sufficient:
- `std::fs` - Read /proc files
- `std::process::Child::id()` - Get PID
- String parsing with `str::lines()` and `str::split()`

## Implementation Steps

1. ✅ Create `crates/runtime/src/metrics.rs`
2. ✅ Implement `ProcessMetrics` struct and parsing
3. ✅ Add tests for /proc parsing
4. ✅ Update `WarmProcess` with metrics tracking
5. ✅ Update `ProcessPool` to collect metrics
6. ✅ Update `ExecutionResult` to include real metrics
7. ✅ Test with real Python processes
8. ✅ Add error handling for edge cases
9. ✅ Document metric API usage

## Expected Benefits

1. **Accurate Memory Tracking**: Know actual RAM usage vs placeholder
2. **Resource Optimization**: Identify memory leaks, high CPU processes
3. **Better Monitoring**: Real metrics for dashboards/alerts
4. **Debugging**: Understand performance characteristics
5. **Production Ready**: Professional-grade observability

## Next Steps After Implementation

1. Add Prometheus metrics exporter
2. Create Grafana dashboard
3. Set up alerting thresholds
4. Add metric-based autoscaling
5. Optimize based on real data
