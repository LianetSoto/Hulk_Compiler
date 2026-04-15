use logos::Logos;

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Ignora este patrón entre tokens
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

    // Regular expression patterns
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Number(f64),

}

pub fn tokenize(input: &str) -> Vec<Token> {
    Token::lexer(input)
        .filter_map(|result| result.ok())
        .collect()
}