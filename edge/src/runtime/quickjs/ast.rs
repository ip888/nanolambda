//! JavaScript AST and Parser
//!
//! This module provides a simplified JavaScript parser for the QuickJS runtime.
//! It supports a subset of ES2023 sufficient for serverless function execution.
//!
//! # Supported Syntax
//!
//! - Variable declarations (const, let, var)
//! - Function declarations and expressions
//! - Arrow functions
//! - Async/await
//! - Object and array literals
//! - Operators (arithmetic, comparison, logical)
//! - Control flow (if/else, for, while, return)
//! - Template literals
//! - Destructuring assignment
//! - Spread operator

use super::QuickJsError;

/// Source position
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub offset: usize,
}

/// Token types
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    Identifier(String),
    
    // Keywords
    Const,
    Let,
    Var,
    Function,
    Async,
    Await,
    Return,
    If,
    Else,
    For,
    While,
    Do,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    Try,
    Catch,
    Finally,
    Throw,
    New,
    This,
    Class,
    Extends,
    Static,
    Import,
    Export,
    From,
    Of,
    In,
    TypeOf,
    InstanceOf,
    Delete,
    Void,
    Yield,
    
    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,
    QuestionMark,
    OptionalChain,  // ?.
    NullishCoalesce, // ??
    Spread,         // ...
    Arrow,          // =>
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,       // **
    PlusPlus,
    MinusMinus,
    Equals,
    PlusEquals,
    MinusEquals,
    StarEquals,
    SlashEquals,
    DoubleEquals,
    TripleEquals,
    BangEquals,
    BangDoubleEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
    Ampersand,
    DoubleAmpersand,
    Pipe,
    DoublePipe,
    Caret,
    Tilde,
    Bang,
    LeftShift,
    RightShift,
    UnsignedRightShift,
    
    // Template literals
    TemplateHead(String),
    TemplateMiddle(String),
    TemplateTail(String),
    TemplateNoSubstitution(String),
    
    // End of file
    Eof,
}

/// Token with position
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub position: Position,
}

/// AST Node types
#[derive(Debug, Clone)]
pub enum AstNode {
    // Program
    Program(Vec<AstNode>),
    
    // Declarations
    VariableDeclaration {
        kind: VariableKind,
        declarations: Vec<VariableDeclarator>,
    },
    FunctionDeclaration {
        name: String,
        params: Vec<Pattern>,
        body: Box<AstNode>,
        is_async: bool,
        is_generator: bool,
    },
    ClassDeclaration {
        name: String,
        super_class: Option<Box<AstNode>>,
        body: Vec<AstNode>,
    },
    
