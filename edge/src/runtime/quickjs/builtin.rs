//! Built-in APIs for QuickJS Runtime
//!
//! Provides standard JavaScript APIs:
//! - Console API (log, warn, error, etc.)
//! - JSON API (parse, stringify)
//! - Timer API (setTimeout, setInterval, clearTimeout, clearInterval)

use super::value::{JsArray, JsObject, JsVal};
use super::{ConsoleLog, ConsoleLogLevel};
use std::collections::HashMap;

/// Console API implementation
#[derive(Debug, Default)]
pub struct ConsoleApi {
    /// Captured logs
    logs: Vec<ConsoleLog>,
    /// Execution start time for timestamps
    start_time: Option<web_time::Instant>,
}

impl ConsoleApi {
    /// Create a new console API
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
            start_time: Some(web_time::Instant::now()),
        }
    }

    /// Log a message
    pub fn log(&mut self, args: &[JsVal]) {
        self.add_log(ConsoleLogLevel::Log, args);
    }

    /// Debug message
    pub fn debug(&mut self, args: &[JsVal]) {
        self.add_log(ConsoleLogLevel::Debug, args);
    }

    /// Info message
    pub fn info(&mut self, args: &[JsVal]) {
        self.add_log(ConsoleLogLevel::Info, args);
    }

    /// Warning message
    pub fn warn(&mut self, args: &[JsVal]) {
        self.add_log(ConsoleLogLevel::Warn, args);
    }

    /// Error message
    pub fn error(&mut self, args: &[JsVal]) {
        self.add_log(ConsoleLogLevel::Error, args);
    }

    /// Trace message with stack
    pub fn trace(&mut self, args: &[JsVal]) {
        self.add_log(ConsoleLogLevel::Trace, args);
    }

    /// Add log entry
    fn add_log(&mut self, level: ConsoleLogLevel, args: &[JsVal]) {
        let message = if args.is_empty() {
            String::new()
        } else {
            args.iter()
                .map(|v| self.format_value(v))
                .collect::<Vec<_>>()
                .join(" ")
        };

        let timestamp_ms = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        self.logs.push(ConsoleLog {
            level,
            message,
            args: args.iter().map(|v| v.to_json()).collect(),
            timestamp_ms,
        });
    }

    /// Format a value for console output
    fn format_value(&self, value: &JsVal) -> String {
        match value {
            JsVal::Undefined => "undefined".to_string(),
            JsVal::Null => "null".to_string(),
            JsVal::Boolean(b) => b.to_string(),
            JsVal::Number(n) => {
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if *n > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
                } else {
                    n.to_string()
                }
            }
            JsVal::String(s) => s.clone(),
            JsVal::Array(arr) => {
                let items: Vec<_> = arr.elements.iter().map(|v| self.format_value(v)).collect();
                format!("[{}]", items.join(", "))
            }
            JsVal::Object(obj) => {
                let items: Vec<_> = obj
                    .properties
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.format_value(v)))
                    .collect();
                format!("{{ {} }}", items.join(", "))
            }
            JsVal::Function(f) => {
                if f.name.is_empty() {
                    "[Function (anonymous)]".to_string()
                } else {
                    format!("[Function: {}]", f.name)
                }
            }
        }
    }

    /// Get all captured logs
    pub fn get_logs(&self) -> &[ConsoleLog] {
        &self.logs
    }

    /// Take all logs (consumes them)
    pub fn take_logs(&mut self) -> Vec<ConsoleLog> {
        std::mem::take(&mut self.logs)
    }

    /// Clear all logs
    pub fn clear(&mut self) {
        self.logs.clear();
    }

    /// Create console object for JS runtime
    pub fn create_console_object(&self) -> JsObject {
        let mut obj = JsObject::new();
        // Methods would be bound as native functions in actual implementation
        obj.set("_type", JsVal::String("console".to_string()));
        obj
    }
}

/// JSON API implementation
#[derive(Debug, Default)]
pub struct JsonApi;

impl JsonApi {
    /// Create a new JSON API
    pub fn new() -> Self {
        Self
    }

