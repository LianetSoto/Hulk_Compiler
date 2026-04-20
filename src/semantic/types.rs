#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HulkType {
    Number,
    String,
    Boolean,
    Object,
    Error,
}

impl HulkType {
    /// Verifica si un tipo es compatible con otro (para asignaciones, argumentos, etc.)
    pub fn is_compatible_with(&self, other: &HulkType) -> bool {
        match (self, other) {
            (HulkType::Number, HulkType::Number) => true,
            (HulkType::String, HulkType::String) => true,
            (HulkType::Boolean, HulkType::Boolean) => true,
            // En el futuro: Object y subtipado
            _ => false,
        }
    }
}