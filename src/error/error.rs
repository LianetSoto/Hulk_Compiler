use thiserror::Error;
use crate::error::SourceMap;
use crate::error::Span;

/// Central error type for the HULK compiler.
#[derive(Error, Debug, Clone)]
pub enum CompilerError {
    /// Lexical error (invalid character, malformed token, etc.).
    #[error("LEXICAL: {msg}")]
    LexerError { msg: String, span: Span },

    /// Syntactic error (unexpected token, missing delimiter, etc.).
    #[error("SYNTACTIC: {msg}")]
    ParserError { msg: String, span: Option<Span> },

    /// Semantic error (type mismatch, undefined variable, wrong arity, etc.).
    #[error("SEMANTIC: {msg}")]
    TypeError { msg: String, span: Span },

    /// Internal error during LLVM code generation.
    #[error("Code generation error: {msg}")]
    CodegenError { msg: String, span: Option<Span> },

    /// File system or subprocess error.
    #[error("I/O error: {0}")]
    IoError(String),

    /// Error during the monomorphization pass (generational type resolution).
    #[error("Monomorphization error: {msg}")]
    MonomorphizationError { msg: String, span: Span },
}

// Allow `std::io::Error` to be automatically converted into `CompilerError::IoError`.
impl From<std::io::Error> for CompilerError {
    fn from(err: std::io::Error) -> Self {
        CompilerError::IoError(err.to_string())
    }
}

impl CompilerError {
    /// Returns the source span of the error, if any.
    pub fn span(&self) -> Option<Span> {
        match self {
            CompilerError::LexerError { span, .. }         => Some(*span),
            CompilerError::ParserError { span, .. }        => *span,
            CompilerError::TypeError { span, .. }          => Some(*span),
            CompilerError::CodegenError { span, .. }       => *span,
            CompilerError::IoError(_)                      => None,
            CompilerError::MonomorphizationError { span, .. } => Some(*span),
        }
    }
}

/// Prints a single error in the CI‑required format:
///
/// ```text
/// (line,col) TYPE: message
/// ```
///
/// If no span is available, `(0,0)` is used as the position.
pub fn report_std_error(error: &CompilerError, source_map: &SourceMap) {
    let (line, col) = match error.span() {
        Some(span) => {
            let (l, c, _, _) = source_map.span_to_line_col(span);
            (l, c)
        }
        None => (0, 0),
    };

    eprintln!("({},{}) {}", line, col, error);
}

/// Prints a developer‑friendly error message that includes the source file,
/// the offending line, and a column underline.
pub fn report_error(error: &CompilerError, source_map: &SourceMap, filename: &str) {
    if let Some(span) = error.span() {
        let (start_line, start_col, end_line, end_col) =
            source_map.span_to_line_col(span);

        // Header
        eprintln!("{error}");
        eprintln!("--> {}:{}:{}", filename, start_line, start_col);
        eprintln!();

        // Source line
        if let Some(line_str) = source_map.get_line(start_line) {
            eprintln!("{:3} |", "");
            eprintln!("{:3} | {}", start_line, line_str);

            // Underline
            let padding = " ".repeat(start_col - 1);
            let width = (end_col - start_col).max(1);
            eprintln!("{:3} | {}{}", "", padding, "^".repeat(width));
        }
    } else {
        eprintln!("Error: {}", error);
    }
}