use crate::ast::expr::UnaryOp;
use crate::ast::*;
use crate::error::CompilerError;
use super::types::HulkType;
use crate::error::Span;
use std::collections::{HashMap, HashSet};
use crate::semantic::inference::Unifier;

#[derive(Clone)]
struct FunctionInfo {
    params_len: usize,
    param_types: Option<Vec<HulkType>>,
    return_type: Option<HulkType>,
}

pub struct TypeChecker {
    errors: Vec<CompilerError>,
    scopes: Vec<HashMap<String, HulkType>>,
    functions: HashMap<String, FunctionInfo>,
    unifier: Unifier,               // para gestionar variables de tipo
    param_vars: HashMap<String, Vec<HulkType>>, // para cada función, lista de variables de tipo de sus parámetros
    return_var: HashMap<String, HulkType>,      // variable de tipo para el retorno
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(HashMap::new()); // Scope global
        Self {
            errors: Vec::new(),
            scopes,
            functions: HashMap::new(),
            unifier: Unifier::default(),
            param_vars: HashMap::new(),
            return_var: HashMap::new()
        }
    }

    pub fn check(&mut self, program: &mut Program) -> Result<(), Vec<CompilerError>> {
        // 1. Registrar nombres de funciones y crear variables de tipo para parámetros y retorno
        for func in &program.functions {
            self.functions.insert(func.name.clone(), FunctionInfo {
                params_len: func.params.len(),
                return_type: None,
                param_types: None// lo llenaremos después de inferir
            });
        }
        self.prepare_function_vars(program);

        // 2. Inferir tipos función por función (esto visita los cuerpos)
        for func in &mut program.functions {
            func.accept(self);
            // Después de visitar, ya tenemos los tipos resueltos en func.params[].ty
            // Guardar los tipos resueltos en self.functions para usarlos en llamadas
            let param_types: Vec<HulkType> = func.params.iter()
                .map(|p| p.ty.as_ref().unwrap().clone())
                .collect();
            if let Some(info) = self.functions.get_mut(&func.name) {
                info.param_types = Some(param_types);
                info.return_type = func.ty.clone();
            }
        }

        // 3. Verificar la expresión principal (main_expr)
        program.main_expr.accept(self);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn is_assignable(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::Variable(_) => true,          // asignación a variable local
            Expr::AttributeAccess(attr) => true, // asignación a atributo (self.x, obj.y)
            // Puedes extender a accesos a vectores, etc.
            _ => false,
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

    fn prepare_function_vars(&mut self, program: &Program) {
        for func in &program.functions {
            let mut param_vars = Vec::new();
            for _ in &func.params {
                param_vars.push(self.unifier.new_var());
            }
            let ret_var = self.unifier.new_var();

            // Guardar en los mapas auxiliares (ya los tienes)
            self.param_vars.insert(func.name.clone(), param_vars.clone());
            self.return_var.insert(func.name.clone(), ret_var.clone());

            // IMPORTANTE: actualizar self.functions con estas variables
            if let Some(info) = self.functions.get_mut(&func.name) {
                info.param_types = Some(param_vars);
                info.return_type = Some(ret_var);
            }
        }
    }
}

impl Visitor for TypeChecker {
    type Result = HulkType;

    

    fn visit_program(&mut self, program: &mut Program) -> Self::Result {
        // Registrar funciones
        for func in &program.functions {
            self.functions.insert(func.name.clone(), FunctionInfo {
                params_len: func.params.len(),
                param_types: None,
                return_type: None,
            });
        }
        self.prepare_function_vars(program);

        // Inferir funciones
        for func in &mut program.functions {
            func.accept(self);
        }

        // Expresión principal
        let main_ty = program.main_expr.accept(self);
        main_ty
    }

    fn visit_number(&mut self, expr: &mut NumberExpr) -> Self::Result {
        let ty = HulkType::Number;
        expr.ty = Some(ty.clone());
        ty
    }

    fn visit_binary_op(&mut self, expr: &mut BinaryOpExpr) -> Self::Result {
        let left_ty = expr.left.accept(self);
        let right_ty = expr.right.accept(self);
        let result_var = self.unifier.new_var();

        match expr.op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow | BinOp::Mod => {
                if let Err(msg) = self.unifier.unify(&left_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.left.span());
                }
                if let Err(msg) = self.unifier.unify(&right_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.right.span());
                }
                if let Err(msg) = self.unifier.unify(&result_var, &HulkType::Number) {
                    self.add_type_error(msg, expr.span);
                }
            }
            BinOp::Concat | BinOp::ConcatSpace => {
                if let Err(msg) = self.unifier.unify(&left_ty, &HulkType::String) {
                    self.add_type_error(msg, expr.left.span());
                }
                if let Err(msg) = self.unifier.unify(&right_ty, &HulkType::String) {
                    self.add_type_error(msg, expr.right.span());
                }
                if let Err(msg) = self.unifier.unify(&result_var, &HulkType::String) {
                    self.add_type_error(msg, expr.span);
                }
            }
            BinOp::Eq | BinOp::Neq => {
                if let Err(msg) = self.unifier.unify(&left_ty, &right_ty) {
                    self.add_type_error(msg, expr.span);
                }
                if let Err(msg) = self.unifier.unify(&result_var, &HulkType::Boolean) {
                    self.add_type_error(msg, expr.span);
                }
            }
            BinOp::Lt | BinOp::Gt | BinOp::Leq | BinOp::Geq => {
                if let Err(msg) = self.unifier.unify(&left_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.left.span());
                }
                if let Err(msg) = self.unifier.unify(&right_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.right.span());
                }
                if let Err(msg) = self.unifier.unify(&result_var, &HulkType::Boolean) {
                    self.add_type_error(msg, expr.span);
                }
            }
            BinOp::And | BinOp::Or => {
                if let Err(msg) = self.unifier.unify(&left_ty, &HulkType::Boolean) {
                    self.add_type_error(msg, expr.left.span());
                }
                if let Err(msg) = self.unifier.unify(&right_ty, &HulkType::Boolean) {
                    self.add_type_error(msg, expr.right.span());
                }
                if let Err(msg) = self.unifier.unify(&result_var, &HulkType::Boolean) {
                    self.add_type_error(msg, expr.span);
                }
            }
        }

        let final_ty = self.unifier.resolve(&result_var);
        expr.ty = Some(final_ty.clone());
        final_ty
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
        match expr.func.as_str() {
            "sin" | "cos" | "sqrt" | "exp" => {
                if expr.args.len() != 1 {
                    self.add_type_error("Function takes 1 argument".into(), expr.span);
                    return HulkType::Error;
                }
                let arg_ty = expr.args[0].accept(self);
                if let Err(msg) = self.unifier.unify(&arg_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.args[0].span());
                }
                let ret_ty = HulkType::Number;
                expr.ty = Some(ret_ty.clone());
                ret_ty
            }
            "rand" => {
                if !expr.args.is_empty() {
                    self.add_type_error("rand takes 0 arguments".into(), expr.span);
                    return HulkType::Error;
                }
                let ret_ty = HulkType::Number;
                expr.ty = Some(ret_ty.clone());
                ret_ty
            }
            "log" => {
                if expr.args.len() != 2 {
                    self.add_type_error("log expects 2 arguments".into(), expr.span);
                    return HulkType::Error;
                }
                let base_ty = expr.args[0].accept(self);
                let val_ty = expr.args[1].accept(self);
                if let Err(msg) = self.unifier.unify(&base_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.args[0].span());
                }
                if let Err(msg) = self.unifier.unify(&val_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.args[1].span());
                }
                let ret_ty = HulkType::Number;
                expr.ty = Some(ret_ty.clone());
                ret_ty
            }
            "range" => {
                if expr.args.len() != 2 {
                    self.add_type_error("range expects 2 arguments".into(), expr.span);
                    return HulkType::Error;
                }
                let start_ty = expr.args[0].accept(self);
                let end_ty = expr.args[1].accept(self);
                if let Err(msg) = self.unifier.unify(&start_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.args[0].span());
                }
                if let Err(msg) = self.unifier.unify(&end_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.args[1].span());
                }
                let ret_ty = HulkType::Object;
                expr.ty = Some(ret_ty.clone());
                ret_ty
            }
            "print" => {
                if expr.args.len() != 1 {
                    self.add_type_error("print expects 1 argument".into(), expr.span);
                    return HulkType::Error;
                }
                let arg_ty = expr.args[0].accept(self);
                expr.ty = Some(arg_ty.clone());
                arg_ty
            }
            _ => {
                // Función definida por el usuario
                if let Some(func_info) = self.functions.get(&expr.func).cloned() {
                    if expr.args.len() != func_info.params_len {
                        self.add_type_error(
                            format!("Function '{}' expects {} arguments", expr.func, func_info.params_len),
                            expr.span,
                        );
                        return HulkType::Error;
                    }
                    let param_types = match func_info.param_types {
                        Some(pts) => pts,
                        None => {
                            self.add_type_error(format!("Function '{}' has not been inferred yet", expr.func), expr.span);
                            return HulkType::Error;
                        }
                    };
                    for (arg, expected) in expr.args.iter_mut().zip(param_types.iter()) {
                        let arg_ty = arg.accept(self);
                        if let Err(msg) = self.unifier.unify(&arg_ty, expected) {
                            self.add_type_error(msg, arg.span());
                        }
                    }
                    let ret_ty = func_info.return_type.unwrap_or(HulkType::Object);
                    expr.ty = Some(ret_ty.clone());
                    ret_ty
                } else {
                    self.add_type_error(format!("Unknown function '{}'", expr.func), expr.span);
                    HulkType::Error
                }
            }
        }
    }

    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result {
        let ty = HulkType::Boolean;
        expr.ty = Some(ty.clone());
        ty
    }

    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result {
        let operand_ty = expr.expr.accept(self);
        let result_var = self.unifier.new_var();

        match expr.op {
            UnaryOp::Not => {
                if let Err(msg) = self.unifier.unify(&operand_ty, &HulkType::Boolean) {
                    self.add_type_error(msg, expr.expr.span());
                }
                if let Err(msg) = self.unifier.unify(&result_var, &HulkType::Boolean) {
                    self.add_type_error(msg, expr.span);
                }
            }
            UnaryOp::Neg => {
                if let Err(msg) = self.unifier.unify(&operand_ty, &HulkType::Number) {
                    self.add_type_error(msg, expr.expr.span());
                }
                if let Err(msg) = self.unifier.unify(&result_var, &HulkType::Number) {
                    self.add_type_error(msg, expr.span);
                }
            }
        }

        let final_ty = self.unifier.resolve(&result_var);
        expr.ty = Some(final_ty.clone());
        final_ty
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
        self.enter_scope();
        for (name, init_expr) in &mut expr.bindings {
            let init_ty = init_expr.accept(self);
            self.declare_var(name.clone(), init_ty);
        }
        let body_ty = expr.body.accept(self);
        self.exit_scope();
        expr.ty = Some(body_ty.clone());
        body_ty
    }

    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> HulkType {
        // Evaluate the type of the left-hand side (lhs)
        let lhs_ty = expr.lhs.accept(self);
        
        // Check that the lhs is assignable (e.g., variable or attribute access)
        if !self.is_assignable(&expr.lhs) {
            self.add_type_error(
                format!("Left-hand side of assignment is not assignable"),
                expr.span
            );
            expr.ty = Some(HulkType::Error);
            return HulkType::Error;
        }
        
        // Evaluate the type of the assigned value
        let value_ty = expr.value.accept(self);
        // Verify type compatibility
        if lhs_ty != HulkType::Error && !value_ty.is_compatible_with(&lhs_ty) {
            self.add_type_error(
                format!("Cannot assign {:?} to expression of type {:?}", value_ty, lhs_ty),
                expr.span
            );
        }
        expr.ty = Some(value_ty.clone());
        value_ty
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {
        let mut last_ty = HulkType::Number; // fallback, pero HULK siempre tiene al menos una expresión
        for e in &mut expr.expressions {
            last_ty = e.accept(self);
        }
        expr.ty = Some(last_ty.clone());
        last_ty
    }

    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        let cond_ty = expr.condition.accept(self);
        if let Err(msg) = self.unifier.unify(&cond_ty, &HulkType::Boolean) {
            self.add_type_error(msg, expr.condition.span());
        }
        let then_ty = expr.then_branch.accept(self);
        let else_ty = expr.else_branch.accept(self);
        let result_var = self.unifier.new_var();
        if let Err(msg) = self.unifier.unify(&then_ty, &else_ty) {
            self.add_type_error(msg, expr.span);
        }
        if let Err(msg) = self.unifier.unify(&result_var, &then_ty) {
            self.add_type_error(msg, expr.span);
        }
        let final_ty = self.unifier.resolve(&result_var);
        expr.ty = Some(final_ty.clone());
        final_ty
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        let cond_ty = expr.condition.accept(self);
        if let Err(msg) = self.unifier.unify(&cond_ty, &HulkType::Boolean) {
            self.add_type_error(msg, expr.condition.span());
        }
        let body_ty = expr.body.accept(self);
        expr.ty = Some(body_ty.clone());
        body_ty
    }

    fn visit_function_def(&mut self, func: &mut FunctionDef) -> Self::Result {
        let param_vars = self.param_vars.get(&func.name).expect("No param vars").clone();
        let ret_var = self.return_var.get(&func.name).expect("No return var").clone();

        self.enter_scope();
        for (param, var_ty) in func.params.iter_mut().zip(param_vars.iter()) {
            param.ty = Some(var_ty.clone());
            self.declare_var(param.name.clone(), var_ty.clone());
        }

        let body_ty = func.body.accept(self);

        if let Err(msg) = self.unifier.unify(&body_ty, &ret_var) {
            self.add_type_error(msg, func.span);
        }

        let resolved_params: Vec<HulkType> = param_vars.iter()
            .map(|v| self.unifier.resolve(v))
            .collect();
        for (param, resolved) in func.params.iter_mut().zip(resolved_params.iter()) {
            param.ty = Some(resolved.clone());
        }

        let resolved_ret = self.unifier.resolve(&ret_var);
        func.ty = Some(resolved_ret.clone());

        if let Some(info) = self.functions.get_mut(&func.name) {
            info.param_types = Some(resolved_params);
            info.return_type = Some(resolved_ret.clone());
        }

        self.exit_scope();
        resolved_ret
    }
    
    fn visit_type_def(&mut self, ty: &mut TypeDef) -> Self::Result {
        todo!()
    }
    
    fn visit_attribute(&mut self, attr: &mut Attribute) -> Self::Result {
        todo!()
    }
    
    fn visit_method(&mut self, m: &mut Method) -> Self::Result {
        todo!()
    }
    
    fn visit_new(&mut self, e: &mut NewExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_method_call(&mut self, e: &mut MethodCallExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_self(&mut self, e: &mut SelfExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_base(&mut self, e: &mut BaseExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_attribute_access(&mut self, e: &mut AttributeAccessExpr) -> Self::Result {
        todo!()
    }
}
