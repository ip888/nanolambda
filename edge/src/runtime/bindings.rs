//! JavaScript API bindings
//!
//! Provides native function implementations for the JavaScript runtime.

use super::context::JsValueType;
use super::{JsError, JsResult, JsValue};
use worker::console_log;

/// Native API bindings for JavaScript
pub struct JsBindings {
    /// Registered native function names
    function_names: Vec<String>,
}

impl JsBindings {
    /// Create a new bindings registry
    pub fn new() -> Self {
        let mut bindings = Self {
            function_names: Vec::new(),
        };

        // Register default bindings
        bindings.register_defaults();

        bindings
    }

    /// Register default API functions
    fn register_defaults(&mut self) {
        // Console API
        self.function_names.push("console.log".to_string());
        self.function_names.push("console.error".to_string());
        self.function_names.push("console.warn".to_string());

        // Base64
        self.function_names.push("btoa".to_string());
        self.function_names.push("atob".to_string());

        // JSON
        self.function_names.push("JSON.stringify".to_string());
        self.function_names.push("JSON.parse".to_string());
    }

    /// Check if a function is registered
    pub fn has_function(&self, name: &str) -> bool {
        self.function_names.iter().any(|n| n == name)
    }

    /// Call a native function by name
    pub fn call(&self, name: &str, args: Vec<JsValue>) -> JsResult<JsValue> {
        match name {
            "console.log" | "console.error" | "console.warn" => {
                // Log and return undefined
                let message = args
                    .iter()
                    .map(|v| format!("{:?}", v))
                    .collect::<Vec<_>>()
                    .join(" ");
                console_log!("{}", message);
                Ok(JsValue::undefined())
            }
            "btoa" => Self::btoa(args),
            "atob" => Self::atob(args),
            "JSON.stringify" => Self::json_stringify(args),
            "JSON.parse" => Self::json_parse(args),
            _ => Err(JsError::RuntimeError(format!(
                "Native function '{}' not found",
                name
            ))),
        }
    }

    /// btoa - encode to base64
    fn btoa(args: Vec<JsValue>) -> JsResult<JsValue> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let input = args
            .first()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsError::TypeError("btoa requires a string argument".to_string()))?;

        let encoded = STANDARD.encode(input.as_bytes());
        Ok(JsValue::string(encoded))
    }

    /// atob - decode from base64
    fn atob(args: Vec<JsValue>) -> JsResult<JsValue> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let input = args
            .first()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsError::TypeError("atob requires a string argument".to_string()))?;

        let decoded = STANDARD
            .decode(input)
            .map_err(|e| JsError::RuntimeError(format!("Invalid base64: {}", e)))?;

        let string = String::from_utf8(decoded)
            .map_err(|e| JsError::RuntimeError(format!("Invalid UTF-8: {}", e)))?;

        Ok(JsValue::string(string))
    }

    /// JSON.stringify
    fn json_stringify(args: Vec<JsValue>) -> JsResult<JsValue> {
        let value = args.first().ok_or_else(|| {
            JsError::TypeError("JSON.stringify requires an argument".to_string())
        })?;

        let json = value
            .to_json()
            .map_err(|e| JsError::RuntimeError(format!("Stringify failed: {}", e)))?;

        let string = serde_json::to_string(&json)
            .map_err(|e| JsError::RuntimeError(format!("JSON serialization failed: {}", e)))?;

        Ok(JsValue::string(string))
    }

    /// JSON.parse
    fn json_parse(args: Vec<JsValue>) -> JsResult<JsValue> {
        let input = args
            .first()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsError::TypeError("JSON.parse requires a string argument".to_string()))?;

        let value: serde_json::Value = serde_json::from_str(&input)
            .map_err(|e| JsError::SyntaxError(format!("JSON parse error: {}", e)))?;

        JsValue::from_json(&value)
    }
}

impl Default for JsBindings {
    fn default() -> Self {
        Self::new()
    }
}

// Extend JsValue with helper methods
impl JsValue {
    /// Create a string value
    pub fn string(s: String) -> Self {
        Self {
            value_type: JsValueType::String,
            data: Some(s.into_bytes()),
        }
    }

    /// Get as string if this is a string value
    pub fn as_string(&self) -> Option<String> {
        if matches!(self.value_type, JsValueType::String) {
            self.data
                .as_ref()
                .and_then(|d| String::from_utf8(d.clone()).ok())
        } else {
            None
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Result<serde_json::Value, String> {
        if let Some(data) = &self.data {
            serde_json::from_slice(data).map_err(|e| e.to_string())
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    /// Create from JSON
    pub fn from_json(json: &serde_json::Value) -> JsResult<Self> {
        let data =
            serde_json::to_vec(json).map_err(|e| JsError::RuntimeError(e.to_string()))?;

        let value_type = match json {
            serde_json::Value::Null => JsValueType::Null,
            serde_json::Value::Bool(_) => JsValueType::Boolean,
            serde_json::Value::Number(_) => JsValueType::Number,
            serde_json::Value::String(_) => JsValueType::String,
            serde_json::Value::Array(_) => JsValueType::Array,
            serde_json::Value::Object(_) => JsValueType::Object,
        };

        Ok(Self {
            value_type,
            data: Some(data),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bindings_creation() {
        let bindings = JsBindings::new();
        assert!(bindings.has_function("console.log"));
        assert!(bindings.has_function("btoa"));
        assert!(bindings.has_function("JSON.stringify"));
    }

    #[test]
    fn test_base64_roundtrip() {
        let bindings = JsBindings::new();

        let input = JsValue::string("Hello, World!".to_string());
        let encoded = bindings
            .call("btoa", vec![input])
            .expect("btoa should work");

        let decoded = bindings
            .call("atob", vec![encoded])
            .expect("atob should work");
        assert_eq!(decoded.as_string(), Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_js_value_string() {
        let value = JsValue::string("test".to_string());
        assert_eq!(value.as_string(), Some("test".to_string()));
    }
}
