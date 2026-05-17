use crate::ast::{Node, Visitor, FunctionDef, Expr,TypeDef};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub types: Vec<TypeDef>,
    pub functions: Vec<FunctionDef>,
    pub main_expr: Box<Expr>,
    pub span: Span,
}

impl Node for Program {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_program(self)
    }
}