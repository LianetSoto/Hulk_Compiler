use crate::ast::{Expr, FunctionDef, Node, ProtocolDef, TypeDef, Visitor};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub types: Vec<TypeDef>,
    pub protocols: Vec<ProtocolDef>,
    pub functions: Vec<FunctionDef>,
    pub main_expr: Box<Expr>,
    pub span: Span,
}

impl Node for Program {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_program(self)
    }
}