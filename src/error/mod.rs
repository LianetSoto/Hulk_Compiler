use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum CompilerError {
    #[error("Unexpected character  '{ch}'")]
    UnexpectedCharacter { ch: char},
    #[error("Lexical error at {pos}: {msg}")]
    LexicalError { pos: usize, msg: String },
    #[error("Parser error: {msg}")]
    ParserError { msg: String },
}