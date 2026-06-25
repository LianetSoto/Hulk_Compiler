use std::fmt::{self, write};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {

    // Keywords
    Print,
    Function,
    Let,
    In,
    If,
    Else,
    Elif,
    While,
    For,
    True,
    False,

    // 
    LParen,
    RParen,
    LBrace,
    RBrace,

    // Operators 
    Plus,
    Minus,
    Mult,
    Div,
    Power,
    Dot,
    Comma,
    Colon,
    Semicolon,
    Concat,
    ConcatSpace,
    Percent,

    // Assignment and comparison
    Eq, 
    Assign,
    Arrow,
    EqEq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
    And,
    Or,
    Not,

    //protocols
    Protocol,
    Extends,

    RArrow,

    // Constants
    Pi,
    E,

    Is,
    As,

    T,

    // Arithmetic built‑in functions
    Sin,
    Cos,
    Tan,
    Sqrt,
    Log,
    Exp,
    Rand,

    // Types and OOP
    Type,
    Inherits,
    New,
    NumberType,   // "Number"
    StringType,   // "String"
    BooleanType,  // "Boolean"
    ObjectType,   // "Object"

     // Identifiers
    Identifier(String),    

    // Regular expression patterns
    Number(f64),

    // String literals
    Str(String),

    Error,
}

// Implementación de Display para errores bonitos
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Print => write!(f, "print"),
            Token::Function => write!(f, "function"),
            Token::Let => write!(f, "let"),
            Token::In => write!(f, "in"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Elif => write!(f, "elif"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Mult => write!(f, "*"),
            Token::Div => write!(f, "/"),
            Token::Power => write!(f, "^"),
            Token::Percent => write!(f,"%"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Concat => write!(f, "@"),
            Token::ConcatSpace => write!(f, "@@"),
            Token::Eq => write!(f, "="),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Assign => write!(f, ":="),
            Token::Arrow => write!(f, "=>"),
            Token::EqEq => write!(f, "=="),
            Token::Neq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::Leq => write!(f, "<="),
            Token::Geq => write!(f, ">="),
            Token::And => write!(f, "&"),
            Token::Or => write!(f, "|"),
            Token::Not => write!(f, "!"),
            Token::Pi => write!(f, "PI"),
            Token::E  => write!(f, "E"),
            Token::Sin => write!(f, "sin"),
            Token::Cos => write!(f, "cos"),
            Token::Tan => write!(f, "tan"),
            Token::Sqrt => write!(f, "sqrt"),
            Token::Log => write!(f, "log"),
            Token::Exp => write!(f, "exp"),
            Token::Rand => write!(f, "rand"),
            Token::Number(v) => write!(f, "{}", v),
            Token::Str(s) => write!(f, "\"{}\"", s),
            Token::Type => write!(f, "type"),
            Token::Inherits => write!(f, "inherits"),
            Token::New => write!(f, "new"),
            Token::NumberType => write!(f, "Number"),
            Token::StringType => write!(f, "String"),
            Token::BooleanType => write!(f, "Boolean"),
            Token::ObjectType => write!(f, "Object"),
            Token::Range => write!(f, "range"),
            Token::Protocol => write!(f, "protocol"),
            Token::Extends => write!(f, "extends"),
            Token::Error =>  write!(f, "error"),
            Token::RArrow=>  write!(f, "->"),
            Token::Is => write!(f, "is"),
            Token::As => write!(f, "as"),
            Token::T => write!(f, "T"),
        }
    }
}
