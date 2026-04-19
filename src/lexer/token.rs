use logos::Logos;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Logos)]
#[logos(skip r"[ \t\n\f]+")]  // ignorar espacios, tabs, newlines y form feeds
pub enum Token {

    // Keywords
    #[token("print")] Print,
    #[token("function")] Function,
    #[token("let")] Let,
    #[token("in")] In,
    #[token("if")] If,
    #[token("else")] Else,
    #[token("elif")] Elif,
    #[token("while")] While,
    #[token("for")] For,
    #[token("true")] True,
    #[token("false")] False,

    // 
    #[token("(")]  LParen,
    #[token(")")]  RParen,
    #[token("{")]  LBrace,
    #[token("}")]  RBrace,

    // Operators 
    #[token("+")]  Plus,
    #[token("-")]  Minus,
    #[token("*")]  Mult,
    #[token("/")]  Div,
    #[token("^")]  Power,
    #[token(".")]  Dot,
    #[token(",")]  Comma,
    #[token(":")]  Colon,
    #[token(";")]  Semicolon,
    #[token("@")]  Concat,

    // revisar
    #[token(":=")] Assign,
    #[token("=>")] Arrow,
    #[token("==")] EqEq,
    #[token("!=")] Neq,
    #[token("<")]  Lt,
    #[token(">")]  Gt,
    #[token("<=")] Leq,
    #[token(">=")] Geq,
    #[token("&")]  And,
    #[token("|")]  Or,
    #[token("!")]  Not,

    // Constantes matemáticas
    #[token("PI")] Pi,
    #[token("E")]  E,

    // Funciones matemáticas built‑in
    #[token("sin")]  Sin,
    #[token("cos")]  Cos,
    #[token("sqrt")] Sqrt,
    #[token("rand")] Rand,
    #[token("exp")] Exp,
    #[token("log")] Log,

    // Regular expression patterns
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Number(f64),

    // Cadenas con escapes básicos
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        // Quitar las comillas dobles del principio y final
        let inner = &s[1..s.len()-1];
        let mut result = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some(c) => result.push(c),
                    None => result.push('\\'),
                }
            } else {
                result.push(c);
            }
        }
        result
    })]
    String(String),
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
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Concat => write!(f, "@"),
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
            Token::Sqrt => write!(f, "sqrt"),
            Token::Rand => write!(f, "rand"),
            Token::Log => write!(f,"log"),
            Token::Exp => write!(f,"exp"),
            Token::Number(v) => write!(f, "{}", v),
            Token::String(s) => write!(f, "\"{}\"", s),
        }
    }
}
