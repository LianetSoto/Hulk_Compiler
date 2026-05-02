use crate::ast::expr::UnaryOp;
use crate::ast::*;
use crate::error::CompilerError;
use super::types::HulkType;
use crate::error::Span;
use std::collections::HashMap;

pub struct TypeChecker {
    errors: Vec<CompilerError>,
    scopes: Vec<HashMap<String, HulkType>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(HashMap::new()); // Scope global
        Self {
            errors: Vec::new(),
            scopes,
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

    /// Crea un nuevo scope (ámbito de variables)
    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Sale del scope actual
    fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Declara una variable en el scope actual
    fn declare_var(&mut self, name: String, ty: HulkType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    /// Busca una variable en los scopes (desde el más interno al global)
    fn lookup_var(&self, name: &str) -> Option<HulkType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
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
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow | BinOp::Mod=> {
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
        let ty = arg_type; 
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
            "range" => {
                if expr.args.len() != 2 {
                    self.add_type_error("range expects 2 arguments (start, end)".to_string(), expr.span);
                } else {
                    let start_ty = expr.args[0].accept(self);
                    let end_ty = expr.args[1].accept(self);
                    if !start_ty.is_compatible_with(&HulkType::Number) {
                        self.add_type_error("range start must be Number".to_string(), expr.args[0].span());
                    }
                    if !end_ty.is_compatible_with(&HulkType::Number) {
                        self.add_type_error("range end must be Number".to_string(), expr.args[1].span());
                    }
                }
                // range devuelve un iterable (por ahora tratamos como Number o podrías definir un tipo especial)
                HulkType::Object
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

    fn visit_variable(&mut self, expr: &mut VariableExpr) -> Self::Result {
        match self.lookup_var(&expr.name) {
            Some(ty) => {
                expr.ty = Some(ty.clone());
                ty
            }
            None => {
                self.add_type_error(
                    format!("Undefined variable '{}'", expr.name),
                    expr.span
                );
                let ty = HulkType::Error;
                expr.ty = Some(ty.clone());
                ty
            }
        }
    }

    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result {
        // Crear nuevo scope
        self.enter_scope();

        // Declarar cada binding
        for (name, init_expr) in &mut expr.bindings {
            let init_ty = init_expr.accept(self);
            self.declare_var(name.clone(), init_ty);
        }

        // Evaluar el cuerpo en el nuevo scope
        let body_ty = expr.body.accept(self);

        // Salir del scope
        self.exit_scope();

        // El tipo del let es el tipo del cuerpo
        expr.ty = Some(body_ty.clone());
        body_ty
    }

    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> Self::Result {
        // Buscar la variable
        let var_ty = match self.lookup_var(&expr.name) {
            Some(ty) => ty,
            None => {
                self.add_type_error(
                    format!("Undefined variable '{}'", expr.name),
                    expr.span
                );
                HulkType::Error
            }
        };

        // Evaluar el valor asignado
        let value_ty = expr.value.accept(self);

        // Verificar compatibilidad de tipos
        if var_ty != HulkType::Error && !value_ty.is_compatible_with(&var_ty) {
            self.add_type_error(
                format!("Cannot assign {:?} to variable of type {:?}", value_ty, var_ty),
                expr.span
            );
        }

        // El tipo de una asignación es el tipo del valor
        expr.ty = Some(value_ty.clone());
        value_ty
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {

        let mut result_ty = HulkType::Number; // Por defecto

        // Evaluar cada expresión en orden
        for e in &mut expr.expressions {
            result_ty = e.accept(self);
        }

        // El tipo del bloque es el tipo de la última expresión
        expr.ty = Some(result_ty.clone());
        result_ty
    }

    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        let cond_ty = expr.condition.accept(self);
        if !cond_ty.is_compatible_with(&HulkType::Boolean) {
            self.add_type_error(
                "If condition must be Boolean".to_string(),
                expr.condition.span(),
            );
        }

        let then_ty = expr.then_branch.accept(self);
        let else_ty = expr.else_branch.accept(self);

        let result_ty = if then_ty.is_compatible_with(&else_ty) {
            then_ty.clone()
        } else {
            self.add_type_error(
                format!("If branches return incompatible types: {:?} vs {:?}", then_ty, else_ty),
                expr.span,
            );
            HulkType::Error
        };

        expr.ty = Some(result_ty.clone());
        result_ty
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        let cond_ty = expr.condition.accept(self);
        if !cond_ty.is_compatible_with(&HulkType::Boolean) {
            self.add_type_error(
                "While condition must be Boolean".to_string(),
                expr.condition.span(),
            );
        }

        let body_ty = expr.body.accept(self);

        expr.ty = Some(body_ty.clone());
        body_ty
    }

    fn visit_for(&mut self, expr: &mut ForExpr) -> Self::Result {
        let iterable_ty = expr.iterable.accept(self);
        if !iterable_ty.is_compatible_with(&HulkType::Object) && !iterable_ty.is_compatible_with(&HulkType::Number) {
            self.add_type_error(
                "For iterable must be a range or iterable object".to_string(),
                expr.iterable.span(),
            );
        }

        self.declare_var(expr.var.clone(), HulkType::Number);
        let body_ty = expr.body.accept(self);

        expr.ty = Some(body_ty.clone());
        body_ty
    }
}
