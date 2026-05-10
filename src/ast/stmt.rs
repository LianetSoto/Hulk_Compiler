use crate::ast::{Node, Visitor, Expr, FunctionDef};
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Function(FunctionDef),
    Expr(ExprStmt),
}
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Box<Expr>,
    pub span: Span,
}

//Implementation of Node for the main enums
impl Node for Stmt {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        match self {
            Stmt::Function(f) => f.accept(visitor),
            Stmt::Expr(s) => s.expr.accept(visitor),
        }
    }
}