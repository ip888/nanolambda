//! QuickJS Execution Engine
//!
//! This module provides the main execution engine for JavaScript code.

use super::ast::{Lexer, Token, TokenType};
use super::builtin::{ConsoleApi, JsonApi, MathApi, TimerApi};
use super::fetch::FetchApi;
use super::value::{JsArray, JsFunction, JsObject, JsVal};
use super::{
    ConsoleLog, ExecutionStats, QuickJsConfig, QuickJsError, QuickJsResult, QuickJsResultType,
};
use std::collections::HashMap;

/// Scope for variable bindings
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Variable bindings
    bindings: HashMap<String, JsVal>,
    /// Parent scope reference (index into scope stack)
    parent: Option<usize>,
    /// Whether this is a function scope
    is_function_scope: bool,
}

impl Scope {
    /// Create a new global scope
    pub fn global() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
            is_function_scope: false,
        }
    }

    /// Create a new block scope
    pub fn block(parent: usize) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(parent),
            is_function_scope: false,
        }
    }

    /// Create a new function scope
    pub fn function(parent: usize) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(parent),
            is_function_scope: true,
        }
    }

    /// Define a variable in this scope
    pub fn define(&mut self, name: impl Into<String>, value: JsVal) {
        self.bindings.insert(name.into(), value);
    }

    /// Get a variable from this scope (not looking up the chain)
    pub fn get_own(&self, name: &str) -> Option<&JsVal> {
        self.bindings.get(name)
    }

    /// Set a variable in this scope
    pub fn set(&mut self, name: &str, value: JsVal) -> bool {
        if self.bindings.contains_key(name) {
            self.bindings.insert(name.to_string(), value);
            true
        } else {
            false
        }
    }
}

/// QuickJS execution engine
pub struct QuickJsEngine {
    /// Configuration
    config: QuickJsConfig,
    /// Scope stack
    scopes: Vec<Scope>,
    /// Current scope index
    current_scope: usize,
    /// Console API
    console: ConsoleApi,
    /// JSON API
    json: JsonApi,
    /// Timer API
    timers: TimerApi,
    /// Fetch API
    fetch: FetchApi,
    /// Execution statistics
    stats: ExecutionStats,
    /// Start time for timeout tracking
    start_time: Option<web_time::Instant>,
}

impl QuickJsEngine {
    /// Create a new engine with the given configuration
    pub fn new(config: QuickJsConfig) -> Self {
        let mut engine = Self {
            config,
            scopes: vec![Scope::global()],
            current_scope: 0,
            console: ConsoleApi::new(),
            json: JsonApi::new(),
            timers: TimerApi::new(),
            fetch: FetchApi::new(),
            stats: ExecutionStats::default(),
            start_time: None,
        };
        engine.setup_globals();
        engine
    }

