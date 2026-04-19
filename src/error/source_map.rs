use crate::error::Span;

pub struct SourceMap {
    source: String,
    line_starts: Vec<usize>,
}

impl SourceMap {
    pub fn new(source: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self { source, line_starts }
    }

    /// Dado un índice de byte, devuelve (línea, columna)
    pub fn byte_to_line_col(&self, pos: usize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&i| i <= pos);
        let line_start = self.line_starts[line - 1];
        let col = pos - line_start + 1;
        (line, col)
    }

    /// Convierte un Span a (línea_inicio, col_inicio, línea_fin, col_fin)
    pub fn span_to_line_col(&self, span: Span) -> (usize, usize, usize, usize) {
        let (start_line, start_col) = self.byte_to_line_col(span.start);
        let end_pos = if span.end > 0 { span.end - 1 } else { 0 };
        let (end_line, end_col) = self.byte_to_line_col(end_pos);
        (start_line, start_col, end_line, end_col + 1)
    }

    /// Obtiene la línea completa (sin salto de línea)
    pub fn get_line(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line - 1)?;
        let end = self.line_starts.get(line).copied().unwrap_or(self.source.len());
        Some(&self.source[start..end].trim_end_matches('\n'))
    }
}