    /// Parse JSON string to JS value
    pub fn parse(&self, text: &str) -> Result<JsVal, String> {
        let json: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("SyntaxError: {}", e))?;
        Ok(JsVal::from_json(json))
    }

    /// Stringify JS value to JSON
    pub fn stringify(&self, value: &JsVal) -> Result<String, String> {
        self.stringify_with_options(value, None, None)
    }

    /// Stringify with optional replacer and space
    pub fn stringify_with_options(
        &self,
        value: &JsVal,
        _replacer: Option<&JsVal>,
        space: Option<&JsVal>,
    ) -> Result<String, String> {
        let json = value.to_json();

        match space {
            Some(JsVal::Number(n)) => {
                let indent = (*n as usize).min(10);
                serde_json::to_string_pretty(&json)
                    .map(|s| {
                        // Replace default 2-space indent with requested indent
                        if indent != 2 {
                            s.replace("  ", &" ".repeat(indent))
                        } else {
                            s
                        }
                    })
                    .map_err(|e| e.to_string())
            }
            Some(JsVal::String(s)) => {
                let indent = &s[..s.len().min(10)];
                serde_json::to_string_pretty(&json)
                    .map(|result| result.replace("  ", indent))
                    .map_err(|e| e.to_string())
            }
            _ => serde_json::to_string(&json).map_err(|e| e.to_string()),
        }
    }

    /// Create JSON object for JS runtime
    pub fn create_json_object(&self) -> JsObject {
        let mut obj = JsObject::new();
        obj.set("_type", JsVal::String("JSON".to_string()));
        obj
    }
}

/// Timer handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(pub u32);

/// Timer entry
#[derive(Debug)]
pub struct TimerEntry {
    /// Timer ID
    pub id: TimerId,
    /// Callback function
    pub callback: JsVal,
    /// Delay in milliseconds
    pub delay_ms: u32,
    /// Whether this is an interval
    pub is_interval: bool,
    /// When the timer was set
    pub set_at: web_time::Instant,
    /// Whether the timer has been cleared
    pub cleared: bool,
}

/// Timer API implementation
#[derive(Debug, Default)]
pub struct TimerApi {
    /// Active timers
    timers: HashMap<TimerId, TimerEntry>,
    /// Next timer ID
    next_id: u32,
}

impl TimerApi {
    /// Create a new timer API
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
            next_id: 1,
        }
    }

    /// Set a timeout
    pub fn set_timeout(&mut self, callback: JsVal, delay_ms: u32) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;

        self.timers.insert(
            id,
            TimerEntry {
                id,
                callback,
                delay_ms,
                is_interval: false,
                set_at: web_time::Instant::now(),
                cleared: false,
            },
        );

        id
    }

    /// Set an interval
    pub fn set_interval(&mut self, callback: JsVal, delay_ms: u32) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;

        self.timers.insert(
            id,
            TimerEntry {
                id,
                callback,
                delay_ms,
                is_interval: true,
                set_at: web_time::Instant::now(),
                cleared: false,
            },
        );

        id
    }

    /// Clear a timeout or interval
    pub fn clear_timer(&mut self, id: TimerId) {
        if let Some(timer) = self.timers.get_mut(&id) {
            timer.cleared = true;
        }
    }

    /// Get ready timers (those that have elapsed)
    pub fn get_ready_timers(&mut self) -> Vec<(TimerId, JsVal, bool)> {
        let now = web_time::Instant::now();
        let mut ready = Vec::new();

        for timer in self.timers.values_mut() {
            if timer.cleared {
                continue;
            }

            let elapsed = now.duration_since(timer.set_at).as_millis() as u32;
            if elapsed >= timer.delay_ms {
                ready.push((timer.id, timer.callback.clone(), timer.is_interval));

                if timer.is_interval {
                    // Reset for next interval
                    timer.set_at = now;
                } else {
                    timer.cleared = true;
                }
            }
        }

        ready
    }

    /// Clean up cleared timers
    pub fn cleanup(&mut self) {
        self.timers.retain(|_, t| !t.cleared);
    }

    /// Check if there are pending timers
    pub fn has_pending(&self) -> bool {
        self.timers.values().any(|t| !t.cleared)
    }

    /// Get number of active timers
    pub fn active_count(&self) -> usize {
        self.timers.values().filter(|t| !t.cleared).count()
    }
}

/// Math API - provides common math functions
#[derive(Debug, Default)]
pub struct MathApi;

impl MathApi {
    /// Create math object for JS runtime
    pub fn create_math_object() -> JsObject {
        let mut obj = JsObject::new();

        // Constants
        obj.set("E", JsVal::Number(std::f64::consts::E));
        obj.set("LN10", JsVal::Number(std::f64::consts::LN_10));
        obj.set("LN2", JsVal::Number(std::f64::consts::LN_2));
        obj.set("LOG10E", JsVal::Number(std::f64::consts::LOG10_E));
        obj.set("LOG2E", JsVal::Number(std::f64::consts::LOG2_E));
        obj.set("PI", JsVal::Number(std::f64::consts::PI));
        obj.set("SQRT1_2", JsVal::Number(std::f64::consts::FRAC_1_SQRT_2));
        obj.set("SQRT2", JsVal::Number(std::f64::consts::SQRT_2));

        obj
    }

