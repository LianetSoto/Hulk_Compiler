use crate::ast::{Program, ExprStmt, NumberExpr, BinaryOpExpr, PrintExpr, StringExpr, CallExpr, ConstExpr};

pub trait Visitor {
    
    /// Tipo que devuelve el visitor (por ejemplo, `()` para PrettyPrinter, `Type` para TypeChecker).
    type Result;

    fn visit_program(&mut self, program: &Program) -> Self::Result;
    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Result;
    fn visit_number(&mut self, expr: &NumberExpr) -> Self::Result;
    fn visit_binary_op(&mut self, expr: &BinaryOpExpr) -> Self::Result;
    fn visit_print(&mut self, expr: &PrintExpr) -> Self::Result;
    fn visit_string(&mut self, expr: &StringExpr) -> Self::Result;
    fn visit_call(&mut self, expr: &CallExpr) -> Self::Result;
    fn visit_const(&mut self, expr: &ConstExpr) -> Self::Result;
}