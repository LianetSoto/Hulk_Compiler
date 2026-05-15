use crate::ast::expr::UnaryOp;
use crate::ast::*;
use crate::error::CompilerError;
use super::types::HulkType;
use crate::error::Span;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct FunctionInfo {
    params_len: usize,
    return_type: Option<HulkType>,
}

pub struct TypeChecker {
    errors: Vec<CompilerError>,
    scopes: Vec<HashMap<String, HulkType>>,
    functions: HashMap<String, FunctionInfo>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(HashMap::new()); // Scope global
        Self {
            errors: Vec::new(),
            scopes,
            functions: HashMap::new(),
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

    // fn visit_program(&mut self, program: &mut Program) -> Self::Result {
   
    //     for stmt in &mut program.statements {
    //         match stmt {
    //             Stmt::Function(func) => {
    //                 // 1. verificar si la funcion ya fue declarada (Duplicado)
    //                 if self.functions.contains_key(&func.name) {
    //                     self.add_type_error(
    //                         format!("Duplicate function '{}'", func.name),
    //                         func.span,
    //                     );
    //                 } else {
    //                     // 2. registrar la funcion ANTES de revisar su cuerpo.
                      
    //                     self.functions.insert(func.name.clone(), FunctionInfo {
    //                         params_len: func.params.len(),
    //                         return_type: None, // Se actualizará al analizar el cuerpo
    //                     });
    //                 }

    //                 // 3. revisdar el cuerpo de la funcion.
                    
    //                 func.accept(self);
    //             }
    //             Stmt::Expr(expr_stmt) => {
    //                 expr_stmt.expr.accept(self);
    //             }
    //         }
    //     }

    //     HulkType::Number
    // }
    fn visit_program(&mut self, program: &mut Program) -> Self::Result {
        // 1. Registrar y analizar todas las funciones
        for func in &mut program.functions {
            // Verificar duplicado
            if self.functions.contains_key(&func.name) {
                self.add_type_error(
                    format!("Duplicate function '{}'", func.name),
                    func.span,
                );
            } else {
                self.functions.insert(func.name.clone(), FunctionInfo {
                    params_len: func.params.len(),
                    return_type: None,
                });
            }
            // Analizar el cuerpo de la función
            func.accept(self);
        }

        // 2. Analizar la expresión principal y devolver su tipo
        let main_ty = program.main_expr.accept(self);
        main_ty
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

            BinOp::Concat | BinOp::ConcatSpace => {
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
                if let Some(func_info) = self.functions.get(&expr.func).cloned() {
                    if expr.args.len() != func_info.params_len {
                        self.add_type_error(
                            format!("Function '{}' expects {} arguments", expr.func, func_info.params_len),
                            expr.span,
                        );
                    }

                    for arg in &mut expr.args {
                        arg.accept(self);
                    }

                    return match func_info.return_type {
                        Some(ret_ty) => {
                            expr.ty = Some(ret_ty.clone());
                            ret_ty
                        }
                        None => {
                            expr.ty = Some(HulkType::Object);
                            HulkType::Object
                        }
                    };
                }

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
    
    // fn visit_function_def(&mut self, func: &mut FunctionDef) -> Self::Result {
    //     let mut seen_params = HashSet::new();
    //     for param in &func.params {
    //         if !seen_params.insert(param.name.clone()) {
    //             self.add_type_error(
    //                 format!("Duplicate parameter name '{}' in function '{}'", param.name.clone(), func.name),
    //                 func.span,
    //             );
    //         }
    //     }

    //     self.enter_scope();
    //     for param in &func.params {
    //         self.declare_var(param.name.clone(), HulkType::Object);
    //     }

    //     let body_ty = func.body.accept(self);
    //     self.exit_scope();

    //     func.ty = Some(body_ty.clone());
    //     if let Some(func_info) = self.functions.get_mut(&func.name) {
    //         func_info.return_type = Some(body_ty.clone());
    //     }

    //     body_ty
    // }
    fn visit_function_def(&mut self, func: &mut FunctionDef) -> Self::Result {
        let mut seen_params = HashSet::new();
        for param in &func.params {
            if !seen_params.insert(param.name.clone()) {
                self.add_type_error(
                    format!("Duplicate parameter name '{}' in function '{}'", param.name.clone(), func.name),
                    func.span,
                );
            }
        }

        self.enter_scope();
        for param in &func.params {
            // Cambio: los parámetros ahora son de tipo Number en lugar de Object
            self.declare_var(param.name.clone(), HulkType::Number);
        }

        let body_ty = func.body.accept(self);
        self.exit_scope();

        func.ty = Some(body_ty.clone());
        if let Some(func_info) = self.functions.get_mut(&func.name) {
            func_info.return_type = Some(body_ty.clone());
        }

        body_ty
    }
}
