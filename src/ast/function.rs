use crate::ast::{Expr, Node, Visitor};
use crate::error::Span;
use crate::semantic::types::HulkType;

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub name: String,
    pub ty: Option<HulkType>,   
    pub span: Span
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Params>,
    pub body: Box<Expr>,  // The body is always an expression (simple or block)
    pub span: Span,
    pub ty: Option<HulkType>,  // Return type
    pub is_generic: bool,
}

impl Node for FunctionDef {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_function_def(self)
    }
}