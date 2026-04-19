use thiserror::Error;
use crate::error::SourceMap;
use crate::error::Span;

#[derive(Error, Debug, Clone)]
pub enum CompilerError {
    #[error("Unexpected character '{ch}'")]
    UnexpectedCharacter { ch: char, span: Span },

    #[error("Parser error: {msg}")]
    ParserError { msg: String, span: Option<Span> },
    // ... otros errores
}

impl CompilerError {
    pub fn span(&self) -> Option<Span> {
        match self {
            CompilerError::UnexpectedCharacter { span, .. } => Some(*span),
            CompilerError::ParserError { span, .. } => *span,
            // ... otros
            _ => None,
        }
    }
}

pub fn report_error(error: &CompilerError, source_map: &SourceMap) {
    if let Some(span) = error.span() {
        let (start_line, start_col, end_line, end_col) = source_map.span_to_line_col(span);

        // Cabecera del error
        eprintln!("Error: {}", error);
        eprintln!(
            " --> {}:{}:{}",
            start_line, start_col, end_col
        );

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