    /// Set up global variables and functions
    fn setup_globals(&mut self) {
        let scope = &mut self.scopes[0];

        // Undefined and null
        scope.define("undefined", JsVal::Undefined);
        scope.define("null", JsVal::Null);

        // Infinity and NaN
        scope.define("Infinity", JsVal::Number(f64::INFINITY));
        scope.define("NaN", JsVal::Number(f64::NAN));

        // Global objects
        if self.config.enable_console {
            scope.define(
                "console",
                JsVal::Object(self.console.create_console_object()),
            );
        }
        scope.define("JSON", JsVal::Object(self.json.create_json_object()));
        scope.define("Math", JsVal::Object(MathApi::create_math_object()));

        // Fetch API (if enabled)
        if self.config.enable_fetch {
            scope.define("fetch", JsVal::Function(FetchApi::create_fetch_function()));
            scope.define(
                "Request",
                JsVal::Object(FetchApi::create_request_constructor()),
            );
            scope.define(
                "Response",
                JsVal::Object(FetchApi::create_response_constructor()),
            );
            scope.define(
                "Headers",
                JsVal::Object(FetchApi::create_headers_constructor()),
            );
        }

        // Global functions
        scope.define(
            "isNaN",
            JsVal::Function(JsFunction::new("isNaN", vec!["value".to_string()], "")),
        );
        scope.define(
            "isFinite",
            JsVal::Function(JsFunction::new("isFinite", vec!["value".to_string()], "")),
        );
        scope.define(
            "parseFloat",
            JsVal::Function(JsFunction::new(
                "parseFloat",
                vec!["string".to_string()],
                "",
            )),
        );
        scope.define(
            "parseInt",
            JsVal::Function(JsFunction::new(
                "parseInt",
                vec!["string".to_string(), "radix".to_string()],
                "",
            )),
        );

        // Object constructor
        scope.define("Object", JsVal::Object(JsObject::new()));
        scope.define("Array", JsVal::Object(JsObject::new()));
        scope.define("String", JsVal::Object(JsObject::new()));
        scope.define("Number", JsVal::Object(JsObject::new()));
        scope.define("Boolean", JsVal::Object(JsObject::new()));
        scope.define("Date", JsVal::Object(JsObject::new()));
        scope.define("RegExp", JsVal::Object(JsObject::new()));
        scope.define("Error", JsVal::Object(JsObject::new()));
        scope.define("Promise", JsVal::Object(JsObject::new()));
    }

    /// Execute JavaScript code
    pub fn execute(
        &mut self,
        code: &str,
        handler: Option<&str>,
        event: Option<JsVal>,
    ) -> QuickJsResultType<QuickJsResult> {
        self.start_time = Some(web_time::Instant::now());
        self.stats = ExecutionStats::default();

        // Tokenize
        let tokens = self.tokenize(code)?;

        // Check for timeout/memory limits periodically
        self.check_limits()?;

        // Parse and evaluate
        let result = self.evaluate_tokens(&tokens, handler, event)?;

        // Collect stats
        let duration = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        self.stats.execution_time_ms = duration;

        Ok(QuickJsResult {
            value: result.to_json(),
            console_logs: self.console.take_logs(),
            stats: self.stats.clone(),
            success: true,
            error: None,
        })
    }

