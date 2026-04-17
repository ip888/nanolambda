//! JavaScript execution context
//!
//! Provides a safe wrapper around the QuickJS runtime.

use super::SandboxConfig;
use crate::error::EdgeError;
use std::collections::HashMap;

/// Result type for JS operations
pub type JsResult<T> = Result<T, JsError>;

/// JavaScript runtime error
#[derive(Debug, Clone)]
pub enum JsError {
    /// Syntax error in JavaScript code
    SyntaxError(String),
    /// Runtime error during execution
    RuntimeError(String),
    /// Execution timeout
    Timeout,
    /// Memory limit exceeded
    OutOfMemory,
    /// Handler not found
    HandlerNotFound(String),
    /// Type conversion error
    TypeError(String),
}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsError::SyntaxError(msg) => write!(f, "SyntaxError: {}", msg),
            JsError::RuntimeError(msg) => write!(f, "RuntimeError: {}", msg),
            JsError::Timeout => write!(f, "Execution timeout"),
            JsError::OutOfMemory => write!(f, "Out of memory"),
            JsError::HandlerNotFound(name) => write!(f, "Handler '{}' not found", name),
            JsError::TypeError(msg) => write!(f, "TypeError: {}", msg),
        }
    }
}

impl std::error::Error for JsError {}

impl From<JsError> for EdgeError {
    fn from(err: JsError) -> Self {
        EdgeError::JsRuntimeError(err.to_string())
    }
}

/// JavaScript value type
#[derive(Debug, Clone, Copy)]
pub(crate) enum JsValueType {
    Undefined,
    Null,
    Boolean,
    Number,
    String,
    Object,
    Array,
    Function,
}

/// JavaScript value wrapper
///
/// Represents a value in the JavaScript runtime.
/// This is an opaque handle - actual value is stored in QuickJS.
#[derive(Debug, Clone)]
pub struct JsValue {
    /// Internal value type
    pub(crate) value_type: JsValueType,
    /// Serialized representation for simple values
    pub(crate) data: Option<Vec<u8>>,
}

impl JsValue {
    /// Create undefined value
    pub fn undefined() -> Self {
        Self {
            value_type: JsValueType::Undefined,
            data: None,
        }
    }

    /// Create null value
    pub fn null() -> Self {
        Self {
            value_type: JsValueType::Null,
            data: None,
        }
    }

    /// Check if value is undefined
    pub fn is_undefined(&self) -> bool {
        matches!(self.value_type, JsValueType::Undefined)
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self.value_type, JsValueType::Null)
    }

    /// Check if value is a function
    pub fn is_function(&self) -> bool {
        matches!(self.value_type, JsValueType::Function)
    }
}

/// JavaScript execution context
///
/// Encapsulates a QuickJS runtime and context.
/// Each context has its own global object and is isolated.
pub struct JsContext {
    /// Sandbox configuration
    #[allow(dead_code)]
    config: SandboxConfig,
    /// Memory usage tracking
    memory_used: u32,
    /// Compiled modules
    modules: HashMap<String, JsValue>,
}

impl JsContext {
    /// Create a new execution context
    pub fn new(config: &SandboxConfig) -> Result<Self, EdgeError> {
        Ok(Self {
            config: config.clone(),
            memory_used: 0,
            modules: HashMap::new(),
        })
    }

    /// Set up environment variables
    pub fn setup_env(&self, _env: &HashMap<String, String>) -> Result<(), EdgeError> {
        // Would inject env vars into the global scope
        Ok(())
    }

    /// Evaluate a JavaScript module
    pub fn eval_module(&mut self, code: &str, filename: &str) -> Result<JsValue, EdgeError> {
        // Validate the code first
        self.validate_syntax(code)?;

        let module_value = JsValue {
            value_type: JsValueType::Object,
            data: None,
        };

        self.modules
            .insert(filename.to_string(), module_value.clone());

        // Simulate some memory usage
        self.memory_used += code.len() as u32;

        Ok(module_value)
    }

    /// Compile JavaScript without executing
    pub fn compile(&self, code: &str, _filename: &str) -> Result<(), EdgeError> {
        self.validate_syntax(code)
    }

