use crate::ast::{Node, Visitor, Expr};
use crate::error::Span;

// ENUM PRINCIPAL DE SENTENCIAS
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(ExprStmt),
    // Futuras: Let(LetStmt), If(IfStmt), While(WhileStmt), ...
}

// IMPLEMENTACIÓN DE Node PARA LOS ENUMS PRINCIPALES
impl Node for Stmt {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        match self {
            Stmt::Expr(s) => s.accept(visitor),
            // Stmt::Let(l) => l.accept(visitor), // cuando se añada
        }
    }
}

// EXPR STMT (sentencia que contiene una expresión)
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Box<Expr>,
    pub span: Span,
}

impl Node for ExprStmt {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_expr_stmt(self)
    }
}