    /// Tokenize source code
    fn tokenize(&mut self, code: &str) -> QuickJsResultType<Vec<Token>> {
        let mut lexer = Lexer::new(code);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token()?;
            let is_eof = matches!(token.token_type, TokenType::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        self.stats.memory_used += (code.len() + tokens.len() * 32) as u32;
        Ok(tokens)
    }

    /// Evaluate tokens
    fn evaluate_tokens(
        &mut self,
        tokens: &[Token],
        handler: Option<&str>,
        event: Option<JsVal>,
    ) -> QuickJsResultType<JsVal> {
        let mut result = JsVal::Undefined;
        let mut i = 0;

        while i < tokens.len() {
            let token = &tokens[i];

            match &token.token_type {
                TokenType::Eof => break,

                // Variable declarations
                TokenType::Const | TokenType::Let | TokenType::Var => {
                    let (val, consumed) = self.parse_variable_declaration(tokens, i)?;
                    result = val;
                    i += consumed;
                }

                // Function declarations
                TokenType::Function => {
                    let (val, consumed) = self.parse_function_declaration(tokens, i)?;
                    result = val;
                    i += consumed;
                }

                // Return statement (outside function, acts as module export)
                TokenType::Return => {
                    i += 1;
                    if i < tokens.len()
                        && !matches!(
                            tokens[i].token_type,
                            TokenType::Semicolon | TokenType::RightBrace
                        )
                    {
                        let (val, consumed) = self.parse_expression(tokens, i)?;
                        result = val;
                        i += consumed;
                    }
                }

                // Expression statement
                _ => {
                    let (val, consumed) = self.parse_expression(tokens, i)?;
                    result = val;
                    i += consumed;

                    // Skip semicolon if present
                    if i < tokens.len() && matches!(tokens[i].token_type, TokenType::Semicolon) {
                        i += 1;
                    }
                }
            }

            self.check_limits()?;
        }

        // If a handler was specified, try to call it
        if let Some(handler_name) = handler {
            if let Some(func) = self.get_variable(handler_name) {
                if matches!(func, JsVal::Function(_)) {
                    let args = event.map(|e| vec![e]).unwrap_or_default();
                    return self.call_function(&func, &args);
                }
            }
        }

        Ok(result)
    }

    /// Parse a variable declaration
    fn parse_variable_declaration(
        &mut self,
        tokens: &[Token],
        start: usize,
    ) -> QuickJsResultType<(JsVal, usize)> {
        let mut i = start + 1; // Skip const/let/var

        if i >= tokens.len() {
            return Err(QuickJsError::SyntaxError {
                message: "Expected identifier after variable declaration".to_string(),
                line: tokens[start].position.line,
                column: tokens[start].position.column,
            });
        }

        // Get variable name
        let name = match &tokens[i].token_type {
            TokenType::Identifier(n) => n.clone(),
            _ => {
                return Err(QuickJsError::SyntaxError {
                    message: "Expected identifier".to_string(),
                    line: tokens[i].position.line,
                    column: tokens[i].position.column,
                });
            }
        };
        i += 1;

        // Check for initializer
        let value = if i < tokens.len() && matches!(tokens[i].token_type, TokenType::Equals) {
            i += 1;
            let (val, consumed) = self.parse_expression(tokens, i)?;
            i += consumed;
            val
        } else {
            JsVal::Undefined
        };

        // Skip semicolon
        if i < tokens.len() && matches!(tokens[i].token_type, TokenType::Semicolon) {
            i += 1;
        }

        // Define the variable
        self.scopes[self.current_scope].define(&name, value.clone());

        Ok((value, i - start))
    }

    /// Parse a function declaration
    fn parse_function_declaration(
        &mut self,
        tokens: &[Token],
        start: usize,
    ) -> QuickJsResultType<(JsVal, usize)> {
        let mut i = start + 1; // Skip 'function'

        // Check for async
        let is_async = if start > 0 {
            matches!(tokens[start - 1].token_type, TokenType::Async)
        } else {
            false
        };

        // Get function name
        let name = match tokens.get(i).map(|t| &t.token_type) {
            Some(TokenType::Identifier(n)) => {
                i += 1;
                n.clone()
            }
            _ => String::new(), // Anonymous function
        };

        // Parse parameters
        if !matches!(
            tokens.get(i).map(|t| &t.token_type),
            Some(TokenType::LeftParen)
        ) {
            return Err(QuickJsError::SyntaxError {
                message: "Expected '(' after function name".to_string(),
                line: tokens.get(i).map(|t| t.position.line).unwrap_or(0),
                column: tokens.get(i).map(|t| t.position.column).unwrap_or(0),
            });
        }
        i += 1;

        let mut params = Vec::new();
        while i < tokens.len() && !matches!(tokens[i].token_type, TokenType::RightParen) {
            if let TokenType::Identifier(param) = &tokens[i].token_type {
                params.push(param.clone());
                i += 1;
                if matches!(tokens.get(i).map(|t| &t.token_type), Some(TokenType::Comma)) {
                    i += 1;
                }
            } else {
                break;
            }
        }

        // Skip ')'
        if matches!(
            tokens.get(i).map(|t| &t.token_type),
            Some(TokenType::RightParen)
        ) {
            i += 1;
        }

        // Find function body (between braces)
        let body_start = i;
        if !matches!(
            tokens.get(i).map(|t| &t.token_type),
            Some(TokenType::LeftBrace)
        ) {
            return Err(QuickJsError::SyntaxError {
                message: "Expected '{' after function parameters".to_string(),
                line: tokens.get(i).map(|t| t.position.line).unwrap_or(0),
                column: tokens.get(i).map(|t| t.position.column).unwrap_or(0),
            });
        }

        // Skip to matching brace
        let mut brace_count = 1;
        i += 1;
        while i < tokens.len() && brace_count > 0 {
            match &tokens[i].token_type {
                TokenType::LeftBrace => brace_count += 1,
                TokenType::RightBrace => brace_count -= 1,
                _ => {}
            }
            i += 1;
        }

        // Create function value
        let func = JsFunction {
            name: name.clone(),
            params,
            body: format!("{:?}", &tokens[body_start..i]), // Store token indices
            closure: HashMap::new(),
            is_async,
            is_generator: false,
            is_arrow: false,
        };

        let func_val = JsVal::Function(func);

        // Define function in current scope
        if !name.is_empty() {
            self.scopes[self.current_scope].define(&name, func_val.clone());
        }

        Ok((func_val, i - start))
    }

    /// Parse an expression
    fn parse_expression(
        &mut self,
        tokens: &[Token],
        start: usize,
    ) -> QuickJsResultType<(JsVal, usize)> {
        if start >= tokens.len() {
            return Ok((JsVal::Undefined, 0));
        }

        let token = &tokens[start];
        let mut i = start;

        let value = match &token.token_type {
            // Literals
            TokenType::Number(n) => {
                i += 1;
                JsVal::Number(*n)
            }
            TokenType::String(s) => {
                i += 1;
                JsVal::String(s.clone())
            }
            TokenType::Boolean(b) => {
                i += 1;
                JsVal::Boolean(*b)
            }
            TokenType::Null => {
                i += 1;
                JsVal::Null
            }
            TokenType::Undefined => {
                i += 1;
                JsVal::Undefined
            }

            // Identifier (variable reference)
            TokenType::Identifier(name) => {
                i += 1;
                self.get_variable(name).unwrap_or(JsVal::Undefined)
            }

            // Array literal
            TokenType::LeftBracket => {
                let (arr, consumed) = self.parse_array_literal(tokens, i)?;
                i += consumed;
                arr
            }

            // Object literal
            TokenType::LeftBrace => {
                let (obj, consumed) = self.parse_object_literal(tokens, i)?;
                i += consumed;
                obj
            }

            // Parenthesized expression
            TokenType::LeftParen => {
                i += 1;
                let (val, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                if matches!(
                    tokens.get(i).map(|t| &t.token_type),
                    Some(TokenType::RightParen)
                ) {
                    i += 1;
                }
                val
            }

            // Arrow function or function expression
            TokenType::Function => {
                let (func, consumed) = self.parse_function_declaration(tokens, i)?;
                i += consumed;
                func
            }

            // Unary operators
            TokenType::Bang => {
                i += 1;
                let (val, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                JsVal::Boolean(!val.is_truthy())
            }
            TokenType::Minus => {
                i += 1;
                let (val, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                JsVal::Number(-val.to_number())
            }
            TokenType::TypeOf => {
                i += 1;
                let (val, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                JsVal::String(val.type_of().to_string())
            }

            // Skip semicolons and other statement terminators
            TokenType::Semicolon
            | TokenType::RightBrace
            | TokenType::RightParen
            | TokenType::RightBracket
            | TokenType::Eof => {
                return Ok((JsVal::Undefined, 0));
            }

            _ => {
                // Skip unknown tokens
                i += 1;
                JsVal::Undefined
            }
        };

        // Check for binary operators
        let (binary_val, binary_consumed) = self.maybe_parse_binary_op(tokens, i, value)?;

        // Total consumed is from parse start to current position plus any binary op consumption
        let total_consumed = (i - start) + binary_consumed;
        Ok((binary_val, total_consumed))
    }

    /// Try to parse a binary operation
    /// Returns (value, additional_tokens_consumed)
    fn maybe_parse_binary_op(
        &mut self,
        tokens: &[Token],
        start: usize,
        left: JsVal,
    ) -> QuickJsResultType<(JsVal, usize)> {
        if start >= tokens.len() {
            return Ok((left, 0));
        }

        let token = &tokens[start];
        let mut i = start;

        match &token.token_type {
            TokenType::Plus => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                let result = match (&left, &right) {
                    (JsVal::String(a), _) => {
                        JsVal::String(format!("{}{}", a, right.to_js_string()))
                    }
                    (_, JsVal::String(b)) => JsVal::String(format!("{}{}", left.to_js_string(), b)),
                    _ => JsVal::Number(left.to_number() + right.to_number()),
                };
                Ok((result, i - start))
            }
            TokenType::Minus => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((
                    JsVal::Number(left.to_number() - right.to_number()),
                    i - start,
                ))
            }
            TokenType::Star => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((
                    JsVal::Number(left.to_number() * right.to_number()),
                    i - start,
                ))
            }
            TokenType::Slash => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((
                    JsVal::Number(left.to_number() / right.to_number()),
                    i - start,
                ))
            }
            TokenType::DoubleEquals | TokenType::TripleEquals => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                let equal = match (&left, &right) {
                    (JsVal::Number(a), JsVal::Number(b)) => (a - b).abs() < f64::EPSILON,
                    (JsVal::String(a), JsVal::String(b)) => a == b,
                    (JsVal::Boolean(a), JsVal::Boolean(b)) => a == b,
                    (JsVal::Null, JsVal::Null) => true,
                    (JsVal::Undefined, JsVal::Undefined) => true,
                    _ => false,
                };
                Ok((JsVal::Boolean(equal), i - start))
            }
            TokenType::BangEquals | TokenType::BangDoubleEquals => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                let equal = match (&left, &right) {
                    (JsVal::Number(a), JsVal::Number(b)) => (a - b).abs() < f64::EPSILON,
                    (JsVal::String(a), JsVal::String(b)) => a == b,
                    (JsVal::Boolean(a), JsVal::Boolean(b)) => a == b,
                    (JsVal::Null, JsVal::Null) => true,
                    (JsVal::Undefined, JsVal::Undefined) => true,
                    _ => false,
                };
                Ok((JsVal::Boolean(!equal), i - start))
            }
            TokenType::LessThan => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((
                    JsVal::Boolean(left.to_number() < right.to_number()),
                    i - start,
                ))
            }
            TokenType::GreaterThan => {
                i += 1;
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((
                    JsVal::Boolean(left.to_number() > right.to_number()),
                    i - start,
                ))
            }
            TokenType::DoubleAmpersand => {
                i += 1;
                if !left.is_truthy() {
                    return Ok((left, i - start));
                }
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((right, i - start))
            }
            TokenType::DoublePipe => {
                i += 1;
                if left.is_truthy() {
                    return Ok((left, i - start));
                }
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((right, i - start))
            }
            TokenType::NullishCoalesce => {
                i += 1;
                if !left.is_nullish() {
                    return Ok((left, i - start));
                }
                let (right, consumed) = self.parse_expression(tokens, i)?;
                i += consumed;
                Ok((right, i - start))
            }
            _ => Ok((left, 0)),
        }
    }

    /// Parse array literal
    fn parse_array_literal(
        &mut self,
        tokens: &[Token],
        start: usize,
    ) -> QuickJsResultType<(JsVal, usize)> {
        let mut i = start + 1; // Skip '['
        let mut elements = Vec::new();

        while i < tokens.len() && !matches!(tokens[i].token_type, TokenType::RightBracket) {
            if matches!(tokens[i].token_type, TokenType::Comma) {
                elements.push(JsVal::Undefined);
                i += 1;
                continue;
            }

            let (val, consumed) = self.parse_expression(tokens, i)?;
            elements.push(val);
            // Ensure we make progress
            if consumed == 0 {
                i += 1;
            } else {
                i += consumed;
            }

            if matches!(tokens.get(i).map(|t| &t.token_type), Some(TokenType::Comma)) {
                i += 1;
            }
        }

        // Skip ']'
        if matches!(
            tokens.get(i).map(|t| &t.token_type),
            Some(TokenType::RightBracket)
        ) {
            i += 1;
        }

        Ok((JsVal::Array(JsArray::from_elements(elements)), i - start))
    }

    /// Parse object literal
    fn parse_object_literal(
        &mut self,
        tokens: &[Token],
        start: usize,
    ) -> QuickJsResultType<(JsVal, usize)> {
        let mut i = start + 1; // Skip '{'
        let mut obj = JsObject::new();

        while i < tokens.len() && !matches!(tokens[i].token_type, TokenType::RightBrace) {
            // Get key
            let key = match &tokens[i].token_type {
                TokenType::Identifier(k) => k.clone(),
                TokenType::String(k) => k.clone(),
                _ => break,
            };
            i += 1;

            // Expect ':'
            if matches!(tokens.get(i).map(|t| &t.token_type), Some(TokenType::Colon)) {
                i += 1;
            } else {
                // Shorthand property
                obj.set(&key, self.get_variable(&key).unwrap_or(JsVal::Undefined));
                if matches!(tokens.get(i).map(|t| &t.token_type), Some(TokenType::Comma)) {
                    i += 1;
                }
                continue;
            }

            // Get value
            let (val, consumed) = self.parse_expression(tokens, i)?;
            obj.set(&key, val);
            // Ensure we make progress
            if consumed == 0 {
                i += 1;
            } else {
                i += consumed;
            }

            if matches!(tokens.get(i).map(|t| &t.token_type), Some(TokenType::Comma)) {
                i += 1;
            }
        }

        // Skip '}'
        if matches!(
            tokens.get(i).map(|t| &t.token_type),
            Some(TokenType::RightBrace)
        ) {
            i += 1;
        }

        Ok((JsVal::Object(obj), i - start))
    }

    /// Get a variable from the scope chain
    fn get_variable(&self, name: &str) -> Option<JsVal> {
        let mut scope_idx = self.current_scope;
        loop {
            if let Some(val) = self.scopes[scope_idx].get_own(name) {
                return Some(val.clone());
            }
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                break;
            }
        }
        None
    }

    /// Call a function
    fn call_function(&mut self, func: &JsVal, args: &[JsVal]) -> QuickJsResultType<JsVal> {
        self.stats.function_calls += 1;

        match func {
            JsVal::Function(f) => {
                // Create new function scope
                let parent = self.current_scope;
                self.scopes.push(Scope::function(parent));
                let func_scope = self.scopes.len() - 1;
                self.current_scope = func_scope;

                // Bind parameters
                for (i, param) in f.params.iter().enumerate() {
                    let value = args.get(i).cloned().unwrap_or(JsVal::Undefined);
                    self.scopes[func_scope].define(param, value);
                }

                // For now, return the first argument (simplified)
                let result = args.first().cloned().unwrap_or(JsVal::Undefined);

                // Restore scope
                self.current_scope = parent;
                self.scopes.pop();

                Ok(result)
            }
            _ => Err(QuickJsError::TypeError {
                message: "Not a function".to_string(),
            }),
        }
    }

    /// Check execution limits
    fn check_limits(&self) -> QuickJsResultType<()> {
        // Check timeout
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_millis() as u32;
            if elapsed > self.config.timeout_ms {
                return Err(QuickJsError::Timeout {
                    elapsed_ms: elapsed,
                });
            }
        }

        // Check memory
        if self.stats.memory_used > self.config.memory_limit {
            return Err(QuickJsError::OutOfMemory {
                used: self.stats.memory_used,
                limit: self.config.memory_limit,
            });
        }

        Ok(())
    }

    /// Log to console
    pub fn console_log(&mut self, args: &[JsVal]) {
        self.console.log(args);
    }

    /// Get console logs
    pub fn get_console_logs(&self) -> &[ConsoleLog] {
        self.console.get_logs()
    }

    /// Parse JSON
    pub fn json_parse(&self, text: &str) -> QuickJsResultType<JsVal> {
        self.json
            .parse(text)
            .map_err(|msg| QuickJsError::SyntaxError {
                message: msg,
                line: 0,
                column: 0,
            })
    }

    /// Stringify to JSON
    pub fn json_stringify(&self, value: &JsVal) -> QuickJsResultType<String> {
        self.json
            .stringify(value)
            .map_err(|msg| QuickJsError::TypeError { message: msg })
    }

    /// Set timeout
    pub fn set_timeout(&mut self, callback: JsVal, delay_ms: u32) -> u32 {
        self.timers.set_timeout(callback, delay_ms).0
    }

    /// Clear timeout
    pub fn clear_timeout(&mut self, id: u32) {
        self.timers.clear_timer(super::builtin::TimerId(id));
    }
}