    // Expressions
    Literal(LiteralValue),
    Identifier(String),
    BinaryExpression {
        operator: BinaryOperator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    UnaryExpression {
        operator: UnaryOperator,
        argument: Box<AstNode>,
        prefix: bool,
    },
    UpdateExpression {
        operator: UpdateOperator,
        argument: Box<AstNode>,
        prefix: bool,
    },
    AssignmentExpression {
        operator: AssignmentOperator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    LogicalExpression {
        operator: LogicalOperator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    ConditionalExpression {
        test: Box<AstNode>,
        consequent: Box<AstNode>,
        alternate: Box<AstNode>,
    },
    CallExpression {
        callee: Box<AstNode>,
        arguments: Vec<AstNode>,
    },
    MemberExpression {
        object: Box<AstNode>,
        property: Box<AstNode>,
        computed: bool,
        optional: bool,
    },
    ArrayExpression(Vec<Option<AstNode>>),
    ObjectExpression(Vec<Property>),
    FunctionExpression {
        name: Option<String>,
        params: Vec<Pattern>,
        body: Box<AstNode>,
        is_async: bool,
        is_generator: bool,
    },
    ArrowFunctionExpression {
        params: Vec<Pattern>,
        body: Box<AstNode>,
        is_async: bool,
    },
    TemplateLiteral {
        quasis: Vec<String>,
        expressions: Vec<AstNode>,
    },
    NewExpression {
        callee: Box<AstNode>,
        arguments: Vec<AstNode>,
    },
    ThisExpression,
    SpreadElement(Box<AstNode>),
    AwaitExpression(Box<AstNode>),
    YieldExpression {
        argument: Option<Box<AstNode>>,
        delegate: bool,
    },
    
    // Statements
    BlockStatement(Vec<AstNode>),
    ExpressionStatement(Box<AstNode>),
    ReturnStatement(Option<Box<AstNode>>),
    IfStatement {
        test: Box<AstNode>,
        consequent: Box<AstNode>,
        alternate: Option<Box<AstNode>>,
    },
    ForStatement {
        init: Option<Box<AstNode>>,
        test: Option<Box<AstNode>>,
        update: Option<Box<AstNode>>,
        body: Box<AstNode>,
    },
    ForInStatement {
        left: Box<AstNode>,
        right: Box<AstNode>,
        body: Box<AstNode>,
    },
    ForOfStatement {
        left: Box<AstNode>,
        right: Box<AstNode>,
        body: Box<AstNode>,
        is_await: bool,
    },
    WhileStatement {
        test: Box<AstNode>,
        body: Box<AstNode>,
    },
    DoWhileStatement {
        body: Box<AstNode>,
        test: Box<AstNode>,
    },
    SwitchStatement {
        discriminant: Box<AstNode>,
        cases: Vec<SwitchCase>,
    },
    TryStatement {
        block: Box<AstNode>,
        handler: Option<CatchClause>,
        finalizer: Option<Box<AstNode>>,
    },
    ThrowStatement(Box<AstNode>),
    BreakStatement(Option<String>),
    ContinueStatement(Option<String>),
    EmptyStatement,
}

/// Variable declaration kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    Var,
    Let,
    Const,
}

/// Variable declarator
#[derive(Debug, Clone)]
pub struct VariableDeclarator {
    pub id: Pattern,
    pub init: Option<Box<AstNode>>,
}

/// Pattern for destructuring
#[derive(Debug, Clone)]
pub enum Pattern {
    Identifier(String),
    ArrayPattern(Vec<Option<Pattern>>),
    ObjectPattern(Vec<PropertyPattern>),
    AssignmentPattern {
        left: Box<Pattern>,
        right: Box<AstNode>,
    },
    RestElement(Box<Pattern>),
}

/// Property pattern for object destructuring
#[derive(Debug, Clone)]
pub struct PropertyPattern {
    pub key: String,
    pub value: Pattern,
    pub shorthand: bool,
}

/// Object property
#[derive(Debug, Clone)]
pub struct Property {
    pub key: PropertyKey,
    pub value: AstNode,
    pub kind: PropertyKind,
    pub shorthand: bool,
    pub computed: bool,
}

/// Property key
#[derive(Debug, Clone)]
pub enum PropertyKey {
    Identifier(String),
    Literal(LiteralValue),
    Computed(Box<AstNode>),
}

/// Property kind
#[derive(Debug, Clone, Copy)]
pub enum PropertyKind {
    Init,
    Get,
    Set,
}

/// Literal values
#[derive(Debug, Clone)]
pub enum LiteralValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    RegExp { pattern: String, flags: String },
}

/// Binary operators
#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div, Mod, Exp,
    Equal, NotEqual, StrictEqual, StrictNotEqual,
    LessThan, LessThanOrEqual, GreaterThan, GreaterThanOrEqual,
    BitwiseAnd, BitwiseOr, BitwiseXor,
    LeftShift, RightShift, UnsignedRightShift,
    In, InstanceOf,
}

/// Unary operators
#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    Minus, Plus, Not, BitwiseNot, TypeOf, Void, Delete,
}

/// Update operators
#[derive(Debug, Clone, Copy)]
pub enum UpdateOperator {
    Increment, Decrement,
}

/// Assignment operators
#[derive(Debug, Clone, Copy)]
pub enum AssignmentOperator {
    Assign, AddAssign, SubAssign, MulAssign, DivAssign, ModAssign,
    AndAssign, OrAssign, XorAssign, LeftShiftAssign, RightShiftAssign,
    NullishAssign, AndAssignLogical, OrAssignLogical,
}

/// Logical operators
#[derive(Debug, Clone, Copy)]
pub enum LogicalOperator {
    And, Or, NullishCoalesce,
}

/// Switch case
#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub test: Option<AstNode>,
    pub consequent: Vec<AstNode>,
}

/// Catch clause
#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param: Option<Pattern>,
    pub body: Box<AstNode>,
}

