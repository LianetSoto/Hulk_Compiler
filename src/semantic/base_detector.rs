// src/semantic/base_detector.rs

use crate::ast::expr::UnaryOp;
use crate::ast::*;

/// Visitor para detectar si una expresión contiene `base()` en algún lugar.
pub struct BaseDetector {
    pub found: bool,
}

impl BaseDetector {
    pub fn new() -> Self {
        Self { found: false }
    }

    /// Resetea el estado del detector
    pub fn reset(&mut self) {
        self.found = false;
    }
}

impl Visitor for BaseDetector {
    type Result = ();

    fn visit_program(&mut self, _: &mut Program) -> Self::Result {}
    fn visit_function_def(&mut self, _: &mut FunctionDef) -> Self::Result {}
    fn visit_number(&mut self, _: &mut NumberExpr) -> Self::Result {}
    fn visit_string(&mut self, _: &mut StringExpr) -> Self::Result {}
    fn visit_const(&mut self, _: &mut ConstExpr) -> Self::Result {}
    fn visit_bool(&mut self, _: &mut BoolExpr) -> Self::Result {}
    fn visit_variable(&mut self, _: &mut VariableExpr) -> Self::Result {}
    fn visit_self(&mut self, _: &mut SelfExpr) -> Self::Result {}

    fn visit_binary_op(&mut self, expr: &mut BinaryOpExpr) -> Self::Result {
        expr.left.accept(self);
        expr.right.accept(self);
    }

    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result {
        for arg in &mut expr.args {
            arg.accept(self);
        }
    }

    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result {
        expr.expr.accept(self);
    }

    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result {
        for (_, _, init) in &mut expr.bindings {
            init.accept(self);
        }
        expr.body.accept(self);
    }

    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> Self::Result {
        expr.lhs.accept(self);
        expr.value.accept(self);
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {
        for e in &mut expr.expressions {
            e.accept(self);
        }
    }

    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        expr.condition.accept(self);
        expr.then_branch.accept(self);
        expr.else_branch.accept(self);
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        expr.condition.accept(self);
        expr.body.accept(self);
    }

    fn visit_new(&mut self, expr: &mut NewExpr) -> Self::Result {
        for arg in &mut expr.args {
            arg.accept(self);
        }
    }

    fn visit_method_call(&mut self, expr: &mut MethodCallExpr) -> Self::Result {
        // Detectar si el objeto es `base`
        if let Expr::Base(_) = &*expr.object {
            self.found = true;
        }
        expr.object.accept(self);
        for arg in &mut expr.args {
            arg.accept(self);
        }
    }

    fn visit_base(&mut self, _: &mut BaseExpr) -> Self::Result {
        self.found = true;
    }

    fn visit_attribute_access(&mut self, expr: &mut AttributeAccessExpr) -> Self::Result {
        expr.object.accept(self);
    }

    // Nodos de protocolo y otros que no contienen expresiones relevantes
    fn visit_type_def(&mut self, _: &mut TypeDef) -> Self::Result {}
    fn visit_attribute(&mut self, _: &mut Attribute) -> Self::Result {}
    fn visit_method(&mut self, _: &mut Method) -> Self::Result {}
    fn visit_protocol_def(&mut self, _: &mut ProtocolDef) -> Self::Result {}
    fn visit_protocol_method(&mut self, _: &mut ProtocolMethod) -> Self::Result {}
    fn visit_is(&mut self, _: &mut IsExpr) -> Self::Result {}
    fn visit_as(&mut self, _: &mut AsExpr) -> Self::Result {}
}