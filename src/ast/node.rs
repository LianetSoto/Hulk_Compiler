use crate::ast::{Visitor};

pub trait Node {
    /// Acepta un visitor (patrón de diseño para recorrer el AST)
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result;
}