impl Default for QuickJsEngine {
    fn default() -> Self {
        Self::new(QuickJsConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = QuickJsEngine::new(QuickJsConfig::default());
        assert!(engine.get_variable("undefined").is_some());
        assert!(engine.get_variable("console").is_some());
        assert!(engine.get_variable("JSON").is_some());
    }

    #[test]
    fn test_execute_literals() {
        let mut engine = QuickJsEngine::new(QuickJsConfig::default());

        let result = engine.execute("42", None, None).unwrap();
        assert_eq!(result.value, serde_json::json!(42.0));

        let result = engine.execute("\"hello\"", None, None).unwrap();
        assert_eq!(result.value, serde_json::json!("hello"));

        let result = engine.execute("true", None, None).unwrap();
        assert_eq!(result.value, serde_json::json!(true));
    }

    #[test]
    fn test_execute_variable_declaration() {
        let mut engine = QuickJsEngine::new(QuickJsConfig::default());

        let result = engine.execute("const x = 42", None, None).unwrap();
        assert!(result.success);

        // Variable should be accessible
        assert!(engine.get_variable("x").is_some());
    }

    #[test]
    fn test_execute_arithmetic() {
        let mut engine = QuickJsEngine::new(QuickJsConfig::default());

        let result = engine.execute("2 + 3", None, None).unwrap();
        assert_eq!(result.value, serde_json::json!(5.0));
    }

    #[test]
    fn test_json_operations() {
        let engine = QuickJsEngine::new(QuickJsConfig::default());

        let parsed = engine.json_parse(r#"{"name": "test"}"#).unwrap();
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.get("name").and_then(|v| v.as_string()), Some("test"));

        let stringified = engine.json_stringify(&parsed).unwrap();
        assert!(stringified.contains("\"name\""));
    }

    #[test]
    fn test_array_literal() {
        let mut engine = QuickJsEngine::new(QuickJsConfig::default());

        let result = engine.execute("[1, 2, 3]", None, None).unwrap();
        assert_eq!(result.value, serde_json::json!([1.0, 2.0, 3.0]));
    }

    #[test]
    fn test_object_literal() {
        let mut engine = QuickJsEngine::new(QuickJsConfig::default());

        let result = engine.execute(r#"{"x": 1, "y": 2}"#, None, None).unwrap();
        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.get("x").unwrap(), &serde_json::json!(1.0));
        assert_eq!(obj.get("y").unwrap(), &serde_json::json!(2.0));
    }

    #[test]
    fn test_timeout_check() {
        let config = QuickJsConfig {
            timeout_ms: 1, // Very short timeout
            ..Default::default()
        };
        let mut engine = QuickJsEngine::new(config);
        engine.start_time = Some(web_time::Instant::now());

        // Wait a bit and check limits
        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = engine.check_limits();
        assert!(matches!(result, Err(QuickJsError::Timeout { .. })));
    }
}