    /// abs(x)
    pub fn abs(x: f64) -> f64 {
        x.abs()
    }

    /// ceil(x)
    pub fn ceil(x: f64) -> f64 {
        x.ceil()
    }

    /// floor(x)
    pub fn floor(x: f64) -> f64 {
        x.floor()
    }

    /// round(x)
    pub fn round(x: f64) -> f64 {
        x.round()
    }

    /// trunc(x)
    pub fn trunc(x: f64) -> f64 {
        x.trunc()
    }

    /// max(a, b, ...)
    pub fn max(values: &[f64]) -> f64 {
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }

    /// min(a, b, ...)
    pub fn min(values: &[f64]) -> f64 {
        values.iter().copied().fold(f64::INFINITY, f64::min)
    }

    /// pow(x, y)
    pub fn pow(x: f64, y: f64) -> f64 {
        x.powf(y)
    }

    /// sqrt(x)
    pub fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }

    /// sin(x)
    pub fn sin(x: f64) -> f64 {
        x.sin()
    }

    /// cos(x)
    pub fn cos(x: f64) -> f64 {
        x.cos()
    }

    /// tan(x)
    pub fn tan(x: f64) -> f64 {
        x.tan()
    }

    /// random()
    pub fn random() -> f64 {
        // Use getrandom for WASM-compatible randomness
        let mut buf = [0u8; 8];
        let _ = getrandom::fill(&mut buf);
        let value = u64::from_le_bytes(buf);
        value as f64 / u64::MAX as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_log() {
        let mut console = ConsoleApi::new();
        console.log(&[JsVal::String("hello".to_string()), JsVal::Number(42.0)]);

        let logs = console.get_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, ConsoleLogLevel::Log);
        assert_eq!(logs[0].message, "hello 42");
    }

    #[test]
    fn test_console_format() {
        let console = ConsoleApi::new();

        assert_eq!(console.format_value(&JsVal::Undefined), "undefined");
        assert_eq!(console.format_value(&JsVal::Null), "null");
        assert_eq!(console.format_value(&JsVal::Number(f64::NAN)), "NaN");
        assert_eq!(
            console.format_value(&JsVal::Number(f64::INFINITY)),
            "Infinity"
        );
    }

    #[test]
    fn test_json_parse() {
        let json = JsonApi::new();

        let result = json.parse(r#"{"name": "test", "value": 42}"#).unwrap();
        let obj = result.as_object().unwrap();

        assert_eq!(obj.get("name").and_then(|v| v.as_string()), Some("test"));
        assert_eq!(obj.get("value").and_then(|v| v.as_number()), Some(42.0));
    }

    #[test]
    fn test_json_stringify() {
        let json = JsonApi::new();

        let mut obj = JsObject::new();
        obj.set("name", JsVal::String("test".to_string()));
        obj.set("value", JsVal::Number(42.0));

        let result = json.stringify(&JsVal::Object(obj)).unwrap();
        assert!(result.contains("\"name\""));
        assert!(result.contains("\"test\""));
    }

    #[test]
    fn test_timer_set_and_clear() {
        let mut timers = TimerApi::new();

        let id = timers.set_timeout(JsVal::String("callback".to_string()), 100);
        assert_eq!(timers.active_count(), 1);

        timers.clear_timer(id);
        timers.cleanup();
        assert_eq!(timers.active_count(), 0);
    }

    #[test]
    fn test_math_functions() {
        assert_eq!(MathApi::abs(-5.0), 5.0);
        assert_eq!(MathApi::ceil(4.3), 5.0);
        assert_eq!(MathApi::floor(4.7), 4.0);
        assert_eq!(MathApi::round(4.5), 5.0);
        assert_eq!(MathApi::max(&[1.0, 5.0, 3.0]), 5.0);
        assert_eq!(MathApi::min(&[1.0, 5.0, 3.0]), 1.0);
    }

    #[test]
    fn test_math_constants() {
        let obj = MathApi::create_math_object();
        assert!(
            (obj.get("PI").and_then(|v| v.as_number()).unwrap() - std::f64::consts::PI).abs()
                < 1e-10
        );
        assert!(
            (obj.get("E").and_then(|v| v.as_number()).unwrap() - std::f64::consts::E).abs() < 1e-10
        );
    }
}
