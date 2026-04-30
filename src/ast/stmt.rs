use crate::ast::{Node, Visitor, Expr};
use crate::error::Span;

// ENUM PRINCIPAL DE SENTENCIAS
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(ExprStmt),
}
#[derive(Debug, Clone, PartialEq)]
pub struct  ExprStmt{
    pub expr: Box<Expr>,
    pub span: Span,
}
// IMPLEMENTACIÓN DE Node PARA LOS ENUMS PRINCIPALES
impl Node for Stmt {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        match self {
            Stmt::Expr(s) => s.expr.accept(visitor),
        }
    }
}

