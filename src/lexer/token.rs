use logos::Logos;

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
pub enum Token {

    // Keywords
    #[token("print")]  
    Print,

    // Operators 
    #[token("+")]
    Plus,

    // Regular expression patterns
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Number(f64),

}

pub fn tokenize(input: &str) -> Vec<Token> {
    Token::lexer(input)
        .filter_map(|result| result.ok())
        .collect()
}