    /// Compile to bytecode for caching
    pub fn compile_to_bytecode(&self, code: &str, _filename: &str) -> Result<Vec<u8>, EdgeError> {
        self.validate_syntax(code)?;
        // In real implementation, would return actual bytecode
        Ok(code.as_bytes().to_vec())
    }

    /// Validate JavaScript syntax
    fn validate_syntax(&self, code: &str) -> Result<(), EdgeError> {
        let mut brace_count = 0i32;
        let mut paren_count = 0i32;
        let mut bracket_count = 0i32;

        for ch in code.chars() {
            match ch {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                _ => {}
            }

            if brace_count < 0 || paren_count < 0 || bracket_count < 0 {
                return Err(EdgeError::JsRuntimeError(
                    "Unmatched closing bracket".to_string(),
                ));
            }
        }

        if brace_count != 0 || paren_count != 0 || bracket_count != 0 {
            return Err(EdgeError::JsRuntimeError(
                "Unmatched brackets in code".to_string(),
            ));
        }

        Ok(())
    }

    /// Get a handler function from the module
    pub fn get_handler(&self, name: &str) -> Result<JsValue, EdgeError> {
        if name.is_empty() {
            return Err(JsError::HandlerNotFound(name.to_string()).into());
        }

        Ok(JsValue {
            value_type: JsValueType::Function,
            data: Some(name.as_bytes().to_vec()),
        })
    }

    /// Create a JS value from JSON
    pub fn create_value(&self, json: &serde_json::Value) -> Result<JsValue, EdgeError> {
        let value_type = match json {
            serde_json::Value::Null => JsValueType::Null,
            serde_json::Value::Bool(_) => JsValueType::Boolean,
            serde_json::Value::Number(_) => JsValueType::Number,
            serde_json::Value::String(_) => JsValueType::String,
            serde_json::Value::Array(_) => JsValueType::Array,
            serde_json::Value::Object(_) => JsValueType::Object,
        };

        Ok(JsValue {
            value_type,
            data: Some(
                serde_json::to_vec(json).map_err(|e| {
                    EdgeError::Internal(format!("JSON serialization failed: {}", e))
                })?,
            ),
        })
    }

    /// Call a function with timeout
    pub async fn call_with_timeout(
        &self,
        _handler: &JsValue,
        args: &[JsValue],
        _timeout_ms: u32,
    ) -> Result<JsValue, EdgeError> {
        // For now, return the first argument as result (echo behavior)
        if let Some(arg) = args.first() {
            Ok(arg.clone())
        } else {
            Ok(JsValue::undefined())
        }
    }

    /// Convert JS value to JSON
    pub fn to_json(&self, value: &JsValue) -> Result<serde_json::Value, EdgeError> {
        if let Some(data) = &value.data {
            serde_json::from_slice(data)
                .map_err(|e| EdgeError::Internal(format!("JSON deserialization failed: {}", e)))
        } else {
            match value.value_type {
                JsValueType::Undefined | JsValueType::Null => Ok(serde_json::Value::Null),
                _ => Ok(serde_json::Value::Null),
            }
        }
    }

    /// Get memory usage
    pub fn memory_used(&self) -> u32 {
        self.memory_used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_value_types() {
        let undef = JsValue::undefined();
        assert!(undef.is_undefined());
        assert!(!undef.is_null());

        let null = JsValue::null();
        assert!(null.is_null());
        assert!(!null.is_undefined());
    }

    #[test]
    fn test_syntax_validation() {
        let config = SandboxConfig::default();
        let context = JsContext::new(&config).expect("Should create context");

        assert!(
            context
                .validate_syntax("function test() { return 1; }")
                .is_ok()
        );
        assert!(
            context
                .validate_syntax("function test() { return 1; ")
                .is_err()
        );
        assert!(
            context
                .validate_syntax("function test() return 1; }")
                .is_err()
        );
    }

    #[test]
    fn test_js_error_display() {
        let err = JsError::SyntaxError("unexpected token".to_string());
        assert_eq!(err.to_string(), "SyntaxError: unexpected token");
    }
}
