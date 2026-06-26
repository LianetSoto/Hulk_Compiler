#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HulkType {
    Number,
    String,
    Boolean,
    Object,
    UserDefined(String), 
    Error,
    Var(usize),
    Class(String),
    Protocol(String),
    GenericPlaceholder,
    Iterable(Box<HulkType>), 
}

impl HulkType {
    /// Verifica si un tipo es compatible con otro (para asignaciones, argumentos, etc.)
    pub fn is_compatible_with(&self, other: &HulkType) -> bool {
        match (self, other) {
            (HulkType::Number, HulkType::Number) => true,
            (HulkType::String, HulkType::String) => true,
            (HulkType::Boolean, HulkType::Boolean) => true,
            (HulkType::Object, _) => true,
            (_, HulkType::Object) => true,
            (HulkType::Var(_), _) => true,
            (_, HulkType::Var(_)) => true,
            
            // En el futuro: Object y subtipado
            _ => false,
        }
    }

    pub fn get_var_id(&self) -> Option<usize> {
        if let HulkType::Var(id) = self { Some(*id) } else { None }
    }

}