use crate::ast::expr::UnaryOp;
use crate::ast::*;
use crate::error::CompilerError;
use super::types::HulkType;
use crate::error::Span;

pub struct TypeChecker {
    errors: Vec<CompilerError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    pub fn check(&mut self, program: &mut Program) -> Result<(), Vec<CompilerError>> {
        program.accept(self);
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn add_type_error(&mut self, msg: String, span: Span) {
        self.errors.push(CompilerError::TypeError { msg, span });
    }
}

impl Visitor for TypeChecker {
    type Result = HulkType;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result {
        for stmt in &mut program.statements {
            stmt.accept(self);
        }
        HulkType::Number
    }

    fn visit_expr_stmt(&mut self, stmt: &mut ExprStmt) -> Self::Result {
        stmt.expr.accept(self)
    }

    fn visit_number(&mut self, expr: &mut NumberExpr) -> Self::Result {
        let ty = HulkType::Number;
        expr.ty = Some(ty.clone());
        ty
    }

    fn visit_binary_op(&mut self, expr: &mut BinaryOpExpr) -> Self::Result {
        let left_type = expr.left.accept(self);
        let right_type = expr.right.accept(self);

        let result_ty = match expr.op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => {
                if !left_type.is_compatible_with(&HulkType::Number) {
                    self.add_type_error(
                        "Left operand of arithmetic operator must be Number".to_string(),
                        expr.left.span()
                    );
                }
                if !right_type.is_compatible_with(&HulkType::Number) {
                    self.add_type_error(
                        "Right operand of arithmetic operator must be Number".to_string(),
                        expr.right.span()
                    );
                }
                HulkType::Number
            }

            BinOp::Concat => {
                let left_ok = left_type.is_compatible_with(&HulkType::String) ||
                            left_type.is_compatible_with(&HulkType::Number);
                let right_ok = right_type.is_compatible_with(&HulkType::String) ||
                            right_type.is_compatible_with(&HulkType::Number);
                if !left_ok {
                    self.add_type_error(
                        "Left operand of @ must be String or Number".to_string(),
                        expr.left.span()
                    );
                }
                if !right_ok {
                    self.add_type_error(
                        "Right operand of @ must be String or Number".to_string(),
                        expr.right.span()
                    );
                }
                HulkType::String
            }

            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Leq | BinOp::Geq => {
                match expr.op {
                    BinOp::Eq | BinOp::Neq => {
                        if !left_type.is_compatible_with(&right_type) {
                            self.add_type_error(
                                format!("Cannot compare {:?} with {:?}", left_type, right_type),
                                expr.span
                            );
                        }
                    }
                    _ => {
                        // <, >, <=, >= just for numbers
                        if !left_type.is_compatible_with(&HulkType::Number) {
                            self.add_type_error(
                                "Left operand of comparison must be Number".to_string(),
                                expr.left.span()
                            );
                        }
                        if !right_type.is_compatible_with(&HulkType::Number) {
                            self.add_type_error(
                                "Right operand of comparison must be Number".to_string(),
                                expr.right.span()
                            );
                        }
                    }
                }
                HulkType::Boolean
            }

            BinOp::And | BinOp::Or => {
                if !left_type.is_compatible_with(&HulkType::Boolean) {
                    self.add_type_error(
                        "Left operand of logical operator must be Boolean".to_string(),
                        expr.left.span()
                    );
                }
                if !right_type.is_compatible_with(&HulkType::Boolean) {
                    self.add_type_error(
                        "Right operand of logical operator must be Boolean".to_string(),
                        expr.right.span()
                    );
                }
                HulkType::Boolean
            }
        };

        expr.ty = Some(result_ty.clone());
        result_ty
    }

    fn visit_print(&mut self, expr: &mut PrintExpr) -> Self::Result {
        let arg_type = expr.argument.accept(self);
        if !arg_type.is_compatible_with(&HulkType::Number) &&
           !arg_type.is_compatible_with(&HulkType::String) &&
           !arg_type.is_compatible_with(&HulkType::Boolean) {
            self.add_type_error(
                "print argument must be Number, String or Boolean".to_string(),
                expr.argument.span()
            );
        }
        let ty = HulkType::Number; // print return Number
        expr.ty = Some(ty.clone());
        ty
    }

    fn visit_string(&mut self, expr: &mut StringExpr) -> Self::Result {
        let ty = HulkType::String;
        expr.ty = Some(ty.clone());
        ty
    }

    fn visit_const(&mut self, expr: &mut ConstExpr) -> Self::Result {
        let ty = HulkType::Number;
        expr.ty = Some(ty.clone());
        ty
    }

    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result {
        let result_ty = match expr.func.as_str() {
            "sin" | "cos" | "sqrt" | "exp" => {
                if expr.args.len() != 1 {
                    self.add_type_error(
                        "Function takes 1 argument".to_string(),
                        expr.span
                    );
                } else {
                    let arg_ty = expr.args[0].accept(self);
                    if !arg_ty.is_compatible_with(&HulkType::Number) {
                        self.add_type_error(
                            "Argument must be Number".to_string(),
                            expr.args[0].span()
                        );
                    }
                }
                HulkType::Number
            }
            "rand" => {
                if !expr.args.is_empty() {
                    self.add_type_error(
                        "rand takes 0 arguments".to_string(),
                        expr.span
                    );
                }
                HulkType::Number
            }
            "log" => {
                if expr.args.len() != 2 {
                    self.add_type_error(
                        "log expects 2 arguments (base, value)".to_string(),
                        expr.span
                    );
                } else {
                    let base_ty = expr.args[0].accept(self);
                    let val_ty = expr.args[1].accept(self);
                    if !base_ty.is_compatible_with(&HulkType::Number) {
                        self.add_type_error(
                            "log base must be Number".to_string(),
                            expr.args[0].span()
                        );
                    }
                    if !val_ty.is_compatible_with(&HulkType::Number) {
                        self.add_type_error(
                            "log value must be Number".to_string(),
                            expr.args[1].span()
                        );
                    }
                }
                HulkType::Number
            }
            _ => {
                self.add_type_error(
                    format!("Unknown function '{}'", expr.func),
                    expr.span
                );
                HulkType::Error
            }
        };

        expr.ty = Some(result_ty.clone());
        result_ty
    }

    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result {
        let ty = HulkType::Boolean;
        expr.ty = Some(ty.clone());
        ty
    }

    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result {
        let operand_ty = expr.expr.accept(self);
        let result_ty = match expr.op {
            UnaryOp::Not => {
                if !operand_ty.is_compatible_with(&HulkType::Boolean) {
                    self.add_type_error(
                        "Negation (!) requires Boolean operand".to_string(),
                        expr.span
                    );
                }
                HulkType::Boolean
            }
            UnaryOp::Neg => {
                if !operand_ty.is_compatible_with(&HulkType::Number) {
                    self.add_type_error(
                        "Unary negation (-) requires Number operand".to_string(),
                        expr.span
                    );
                }
                HulkType::Number
            }
        };
        expr.ty = Some(result_ty.clone());
        result_ty
    }
}