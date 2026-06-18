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
    is_generic: bool, 
}

#[derive(Clone)]
struct TypeInfo {
    parent: Option<String>,                     // nombre del padre
    param_vars: Vec<HulkType>, 
    attributes: HashMap<String, HulkType>,      // nombre -> tipo (inferido)
    attr_order: Vec<String>,
    methods: HashMap<String, MethodInfo>,       // nombre -> info del método
}

#[derive(Clone)]
struct MethodInfo {
    param_types: Vec<HulkType>,
    return_type: HulkType,
}

pub struct TypeChecker {
    errors: Vec<CompilerError>,
    scopes: Vec<HashMap<String, HulkType>>,
    functions: HashMap<String, FunctionInfo>,
    unifier: Unifier,               // para gestionar variables de tipo
    param_vars: HashMap<String, Vec<HulkType>>, // para cada función, lista de variables de tipo de sus parámetros
    return_var: HashMap<String, HulkType>,      // variable de tipo para el retorno
    types: HashMap<String, TypeInfo>,          // nombre -> info del tipo
    current_type: Option<String>,               // tipo que se está chequeando
    self_type: Option<HulkType>,
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
            return_var: HashMap::new(),
            types: HashMap::new(),
            current_type: None,
            self_type: None,
        }
    }

    pub fn check(&mut self, program: &mut Program) -> Result<(), Vec<CompilerError>> {
        
        // 0. Procesar tipos (TypeDef) primero
        for type_def in &mut program.types {
            type_def.accept(self);
        }
        
        // 1. Registrar nombres de funciones y crear variables de tipo para parámetros y retorno
        for func in &program.functions {
            self.functions.insert(func.name.clone(), FunctionInfo {
                params_len: func.params.len(),
                return_type: None,
                param_types: None,// lo llenaremos después de inferir
                is_generic: false,
            });
        }
        self.prepare_function_vars(program);

        // 2. Inferir tipos función por función (esto visita los cuerpos)
        // Las funciones generarán variables de tipo que se vincularán durante las llamadas
        for func in &mut program.functions {
            func.accept(self);
            // Los tipos ya están guardados en self.functions sin resolver
        }

        // 3. Verificar la expresión principal (main_expr)
        // Aquí se hacen las llamadas, que vinculan las variables de tipo
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

    pub fn resolve_ast(&mut self, program: &mut Program) {
        // Función recursiva para resolver tipos en una expresión
        fn resolve_expr(unifier: &Unifier, expr: &mut Expr) {
            match expr {
                Expr::Number(n) => {
                    if let Some(ty) = &n.ty {
                        n.ty = Some(unifier.resolve(ty));
                    }
                }
                Expr::BinaryOp(b) => {
                    if let Some(ty) = &b.ty {
                        b.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut b.left);
                    resolve_expr(unifier, &mut b.right);
                }
                Expr::String(s) => {
                    if let Some(ty) = &s.ty {
                        s.ty = Some(unifier.resolve(ty));
                    }
                }
                Expr::Call(c) => {
                    if let Some(ty) = &c.ty {
                        c.ty = Some(unifier.resolve(ty));
                    }
                    for arg in &mut c.args {
                        resolve_expr(unifier, arg);
                    }
                }
                Expr::Const(c) => {
                    if let Some(ty) = &c.ty {
                        c.ty = Some(unifier.resolve(ty));
                    }
                }
                Expr::Bool(b) => {
                    if let Some(ty) = &b.ty {
                        b.ty = Some(unifier.resolve(ty));
                    }
                }
                Expr::UnaryOp(u) => {
                    if let Some(ty) = &u.ty {
                        u.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut u.expr);
                }
                Expr::Variable(v) => {
                    if let Some(ty) = &v.ty {
                        v.ty = Some(unifier.resolve(ty));
                    }
                }
                Expr::Let(l) => {
                    if let Some(ty) = &l.ty {
                        l.ty = Some(unifier.resolve(ty));
                    }
                    for (_, _, init) in &mut l.bindings {
                        resolve_expr(unifier, init);
                    }
                    resolve_expr(unifier, &mut l.body);
                }
                Expr::DestructiveAssign(a) => {
                    if let Some(ty) = &a.ty {
                        a.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut a.lhs);
                    resolve_expr(unifier, &mut a.value);
                }
                Expr::Block(b) => {
                    if let Some(ty) = &b.ty {
                        b.ty = Some(unifier.resolve(ty));
                    }
                    for e in &mut b.expressions {
                        resolve_expr(unifier, e);
                    }
                }
                Expr::If(i) => {
                    if let Some(ty) = &i.ty {
                        i.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut i.condition);
                    resolve_expr(unifier, &mut i.then_branch);
                    resolve_expr(unifier, &mut i.else_branch);
                }
                Expr::While(w) => {
                    if let Some(ty) = &w.ty {
                        w.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut w.condition);
                    resolve_expr(unifier, &mut w.body);
                }
                Expr::New(n) => {
                    if let Some(ty) = &n.ty {
                        n.ty = Some(unifier.resolve(ty));
                    }
                    for arg in &mut n.args {
                        resolve_expr(unifier, arg);
                    }
                }
                Expr::MethodCall(m) => {
                    if let Some(ty) = &m.ty {
                        m.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut m.object);
                    for arg in &mut m.args {
                        resolve_expr(unifier, arg);
                    }
                }
                Expr::SelfExpr(s) => {
                    if let Some(ty) = &s.ty {
                        s.ty = Some(unifier.resolve(ty));
                    }
                }
                Expr::Base(b) => {
                    if let Some(ty) = &b.ty {
                        b.ty = Some(unifier.resolve(ty));
                    }
                }
                Expr::AttributeAccess(a) => {
                    if let Some(ty) = &a.ty {
                        a.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut a.object);
                }
            }
        }

        // Resolver tipos en funciones
        for func in &mut program.functions {
            if let Some(ty) = &func.ty {
                func.ty = Some(self.unifier.resolve(ty));
            }
            for param in &mut func.params {
                if let Some(ty) = &param.ty {
                    param.ty = Some(self.unifier.resolve(ty));
                }
            }
            resolve_expr(&self.unifier, &mut func.body);
        }

        // Resolver tipos en definiciones de tipos
        for type_def in &mut program.types {
            if let Some(ty) = &type_def.ty {
                type_def.ty = Some(self.unifier.resolve(ty));
            }
            for attr in &mut type_def.attributes {
                if let Some(ty) = &attr.ty {
                    attr.ty = Some(self.unifier.resolve(ty));
                }
                if let Some(ann) = &attr.ty_annotation {
                    attr.ty_annotation = Some(self.unifier.resolve(ann));
                }
                resolve_expr(&self.unifier, &mut attr.init_expr);
            }
            for method in &mut type_def.methods {
                if let Some(ty) = &method.ty {
                    method.ty = Some(self.unifier.resolve(ty));
                }
                for param in &mut method.params {
                    if let Some(ty) = &param.ty {
                        param.ty = Some(self.unifier.resolve(ty));
                    }
                    if let Some(ann) = &param.ty_annotation {
                        param.ty_annotation = Some(self.unifier.resolve(ann));
                    }
                }
                resolve_expr(&self.unifier, &mut method.body);
            }
            // Después de resolver atributos y métodos, resolver param_types
            type_def.param_types = type_def.param_types.iter()
                .map(|t| self.unifier.resolve(t))
                .collect();
        }

        // Resolver la expresión principal
        resolve_expr(&self.unifier, &mut program.main_expr);
    }


    pub fn verify_no_type_vars(&self, program: &Program) -> Result<(), Vec<CompilerError>> {
        let mut errors = Vec::new();

        // Función auxiliar para verificar un tipo y añadir error si es Var
        fn check_type(ty: &Option<HulkType>, span: Span, name: &str, errors: &mut Vec<CompilerError>) {
            if let Some(HulkType::Var(id)) = ty {
                errors.push(CompilerError::TypeError {
                    msg: format!(
                        "Type of {} could not be inferred. Please provide an explicit type annotation (e.g., x: Number",
                        name
                    ),
                    span,
                });
            }
        }

        // Recorrer todas las definiciones de tipos
        for type_def in &program.types {
            let type_name = &type_def.name;

            // Verificar parámetros formales del tipo
            for (i, param_ty) in type_def.param_types.iter().enumerate() {
                if let HulkType::Var(id) = param_ty {
                    errors.push(CompilerError::TypeError {
                        msg: format!(
                            "Parameter '{}' of type '{}' could not be inferred (Var({})). Please add an explicit type annotation.",
                            type_def.params.get(i).unwrap_or(&"?".to_string()),
                            type_name,
                            id
                        ),
                        span: type_def.span,
                    });
                }
            }

            // Verificar atributos
            for attr in &type_def.attributes {
                check_type(&attr.ty, attr.span, &format!("attribute '{}' of type '{}'", attr.name, type_name), &mut errors);
            }

            // Verificar métodos
            for method in &type_def.methods {
                check_type(&method.ty, method.span, &format!("return type of method '{}' in type '{}'", method.name, type_name), &mut errors);
                for param in &method.params {
                    check_type(&param.ty, param.span, &format!("parameter '{}' of method '{}' in type '{}'", param.name, method.name, type_name), &mut errors);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Visitor for TypeChecker {
    type Result = HulkType;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result {

        // PASO 2: Procesar cada tipo (atributos y métodos) con el contexto adecuado
        for type_def in &mut program.types {
            type_def.accept(self); 
        }
        
        // Registrar funciones
        for func in &program.functions {
            self.functions.insert(func.name.clone(), FunctionInfo {
                params_len: func.params.len(),
                param_types: None,
                return_type: None,
                 is_generic: false,
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
                // Permitir operandos String o Number
                let left_ok = left_ty.is_compatible_with(&HulkType::String) ||
                            left_ty.is_compatible_with(&HulkType::Number);
                if !left_ok {
                    self.add_type_error(
                        "Left operand of @ must be String or Number".to_string(),
                        expr.left.span()
                    );
                }
                let right_ok = right_ty.is_compatible_with(&HulkType::String) ||
                            right_ty.is_compatible_with(&HulkType::Number);
                if !right_ok {
                    self.add_type_error(
                        "Right operand of @ must be String or Number".to_string(),
                        expr.right.span()
                    );
                }
                // El resultado de la concatenación siempre es String
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
                    // Si la función es genérica, instanciar variables de tipo frescas
                    let (param_types, ret_ty) = if func_info.is_generic {
                        let mut var_map = HashMap::new();
                        
                        // Función auxiliar para instanciar (clonar) variables genéricas 
                        // sin alterar las originales de la definición
                        let mut instantiate = |ty: &HulkType, unifier: &mut Unifier, map: &mut HashMap<usize, HulkType>| -> HulkType {
                            let applied = unifier.apply(ty);
                            if let HulkType::Var(id) = applied {
                                if let Some(new_ty) = map.get(&id) {
                                    new_ty.clone()
                                } else {
                                    let new_var = unifier.new_var();
                                    map.insert(id, new_var.clone());
                                    new_var
                                }
                            } else {
                                applied
                            }
                        };

                        if let Some(orig_ret_var) = self.return_var.get(&expr.func) {
                            if let Some(orig_params) = self.param_vars.get(&expr.func) {
                                let new_params: Vec<HulkType> = orig_params.iter()
                                    .map(|p| instantiate(p, &mut self.unifier, &mut var_map))
                                    .collect();
                                
                                let new_ret = instantiate(orig_ret_var, &mut self.unifier, &mut var_map);
                                
                                (new_params, new_ret)
                            } else {
                                (vec![], HulkType::Error)
                            }
                        } else {
                            (vec![], HulkType::Error)
                        }
                    } else {
                        // Para funciones no-genéricas, usar directamente los tipos
                        let param_types = match func_info.param_types {
                            Some(pts) => pts,
                            None => {
                                self.add_type_error(format!("Function '{}' has not been inferred yet", expr.func), expr.span);
                                return HulkType::Error;
                            }
                        };
                        let ret = func_info.return_type.unwrap_or(HulkType::Object);
                        (param_types, ret)
                    };
                                        
                    for (i, (arg, expected)) in expr.args.iter_mut().zip(param_types.iter()).enumerate() {
                        let arg_ty = arg.accept(self);
                        if let Err(msg) = self.unifier.unify(&arg_ty, expected) {
                            self.add_type_error(msg, arg.span());
                        }
                    }
                    
                    // Resolver el tipo de retorno después de la unificación
                    let applied_ret_ty = self.unifier.apply(&ret_ty);
                    expr.ty = Some(applied_ret_ty.clone());
                    applied_ret_ty
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
        for (name, ann, init_expr) in &mut expr.bindings {
    let init_ty = init_expr.accept(self);
    // Ahora usa `ann` para la anotación de tipo (si existe)
    if let Some(ann_ty) = ann {
        let resolved_ann = match ann_ty {
            HulkType::UserDefined(s) => HulkType::Class(s.clone()),
            _ => ann_ty.clone(),
        };
        if let Err(msg) = self.unifier.unify(&init_ty, &resolved_ann) {
            self.add_type_error(msg, init_expr.span());
        }
        let final_ty = self.unifier.resolve(&init_ty);
        self.declare_var(name.clone(), final_ty);
    } else {
        self.declare_var(name.clone(), init_ty);
    }
}
        let body_ty = expr.body.accept(self);
        for (name, ann, init_expr) in &mut expr.bindings {
    let init_ty = init_expr.accept(self);
    if let Some(ann_ty) = ann {
        let resolved_ann = match ann_ty {
            HulkType::UserDefined(s) => HulkType::Class(s.clone()),
            _ => ann_ty.clone(),
        };
        if let Err(msg) = self.unifier.unify(&init_ty, &resolved_ann) {
            self.add_type_error(msg, init_expr.span());
        }
        // Después de unificar, resolver el tipo para obtener el concreto
        let final_ty = self.unifier.resolve(&init_ty);
        self.declare_var(name.clone(), final_ty);
    } else {
        self.declare_var(name.clone(), init_ty);
    }
}
        self.exit_scope();
        expr.ty = Some(body_ty.clone());
        body_ty
    }

    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> HulkType {
        // Verificar si el lado izquierdo es una variable llamada "self"
        if let Expr::Variable(var) = &*expr.lhs {
            if var.name == "self" {
                self.add_type_error("Cannot assign to 'self'".to_string(), expr.span);
                return HulkType::Error;
            }
        }
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
        expr.ty = Some(result_var.clone());
        result_var
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
            // Guardar la variable de tipo sin resolver
            param.ty = Some(var_ty.clone());
            self.declare_var(param.name.clone(), var_ty.clone());
        }

        let body_ty = func.body.accept(self);

        if let Err(msg) = self.unifier.unify(&body_ty, &ret_var) {
            self.add_type_error(msg, func.span);
        }


        // Guardar sin resolver - las variables de tipo sin resolver
        // Se resolverán al final en resolve_ast
        func.ty = Some(ret_var.clone());

        // Determinar si es genérica basándose en si los parámetros resultan ser Object cuando se resuelvan
        // pero por ahora, una función es genérica si tiene variables de tipo sin vincular
        // Determinar si es genérica basándose en si los parámetros resultan ser variables 
        // de tipo sin vincular después de inferir el cuerpo.
        let is_generic = param_vars.iter().any(|t| {
            matches!(self.unifier.apply(t), HulkType::Var(_))
        });

        func.is_generic = is_generic;

        if let Some(info) = self.functions.get_mut(&func.name) {
            // Guardar las variables de tipo sin resolver
            info.param_types = Some(param_vars.clone());
            info.return_type = Some(ret_var.clone());
            info.is_generic = is_generic;
        }

        for (param, var_ty) in func.params.iter_mut().zip(param_vars.iter()) {
    // Primero la anotación si existe
    if let Some(ann) = &param.ty_annotation {
        let resolved_ann = match ann {
            HulkType::UserDefined(name) => HulkType::Class(name.clone()),
            _ => ann.clone(),
        };
        if let Err(msg) = self.unifier.unify(var_ty, &resolved_ann) {
            self.add_type_error(msg, param.span);
        }
    }
    param.ty = Some(var_ty.clone());
    self.declare_var(param.name.clone(), var_ty.clone());
}

        self.exit_scope();
        ret_var
    }
    
    fn visit_type_def(&mut self, type_def: &mut TypeDef) -> Self::Result {
        if self.types.contains_key(&type_def.name) {
            self.add_type_error(format!("Duplicate type '{}'", type_def.name), type_def.span);
            return HulkType::Error;
        }

        // Verificar herencia
        let parent_name = type_def.parent.as_ref().map(|p| p.name.clone());
        if let Some(ref pname) = parent_name {
            if !self.types.contains_key(pname) && pname != "Object" {
                self.add_type_error(format!("Parent type '{}' not found", pname), type_def.span);
            }
        }

        // Crear variables de tipo para los parámetros formales
        let param_vars: Vec<HulkType> = (0..type_def.params.len())
            .map(|_| self.unifier.new_var())
            .collect();

        // Registrar el tipo con sus variables de parámetro
        let mut type_info = TypeInfo {
            parent: parent_name.clone(),
            param_vars: param_vars.clone(),
            attributes: HashMap::new(),
            attr_order: Vec::new(),
            methods: HashMap::new(),
        };

        // Si hay un tipo padre, heredar sus atributos y métodos
        if let Some(ref pname) = parent_name {
            if let Some(parent_info) = self.types.get(pname) {
                for (attr_name, attr_ty) in &parent_info.attributes {
                    type_info.attributes.insert(attr_name.clone(), attr_ty.clone());
                    if !type_info.attr_order.contains(attr_name) {
                        type_info.attr_order.push(attr_name.clone());
                    }
                }
                for (method_name, method_info) in &parent_info.methods {
                    type_info.methods.insert(method_name.clone(), method_info.clone());
                }
            }
        }

        self.types.insert(type_def.name.clone(), type_info);

        // Ámbito para la inicialización de atributos (contiene los parámetros formales)
        self.current_type = Some(type_def.name.clone());
        self.enter_scope();
        for (param_name, var_ty) in type_def.params.iter().zip(param_vars.iter()) {
            self.declare_var(param_name.clone(), var_ty.clone());
        }

        // Procesar atributos
        for attr in &mut type_def.attributes {
            attr.accept(self);
        }

        // Crear atributos implícitos para parámetros que no tienen atributos explícitos
        if let Some(current_type) = &self.current_type {
            if let Some(current_type_info) = self.types.get_mut(current_type) {
                for (param_idx, param_name) in type_def.params.iter().enumerate() {
                    if !current_type_info.attributes.contains_key(param_name) {
                        if let Some(param_var) = param_vars.get(param_idx) {
                            current_type_info.attributes.insert(param_name.clone(), param_var.clone());
                            current_type_info.attr_order.push(param_name.clone());
                        }
                    }
                }
            }
        }

        self.exit_scope();  // salir del ámbito de parámetros

        // Procesar métodos
        for method in &mut type_def.methods {
            method.accept(self);
        }
        self.current_type = None;

        // Resolver los tipos de los parámetros formales
        let resolved_param_types: Vec<HulkType> = param_vars.iter()
        .map(|v| self.unifier.resolve(v))
        .collect();

        println!("Type {} param types: {:?}", type_def.name, resolved_param_types);

        type_def.param_types = resolved_param_types;

        HulkType::Object
    }

    fn visit_attribute(&mut self, attr: &mut Attribute) -> Self::Result {
        let init_ty = attr.init_expr.accept(self);
        if let Some(ann) = &attr.ty_annotation {
            if let Err(msg) = self.unifier.unify(&init_ty, ann) {
                self.add_type_error(msg, attr.span);
            }
        }
        let final_ty = self.unifier.resolve(&init_ty);
        attr.ty = Some(final_ty.clone());

        if let Some(type_name) = &self.current_type {
            if let Some(type_info) = self.types.get_mut(type_name) {
                if !type_info.attributes.contains_key(&attr.name) {
                    type_info.attr_order.push(attr.name.clone());
                }
                type_info.attributes.insert(attr.name.clone(), final_ty.clone());
            }
        }
        final_ty
    }
    
    fn visit_method(&mut self, method: &mut Method) -> Self::Result {
        let current_type_name = self.current_type.as_ref().unwrap().clone();

        self.enter_scope();

        // Declarar self con el tipo de la clase actual (no Object)
        let self_ty = HulkType::Class(current_type_name.clone());
        self.declare_var("self".to_string(), self_ty.clone());

        // Crear variables de tipo para los parámetros
        let param_vars: Vec<HulkType> = (0..method.params.len())
            .map(|_| self.unifier.new_var())
            .collect();
        let ret_var = self.unifier.new_var();

        // Guardar la información del método (todavía con variables, sin resolver)
        let method_info = MethodInfo {
            param_types: param_vars.clone(),
            return_type: ret_var.clone(),
        };
        if let Some(type_name) = &self.current_type {
            if let Some(type_info) = self.types.get_mut(type_name) {
                type_info.methods.insert(method.name.clone(), method_info);
            }
        }

        // Declarar parámetros y procesar anotaciones de tipo
        for (param, var_ty) in method.params.iter_mut().zip(param_vars.iter()) {
            // Si hay anotación de tipo, unificarla con la variable de tipo
            if let Some(ann) = &param.ty_annotation {
                // Convertir UserDefined a Class si es necesario
                let resolved_ann = match ann {
                    HulkType::UserDefined(name) => HulkType::Class(name.clone()),
                    _ => ann.clone(),
                };
                if let Err(msg) = self.unifier.unify(var_ty, &resolved_ann) {
                    self.add_type_error(msg, param.span);
                }
            }
            param.ty = Some(var_ty.clone());
            self.declare_var(param.name.clone(), var_ty.clone());
        }

        // Procesar anotación de retorno del método
        if let Some(ret_ann) = &method.ty_annotation {
            let resolved_ret_ann = match ret_ann {
                HulkType::UserDefined(name) => HulkType::Class(name.clone()),
                _ => ret_ann.clone(),
            };
            if let Err(msg) = self.unifier.unify(&ret_var, &resolved_ret_ann) {
                self.add_type_error(msg, method.span);
            }
        }

        // Analizar el cuerpo
        let body_ty = method.body.accept(self);
        if let Err(msg) = self.unifier.unify(&body_ty, &ret_var) {
            self.add_type_error(msg, method.span);
        }

        // Resolver tipos finales (las variables de tipo que se unificaron se convierten)
        let resolved_params: Vec<HulkType> = param_vars.iter()
            .map(|v| self.unifier.resolve(v))
            .collect();
        let resolved_ret = self.unifier.resolve(&ret_var);
        method.ty = Some(resolved_ret.clone());

        // Actualizar la información guardada
        if let Some(type_name) = &self.current_type {
            if let Some(type_info) = self.types.get_mut(type_name) {
                if let Some(m_info) = type_info.methods.get_mut(&method.name) {
                    m_info.param_types = resolved_params;
                    m_info.return_type = resolved_ret.clone();
                }
            }
        }

        self.exit_scope();
        resolved_ret
    }
    
    fn visit_new(&mut self, expr: &mut NewExpr) -> Self::Result {
        let type_info = match self.types.get(&expr.type_name) {
            Some(info) => info.clone(),
            None => { /* error */ return HulkType::Error; }
        };

        if expr.args.len() != type_info.param_vars.len() {
            self.add_type_error(
                format!("Type '{}' expects {} arguments", expr.type_name, type_info.param_vars.len()),
                expr.span,
            );
            return HulkType::Error;
        }

        let mut arg_tys = Vec::new();
        for arg in &mut expr.args {
            arg_tys.push(arg.accept(self));
        }

        for (arg_ty, param_var) in arg_tys.iter().zip(type_info.param_vars.iter()) {
            if let Err(msg) = self.unifier.unify(arg_ty, param_var) {
                self.add_type_error(msg, expr.span);
            }
        }

        let obj_ty = HulkType::Class(expr.type_name.clone());
        println!("new returns: {:?}", obj_ty);
        expr.ty = Some(obj_ty.clone());
        obj_ty
    }
        
    fn visit_method_call(&mut self, expr: &mut MethodCallExpr) -> Self::Result {
        // 1. Obtener el tipo del objeto receptor
        let obj_ty = expr.object.accept(self);
        
        // 2. Determinar el nombre del tipo asociado
        let type_name = match &*expr.object {  // ← desreferenciar el Box
            Expr::Variable(var) => {
                if let Some(ty) = self.lookup_var(&var.name) {
                    println!("variable {} type: {:?}", var.name, ty);
                    match ty {
                        HulkType::Class(name) => name,
                        _ => {
                            self.add_type_error("Method call on non‑class object".to_string(), expr.span);
                            return HulkType::Error;
                        }
                    }
                } else {
                    self.add_type_error(format!("Variable '{}' not found", var.name), expr.span);
                    return HulkType::Error;
                }
            }
            Expr::SelfExpr(_) => {
                self.current_type.clone().unwrap_or_else(|| {
                    self.add_type_error("'self' used outside method".to_string(), expr.span);
                    "".to_string()
                })
            }
            _ => {
                self.add_type_error("Method call on unsupported expression".to_string(), expr.span);
                return HulkType::Error;
            }
        };
        
        // 3. Buscar el método en la definición del tipo
        let type_info = match self.types.get(&type_name) {
            Some(info) => info.clone(),
            None => {
                self.add_type_error(format!("Type '{}' not found", type_name), expr.span);
                return HulkType::Error;
            }
        };
        
        let method_info = match type_info.methods.get(&expr.method) {
            Some(m) => m.clone(),
            None => {
                self.add_type_error(format!("Method '{}' not found in type '{}'", expr.method, type_name), expr.span);
                return HulkType::Error;
            }
        };
        
        // 4. Verificar número de argumentos
        if expr.args.len() != method_info.param_types.len() {
            self.add_type_error(
                format!("Method '{}' expects {} arguments", expr.method, method_info.param_types.len()),
                expr.span,
            );
            return HulkType::Error;
        }
        
        // 5. Evaluar argumentos y unificar
        let mut arg_tys = Vec::new();
        for arg in &mut expr.args {
            arg_tys.push(arg.accept(self));
        }
        
        for (arg_ty, param_ty) in arg_tys.iter().zip(method_info.param_types.iter()) {
            if let Err(msg) = self.unifier.unify(arg_ty, param_ty) {
                self.add_type_error(msg, expr.span);
            }
        }
        
        let ret_ty = method_info.return_type.clone();
        // Resolver el tipo de retorno después de la unificación
        let applied_ret_ty = self.unifier.apply(&ret_ty);
        expr.ty = Some(applied_ret_ty.clone());
        
        applied_ret_ty
    }

    fn visit_self(&mut self, expr: &mut SelfExpr) -> Self::Result {
        if let Some(ty) = self.lookup_var("self") {
            expr.ty = Some(ty.clone());
            ty
        } else if let Some(type_name) = &self.current_type {
            // Fallback: si no se declaró (por algún error), usar el tipo actual
            let ty = HulkType::Class(type_name.clone());
            expr.ty = Some(ty.clone());
            ty
        } else {
            self.add_type_error("'self' used outside of method".to_string(), expr.span);
            HulkType::Error
        }
    }
    
    fn visit_base(&mut self, e: &mut BaseExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_attribute_access(&mut self, expr: &mut AttributeAccessExpr) -> Self::Result {
        let obj_ty = expr.object.accept(self);
        // Verificar que el acceso sea permitido (solo si el objeto es `self` y estamos dentro de un método)
        let is_self = match &*expr.object {
            Expr::SelfExpr(_) => true,
            _ => false,
        };
        if !is_self {
            self.add_type_error("Attribute access is only allowed on 'self'".to_string(), expr.span);
            return HulkType::Error;
        }
        // Buscar el atributo en el tipo actual
        if let Some(type_name) = &self.current_type {
            if let Some(type_info) = self.types.get(type_name) {
                if let Some(attr_ty) = type_info.attributes.get(&expr.attribute) {
                    expr.ty = Some(attr_ty.clone());
                    return attr_ty.clone();
                }
            }
        }
        self.add_type_error(format!("Attribute '{}' not found", expr.attribute), expr.span);
        HulkType::Error
    }
    
    fn visit_protocol_def(&mut self, proto: &mut ProtocolDef) -> Self::Result {
        todo!()
    }
    
    fn visit_protocol_method(&mut self, method: &mut ProtocolMethod) -> Self::Result {
        todo!()
    }
}
