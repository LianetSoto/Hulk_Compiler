use crate::ast::{Program, ExprStmt, NumberExpr, BinaryOpExpr, PrintExpr, StringExpr, CallExpr, ConstExpr, BoolExpr, UnaryOpExpr};

pub trait Visitor {
    
    /// Tipo que devuelve el visitor (por ejemplo, `()` para PrettyPrinter, `Type` para TypeChecker).
    type Result;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result;
    fn visit_expr_stmt(&mut self, stmt: &mut ExprStmt) -> Self::Result;
    fn visit_number(&mut self, expr: &mut NumberExpr) -> Self::Result;
    fn visit_binary_op(&mut self, expr: &mut BinaryOpExpr) -> Self::Result;
    fn visit_print(&mut self, expr: &mut PrintExpr) -> Self::Result;
    fn visit_string(&mut self, expr: &mut StringExpr) -> Self::Result;
    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result;
    fn visit_const(&mut self, expr: &mut ConstExpr) -> Self::Result;
    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result;
    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result;
}