/// Simple tokenizer for JavaScript
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    position: Position,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            position: Position { line: 1, column: 1, offset: 0 },
        }
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Result<Token, QuickJsError> {
        self.skip_whitespace_and_comments();

        let start_pos = self.position;

        let Some((offset, ch)) = self.chars.next() else {
            return Ok(Token {
                token_type: TokenType::Eof,
                position: start_pos,
            });
        };

        self.position.offset = offset;
        self.position.column += 1;

        let token_type = match ch {
            // Single-character tokens
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            '[' => TokenType::LeftBracket,
            ']' => TokenType::RightBracket,
            ',' => TokenType::Comma,
            ';' => TokenType::Semicolon,
            ':' => TokenType::Colon,
            '~' => TokenType::Tilde,

            // Multi-character tokens
            '.' => self.scan_dot(),
            '+' => self.scan_plus(),
            '-' => self.scan_minus(),
            '*' => self.scan_star(),
            '/' => self.scan_slash(),
            '%' => self.scan_percent(),
            '=' => self.scan_equals(),
            '!' => self.scan_bang(),
            '<' => self.scan_less(),
            '>' => self.scan_greater(),
            '&' => self.scan_ampersand(),
            '|' => self.scan_pipe(),
            '^' => self.scan_caret(),
            '?' => self.scan_question(),

            // Strings
            '"' | '\'' => self.scan_string(ch)?,

            // Template literals
            '`' => self.scan_template()?,

            // Numbers
            '0'..='9' => self.scan_number(ch)?,

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' | '$' => self.scan_identifier(ch),

            _ => {
                return Err(QuickJsError::SyntaxError {
                    message: format!("Unexpected character: {}", ch),
                    line: start_pos.line,
                    column: start_pos.column,
                });
            }
        };

        Ok(Token {
            token_type,
            position: start_pos,
        })
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_whitespace() {
                self.chars.next();
                if ch == '\n' {
                    self.position.line += 1;
                    self.position.column = 1;
                } else {
                    self.position.column += 1;
                }
            } else if ch == '/' {
                // Peek ahead for comments
                let mut temp = self.chars.clone();
                temp.next();
                if let Some(&(_, next)) = temp.peek() {
                    if next == '/' {
                        // Line comment
                        self.chars.next(); // consume first /
                        self.chars.next(); // consume second /
                        while let Some(&(_, c)) = self.chars.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.chars.next();
                        }
                    } else if next == '*' {
                        // Block comment
                        self.chars.next();
                        self.chars.next();
                        while let Some((_, c)) = self.chars.next() {
                            if c == '\n' {
                                self.position.line += 1;
                                self.position.column = 1;
                            }
                            if c == '*' {
                                if let Some(&(_, '/')) = self.chars.peek() {
                                    self.chars.next();
                                    break;
                                }
                            }
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn scan_identifier(&mut self, first: char) -> TokenType {
        let mut name = String::new();
        name.push(first);

        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                name.push(ch);
                self.chars.next();
                self.position.column += 1;
            } else {
                break;
            }
        }

        // Check for keywords
        match name.as_str() {
            "const" => TokenType::Const,
            "let" => TokenType::Let,
            "var" => TokenType::Var,
            "function" => TokenType::Function,
            "async" => TokenType::Async,
            "await" => TokenType::Await,
            "return" => TokenType::Return,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "for" => TokenType::For,
            "while" => TokenType::While,
            "do" => TokenType::Do,
            "break" => TokenType::Break,
            "continue" => TokenType::Continue,
            "switch" => TokenType::Switch,
            "case" => TokenType::Case,
            "default" => TokenType::Default,
            "try" => TokenType::Try,
            "catch" => TokenType::Catch,
            "finally" => TokenType::Finally,
            "throw" => TokenType::Throw,
            "new" => TokenType::New,
            "this" => TokenType::This,
            "class" => TokenType::Class,
            "extends" => TokenType::Extends,
            "static" => TokenType::Static,
            "import" => TokenType::Import,
            "export" => TokenType::Export,
            "from" => TokenType::From,
            "of" => TokenType::Of,
            "in" => TokenType::In,
            "typeof" => TokenType::TypeOf,
            "instanceof" => TokenType::InstanceOf,
            "delete" => TokenType::Delete,
            "void" => TokenType::Void,
            "yield" => TokenType::Yield,
            "true" => TokenType::Boolean(true),
            "false" => TokenType::Boolean(false),
            "null" => TokenType::Null,
            "undefined" => TokenType::Undefined,
            _ => TokenType::Identifier(name),
        }
    }

    fn scan_number(&mut self, first: char) -> Result<TokenType, QuickJsError> {
        let mut num_str = String::new();
        num_str.push(first);

        // Handle hex, octal, binary
        if first == '0' {
            if let Some(&(_, ch)) = self.chars.peek() {
                if ch == 'x' || ch == 'X' {
                    num_str.push(ch);
                    self.chars.next();
                    while let Some(&(_, c)) = self.chars.peek() {
                        if c.is_ascii_hexdigit() {
                            num_str.push(c);
                            self.chars.next();
                        } else {
                            break;
                        }
                    }
                    let value = i64::from_str_radix(&num_str[2..], 16).unwrap_or(0);
                    return Ok(TokenType::Number(value as f64));
                }
            }
        }

        // Regular decimal number
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.chars.next();
            } else if ch == '.' {
                // Check if it's a decimal point
                let mut temp = self.chars.clone();
                temp.next();
                if let Some(&(_, next)) = temp.peek() {
                    if next.is_ascii_digit() {
                        num_str.push(ch);
                        self.chars.next();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else if ch == 'e' || ch == 'E' {
                num_str.push(ch);
                self.chars.next();
                if let Some(&(_, sign)) = self.chars.peek() {
                    if sign == '+' || sign == '-' {
                        num_str.push(sign);
                        self.chars.next();
                    }
                }
            } else {
                break;
            }
        }

        let value: f64 = num_str.parse().unwrap_or(0.0);
        Ok(TokenType::Number(value))
    }

    fn scan_string(&mut self, quote: char) -> Result<TokenType, QuickJsError> {
        let mut value = String::new();

        loop {
            match self.chars.next() {
                Some((_, ch)) if ch == quote => break,
                Some((_, '\\')) => {
                    // Escape sequence
                    match self.chars.next() {
                        Some((_, 'n')) => value.push('\n'),
                        Some((_, 'r')) => value.push('\r'),
                        Some((_, 't')) => value.push('\t'),
                        Some((_, '\\')) => value.push('\\'),
                        Some((_, '"')) => value.push('"'),
                        Some((_, '\'')) => value.push('\''),
                        Some((_, '0')) => value.push('\0'),
                        Some((_, c)) => value.push(c),
                        None => {
                            return Err(QuickJsError::SyntaxError {
                                message: "Unterminated string".to_string(),
                                line: self.position.line,
                                column: self.position.column,
                            });
                        }
                    }
                }
                Some((_, '\n')) => {
                    return Err(QuickJsError::SyntaxError {
                        message: "Unterminated string".to_string(),
                        line: self.position.line,
                        column: self.position.column,
                    });
                }
                Some((_, ch)) => value.push(ch),
                None => {
                    return Err(QuickJsError::SyntaxError {
                        message: "Unterminated string".to_string(),
                        line: self.position.line,
                        column: self.position.column,
                    });
                }
            }
        }

        Ok(TokenType::String(value))
    }

    fn scan_template(&mut self) -> Result<TokenType, QuickJsError> {
        let mut value = String::new();

        loop {
            match self.chars.next() {
                Some((_, '`')) => {
                    return Ok(TokenType::TemplateNoSubstitution(value));
                }
                Some((_, '$')) => {
                    if let Some(&(_, '{')) = self.chars.peek() {
                        self.chars.next();
                        return Ok(TokenType::TemplateHead(value));
                    } else {
                        value.push('$');
                    }
                }
                Some((_, '\\')) => {
                    if let Some((_, ch)) = self.chars.next() {
                        match ch {
                            'n' => value.push('\n'),
                            'r' => value.push('\r'),
                            't' => value.push('\t'),
                            '\\' => value.push('\\'),
                            '`' => value.push('`'),
                            '$' => value.push('$'),
                            c => value.push(c),
                        }
                    }
                }
                Some((_, ch)) => value.push(ch),
                None => {
                    return Err(QuickJsError::SyntaxError {
                        message: "Unterminated template literal".to_string(),
                        line: self.position.line,
                        column: self.position.column,
                    });
                }
            }
        }
    }

    // Helper methods for multi-character tokens
    fn scan_dot(&mut self) -> TokenType {
        if let Some(&(_, '.')) = self.chars.peek() {
            self.chars.next();
            if let Some(&(_, '.')) = self.chars.peek() {
                self.chars.next();
                return TokenType::Spread;
            }
        }
        TokenType::Dot
    }

    fn scan_plus(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '+')) => { self.chars.next(); TokenType::PlusPlus }
            Some(&(_, '=')) => { self.chars.next(); TokenType::PlusEquals }
            _ => TokenType::Plus,
        }
    }

    fn scan_minus(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '-')) => { self.chars.next(); TokenType::MinusMinus }
            Some(&(_, '=')) => { self.chars.next(); TokenType::MinusEquals }
            _ => TokenType::Minus,
        }
    }

    fn scan_star(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '*')) => { self.chars.next(); TokenType::StarStar }
            Some(&(_, '=')) => { self.chars.next(); TokenType::StarEquals }
            _ => TokenType::Star,
        }
    }

    fn scan_slash(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '=')) => { self.chars.next(); TokenType::SlashEquals }
            _ => TokenType::Slash,
        }
    }

    fn scan_percent(&mut self) -> TokenType {
        TokenType::Percent
    }

    fn scan_equals(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '=')) => {
                self.chars.next();
                if let Some(&(_, '=')) = self.chars.peek() {
                    self.chars.next();
                    TokenType::TripleEquals
                } else {
                    TokenType::DoubleEquals
                }
            }
            Some(&(_, '>')) => { self.chars.next(); TokenType::Arrow }
            _ => TokenType::Equals,
        }
    }

    fn scan_bang(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '=')) => {
                self.chars.next();
                if let Some(&(_, '=')) = self.chars.peek() {
                    self.chars.next();
                    TokenType::BangDoubleEquals
                } else {
                    TokenType::BangEquals
                }
            }
            _ => TokenType::Bang,
        }
    }

    fn scan_less(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '=')) => { self.chars.next(); TokenType::LessThanEquals }
            Some(&(_, '<')) => { self.chars.next(); TokenType::LeftShift }
            _ => TokenType::LessThan,
        }
    }

    fn scan_greater(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '=')) => { self.chars.next(); TokenType::GreaterThanEquals }
            Some(&(_, '>')) => {
                self.chars.next();
                if let Some(&(_, '>')) = self.chars.peek() {
                    self.chars.next();
                    TokenType::UnsignedRightShift
                } else {
                    TokenType::RightShift
                }
            }
            _ => TokenType::GreaterThan,
        }
    }

    fn scan_ampersand(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '&')) => { self.chars.next(); TokenType::DoubleAmpersand }
            _ => TokenType::Ampersand,
        }
    }

    fn scan_pipe(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '|')) => { self.chars.next(); TokenType::DoublePipe }
            _ => TokenType::Pipe,
        }
    }

    fn scan_caret(&mut self) -> TokenType {
        TokenType::Caret
    }

    fn scan_question(&mut self) -> TokenType {
        match self.chars.peek() {
            Some(&(_, '?')) => { self.chars.next(); TokenType::NullishCoalesce }
            Some(&(_, '.')) => { self.chars.next(); TokenType::OptionalChain }
            _ => TokenType::QuestionMark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_identifiers() {
        let mut lexer = Lexer::new("const foo let bar");
        
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Const));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Identifier(s) if s == "foo"));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Let));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Identifier(s) if s == "bar"));
    }

    #[test]
    fn test_lexer_numbers() {
        let mut lexer = Lexer::new("42 3.14 1e10 0xFF");
        
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Number(n) if (n - 42.0).abs() < 0.001));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Number(n) if (n - 3.14).abs() < 0.001));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Number(n) if (n - 1e10).abs() < 1.0));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Number(n) if (n - 255.0).abs() < 0.001));
    }

    #[test]
    fn test_lexer_strings() {
        let mut lexer = Lexer::new(r#""hello" 'world'"#);
        
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::String(s) if s == "hello"));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::String(s) if s == "world"));
    }

    #[test]
    fn test_lexer_operators() {
        let mut lexer = Lexer::new("+ ++ += - -- -= * ** *= / /= === !== => ?? ?.");
        
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Plus));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::PlusPlus));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::PlusEquals));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Minus));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::MinusMinus));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::MinusEquals));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Star));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::StarStar));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::StarEquals));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Slash));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::SlashEquals));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::TripleEquals));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::BangDoubleEquals));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Arrow));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::NullishCoalesce));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::OptionalChain));
    }

    #[test]
    fn test_lexer_comments() {
        let mut lexer = Lexer::new("foo // comment\nbar /* block */ baz");
        
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Identifier(s) if s == "foo"));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Identifier(s) if s == "bar"));
        assert!(matches!(lexer.next_token().unwrap().token_type, TokenType::Identifier(s) if s == "baz"));
    }
}
