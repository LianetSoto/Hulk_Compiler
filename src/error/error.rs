use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum CompilerError {
    #[error("Unexpected character  '{ch}'")]
    UnexpectedCharacter { ch: char},


}