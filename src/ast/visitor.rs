use crate::ast::{expr::AttributeAccessExpr, *};

pub trait Visitor {
    
    /// The type returned by the visitor (e.g., `()` for PrettyPrinter, `Type` for TypeChecker).
    type Result;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result;
    fn visit_function_def(&mut self, func: &mut FunctionDef) -> Self::Result;
    fn visit_number(&mut self, expr: &mut NumberExpr) -> Self::Result;
    fn visit_binary_op(&mut self, expr: &mut BinaryOpExpr) -> Self::Result;
    fn visit_string(&mut self, expr: &mut StringExpr) -> Self::Result;
    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result;
    fn visit_const(&mut self, expr: &mut ConstExpr) -> Self::Result;
    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result;
    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result;
    fn visit_variable(&mut self, expr: &mut VariableExpr) -> Self::Result;
    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result;
    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> Self::Result;
    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result;
    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result;
    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result;
    fn visit_type_def(&mut self, ty: &mut TypeDef) -> Self::Result;
    fn visit_attribute(&mut self, attr: &mut Attribute) -> Self::Result;
    fn visit_method(&mut self, m: &mut Method) -> Self::Result;
    fn visit_new(&mut self, e: &mut NewExpr) -> Self::Result;
    fn visit_method_call(&mut self, e: &mut MethodCallExpr) -> Self::Result;
    fn visit_self(&mut self, e: &mut SelfExpr) -> Self::Result;
    fn visit_base(&mut self, e: &mut BaseExpr) -> Self::Result;
    fn visit_attribute_access(&mut self, e: &mut AttributeAccessExpr) -> Self::Result;
    fn visit_protocol_def(&mut self, proto: &mut ProtocolDef) -> Self::Result;
    fn visit_protocol_method(&mut self, method: &mut ProtocolMethod) -> Self::Result;
    fn visit_base_call(&mut self, expr: &mut BaseCallExpr) -> Self::Result;
}