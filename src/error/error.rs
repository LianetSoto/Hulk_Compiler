use thiserror::Error;
use crate::error::SourceMap;
use crate::error::Span;

#[derive(Error, Debug, Clone)]
pub enum CompilerError {
    #[error("Unexpected character '{ch}'")]
    UnexpectedCharacter { ch: char, span: Span },

    #[error("Parser error: {msg}")]
    ParserError { msg: String, span: Option<Span> },

    #[error("Type error: {msg}")]
    TypeError { msg: String, span: Span },

    #[error("Undefined variable '{name}'")]
    UndefinedVariable { name: String, span: Span },

    #[error("code generation error: {msg}")]
    CodegenError { msg: String, span: Option<Span> },

    #[error("I/O error: {0}")]
    IoError(String), 

    // ... otros errores
}

// Implementación manual de From<std::io::Error> para CompilerError
impl From<std::io::Error> for CompilerError {
    fn from(err: std::io::Error) -> Self {
        CompilerError::IoError(err.to_string())
    }
}

impl CompilerError {
    pub fn span(&self) -> Option<Span> {
        match self {
            CompilerError::UnexpectedCharacter { span, .. } => Some(*span),
            CompilerError::ParserError { span, .. } => *span,
            CompilerError::TypeError { span, .. } => Some(*span),
            CompilerError::UndefinedVariable { span, .. } => Some(*span),
            CompilerError::CodegenError { span, ..} => *span,    
            CompilerError::IoError(_) => None, 
            _ => None,
        }
    }
}

pub fn report_error(error: &CompilerError, source_map: &SourceMap, filename: &str) {
    if let Some(span) = error.span() {
        let (start_line, start_col, end_line, end_col) = source_map.span_to_line_col(span);

        // Cabecera del error
        eprintln!("Error: {}", error);
        eprintln!("--> {}:{}:{}", filename, start_line, start_col);
        eprintln!("");

        // Mostrar la línea de código
        if let Some(line_str) = source_map.get_line(start_line) {
            // Número de línea con ancho fijo (ej. 3 dígitos)
            eprintln!("{:3} |", "");
            eprintln!("{:3} | {}", start_line, line_str);
            
            // Padding para la flecha: los espacios hasta la columna, más la barra vertical
            let padding = " ".repeat(start_col - 1);
            eprintln!("{:3} | {}{}", "", padding, "^".repeat(end_col - start_col));
        }
    } else {
        eprintln!("Error: {}", error);
    }
}