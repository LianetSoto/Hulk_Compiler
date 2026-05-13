use crate::ast::{Node, Visitor, FunctionDef, Expr};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<FunctionDef>,  // Funciones primero
    pub main_expr: Box<Expr>,         // Luego una sola expresión principal
    pub span: Span,
}

impl Node for Program {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_program(self)
    }
}