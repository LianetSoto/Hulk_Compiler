// src/semantic/type_checker.rs

use crate::ast::*;
use crate::error::CompilerError;
use super::types::HulkType;

pub struct TypeChecker {
    errors: Vec<CompilerError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    /// Ejecuta el type checking sobre el programa. Si hay errores, los devuelve.
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<CompilerError>> {
        program.accept(self);
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn add_error(&mut self, msg: String) {
        // TODO: agregar ubicación (línea/columna) cuando tengamos spans en el AST
        self.errors.push(CompilerError::ParserError { msg });
    }
}

impl Visitor for TypeChecker {
    type Result = HulkType;

    fn visit_program(&mut self, program: &Program) -> Self::Result {
        for stmt in &program.statements {
            stmt.accept(self);
        }
        // El programa en sí no tiene un tipo, pero devolvemos Number por convención
        HulkType::Number
    }

    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Result {
        // Una sentencia expresión se evalúa, pero su valor se ignora.
        // Aun así verificamos su tipo para detectar errores internos.
        stmt.expr.accept(self)
    }

    fn visit_number(&mut self, _expr: &NumberExpr) -> Self::Result {
        HulkType::Number
    }

    fn visit_binary_op(&mut self, expr: &BinaryOpExpr) -> Self::Result {
        let left_type = expr.left.accept(self);
        let right_type = expr.right.accept(self);

        // Verificar que ambos operandos sean Number
        if !left_type.is_compatible_with(&HulkType::Number) {
            self.add_error("Left operand of binary operator must be Number".to_string());
        }
        if !right_type.is_compatible_with(&HulkType::Number) {
            self.add_error("Right operand of binary operator must be Number".to_string());
        }

        // El resultado de cualquier operación aritmética es Number
        HulkType::Number
    }

    fn visit_print(&mut self, expr: &PrintExpr) -> Self::Result {
        let arg_type = expr.argument.accept(self);
        if !arg_type.is_compatible_with(&HulkType::Number) {
            self.add_error("print argument must be Number (for now)".to_string());
        }
        // Por simplicidad, decimos que devuelve Number.
        HulkType::Number
    }

    fn visit_string(&mut self, _expr: &StringExpr) -> HulkType {
        HulkType::String
    }

    fn visit_const(&mut self, expr: &ConstExpr) -> HulkType {
        // PI y E son números
        HulkType::Number
    }

    fn visit_call(&mut self, expr: &CallExpr) -> HulkType {
        match expr.func.as_str() {
            "sin" | "cos" | "sqrt" | "exp" => {
                if expr.args.len() != 1 {
                    self.add_error("Function takes 1 argument".to_string());
                } else {
                    let arg_ty = expr.args[0].accept(self);
                    if !arg_ty.is_compatible_with(&HulkType::Number) {
                        self.add_error("Argument must be Number".to_string());
                    }
                }
                HulkType::Number
            }
            "rand" => {
                if !expr.args.is_empty() {
                    self.add_error("rand takes 0 arguments".to_string());
                }
                HulkType::Number
            }
            "log" => {
                if expr.args.len() != 2 {
                    self.add_error("log expects 2 arguments (base, value)".to_string());
                } else {
                    let base_ty = expr.args[0].accept(self);
                    let val_ty = expr.args[1].accept(self);
                    if !base_ty.is_compatible_with(&HulkType::Number) ||
                    !val_ty.is_compatible_with(&HulkType::Number) {
                        self.add_error("log arguments must be Number".to_string());
                    }
                }
                HulkType::Number
            }
            _ => {
                self.add_error(format!("Unknown function '{}'", expr.func));
                HulkType::Error
            }
        }
    }
}