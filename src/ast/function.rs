use crate::ast::{Expr, Node, Visitor};
use crate::error::Span;
use crate::semantic::types::HulkType;

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,  // The body is always an expression (simple or block)
    pub span: Span,
    pub ty: Option<HulkType>,  // Return type
}

impl Node for FunctionDef {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_function_def(self)
    }
}