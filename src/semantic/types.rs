/// Representa los tipos del lenguaje Hulk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HulkType {
    Number,
    String,
    Boolean,
    Object,       // tipo base para objetos (por ahora)
    Error,        // para propagar errores sin seguir reportando
}

impl HulkType {
    /// Verifica si un tipo es compatible con otro (para asignaciones, argumentos, etc.)
    pub fn is_compatible_with(&self, other: &HulkType) -> bool {
        match (self, other) {
            // Por ahora solo Number es compatible consigo mismo
            (HulkType::Number, HulkType::Number) => true,
            (HulkType::String, HulkType::String) => true,
            (HulkType::Boolean, HulkType::Boolean) => true,
            // En el futuro: Object y subtipado
            _ => false,
        }
    }
}