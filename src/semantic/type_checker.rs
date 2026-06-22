use crate::ast::expr::UnaryOp;
use crate::ast::*;
use crate::error::CompilerError;
use super::types::HulkType;
use crate::error::Span;
use std::collections::{HashMap, HashSet};
use crate::semantic::inference::Unifier;
use crate::semantic::inference::{Constraint};

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
    own_attributes: HashSet<String>,
}

#[derive(Clone)]
struct MethodInfo {
    param_types: Vec<HulkType>,
    return_type: HulkType,
}

#[derive(Clone)]
pub struct FlattenedMethod {
    pub method: Method,              
    pub vtable_index: usize,        
    pub defining_type: String,
}

#[derive(Clone, Default)]
pub struct FlattenedType {
    pub type_id: u32, 
    pub attributes: Vec<Attribute>,       // Atributos en orden (padre → hijo)
    pub methods: Vec<FlattenedMethod>,    // Métodos efectivos (con índices VTable)
    pub parent_name: Option<String>,      // Nombre del padre directo
    pub params: Vec<(String, HulkType)>,  // Parámetros efectivos (nombre, tipo)
    pub parent_init_args: Option<Vec<Box<Expr>>>, // Argumentos para el constructor padre (si el hijo tiene params propios)
}

#[derive(Clone)]
struct ProtocolInfo {
    methods: HashMap<String, MethodInfo>, // firma de los métodos
    extends: Option<String>,              // protocolo padre
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
    flattened_types: HashMap<String, FlattenedType>,
    type_defs: HashMap<String, TypeDef>,
    current_method: Option<String>,
    protocols: HashMap<String, ProtocolInfo>,
    current_method_params: Option<Vec<HulkType>>,
    next_type_id: u32,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(HashMap::new()); // Scope global
        let mut tc = Self {
            errors: Vec::new(),
            scopes,
            functions: HashMap::new(),
            unifier: Unifier::default(),
            param_vars: HashMap::new(),
            return_var: HashMap::new(),
            types: HashMap::new(),
            current_type: None,
            self_type: None,
            flattened_types: HashMap::new(),
            type_defs: HashMap::new(),
            current_method: None,
            protocols: HashMap::new(),
            current_method_params: None,
            next_type_id: 0,
        };
        tc.register_builtin_types();
        tc
    }

    pub fn check(&mut self, program: &mut Program) -> Result<(), Vec<CompilerError>> {
        // 1. Análisis semántico
        self.visit_program(program);
        
        // Si hay errores, retornarlos inmediatamente
        if !self.errors.is_empty() {
            return Err(self.errors.clone());
        }

        // 2. Resolver variables de tipo en el AST
        self.resolve_ast(program);

        // 3. Aplanar todos los tipos
        self.flatten_all_types();

        // 4. Resolver variables de tipo en los flattened types
        self.resolve_flattened_types();

        // 5. Verificar que no queden variables de tipo sin resolver en el programa
        if let Err(errors) = self.verify_no_type_vars(program) {
            return Err(errors);
        }

        Ok(())
    }

    fn register_builtin_types(&mut self) {
        // Insertar Object en self.types
        if !self.types.contains_key("Object") {
            self.types.insert("Object".to_string(), TypeInfo {
                parent: None,
                param_vars: vec![],
                attributes: HashMap::new(),
                attr_order: vec![],
                methods: HashMap::new(),
                own_attributes: HashSet::new(), 
            });
            // También insertar un TypeDef ficticio para Object
            self.type_defs.insert("Object".to_string(), TypeDef {
                name: "Object".to_string(),
                params: vec![],
                param_types: vec![],
                parent: None,
                attributes: vec![],
                methods: vec![],
                span: Span::new(0, 0),
                ty: None,
            });
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
                Expr::Is(is) => {
                    if let Some(ty) = &is.ty {
                        is.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut is.expr);
                }
                Expr::As(as_expr) => {
                    if let Some(ty) = &as_expr.ty {
                        as_expr.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut as_expr.expr);
                }
            }
        }

        // Resolver tipos en funciones
        for func in &mut program.functions {
            if let Some(ty) = &func.ty {
                func.ty = Some(self.resolve_and_normalize(ty));
            }
            for param in &mut func.params {
                if let Some(ty) = &param.ty {
                    param.ty = Some(self.resolve_and_normalize(ty));
                }
            }
            resolve_expr(&self.unifier, &mut func.body);
        }

        // Resolver tipos en definiciones de tipos
        for type_def in &mut program.types {
            if let Some(ty) = &type_def.ty {
                type_def.ty = Some(self.resolve_and_normalize(ty));
            }
            for attr in &mut type_def.attributes {
                if let Some(ty) = &attr.ty {
                    attr.ty = Some(self.resolve_and_normalize(ty));
                }
                if let Some(ann) = &attr.ty_annotation {
                    attr.ty_annotation = Some(self.unifier.resolve(ann));
                }
                resolve_expr(&self.unifier, &mut attr.init_expr);
            }
            for method in &mut type_def.methods {
                if let Some(ty) = &method.ty {
                    method.ty = Some(self.resolve_and_normalize(ty));
                }
                for param in &mut method.params {
                    if let Some(ty) = &param.ty {
                        param.ty = Some(self.resolve_and_normalize(ty));
                    }
                    if let Some(ann) = &param.ty_annotation {
                        param.ty_annotation = Some(self.unifier.resolve(ann));
                    }
                }
                // Actualizar MethodInfo en self.types con los tipos resueltos
                if let Some(type_info) = self.types.get_mut(&type_def.name) {
                    if let Some(m_info) = type_info.methods.get_mut(&method.name) {
                        m_info.param_types = method.params.iter()
                            .map(|p| p.ty.as_ref().unwrap().clone())
                            .collect();
                        m_info.return_type = method.ty.as_ref().unwrap().clone();
                    }
                }
                resolve_expr(&self.unifier, &mut method.body);
            }
            // Después de resolver atributos y métodos, resolver param_types
            type_def.param_types = type_def.param_types.iter()
                .map(|t| self.unifier.resolve(t))
                .collect();

            self.type_defs.insert(type_def.name.clone(), type_def.clone());
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

    pub fn flatten_all_types(&mut self) {
        // Primero, asegurar que Object está aplanado (base)
        self.flatten_type("Object");

        // Luego aplanar todos los tipos definidos por el usuario
        let type_names: Vec<String> = self.types.keys().cloned().collect();
        for name in type_names {
            if name != "Object" {
                self.flatten_type(&name);
            }
        }
    }

    fn flatten_type(&mut self, type_name: &str) -> FlattenedType {
        if let Some(flattened) = self.flattened_types.get(type_name) {
            return flattened.clone();
        }

        let type_info = self.types.get(type_name)
            .expect("TypeInfo not found")
            .clone();
        let parent_name = type_info.parent.clone();

        let parent_flattened = if let Some(ref pname) = parent_name {
            self.flatten_type(pname)
        } else {
            FlattenedType::default()
        };

        let type_def = self.type_defs.get(type_name)
            .expect("TypeDef not found")
            .clone();

        // --- Determinar parámetros efectivos y argumentos del padre ---
        let (effective_params, parent_init_args) = if type_def.params.is_empty() {
        // Sin parámetros propios: heredar los del padre (ya resueltos en parent_flattened)
        (parent_flattened.params.clone(), None)
        } else {
            // Con parámetros propios: determinar si hay especialización
            let own_params: Vec<(String, HulkType)> = type_def.params
                .iter()
                .zip(type_def.param_types.iter())
                .map(|(name, ty)| (name.clone(), Self::resolve_type_for_flatten(&self.unifier, ty)))
                .collect();

            let has_specialization = type_def.parent.as_ref()
                .map(|p| !p.args.is_empty())
                .unwrap_or(false);

            if has_specialization {
                let args = type_def.parent.as_ref().unwrap().args.clone();
                (own_params, Some(args))
            } else {
                let mut combined = parent_flattened.params.clone();
                combined.extend(own_params);
                (combined, None)
            }
        };
        // --- Construir atributos efectivos ---
        let mut effective_attributes = Vec::new();

        // Si hay especialización, transformar los atributos del padre
        if let Some(args) = &parent_init_args {
            // Construir mapa de sustitución: parámetro del padre -> expresión
            let mut subst = HashMap::new();
            for ((param_name, _), arg_expr) in parent_flattened.params.iter().zip(args.iter()) {
                subst.insert(param_name.clone(), arg_expr.clone());
            }

            // Transformar cada atributo del padre
            for attr in &parent_flattened.attributes {
                let mut new_attr = attr.clone();
                // Sustituir en la expresión de inicialización
                self.substitute_expr(&mut new_attr.init_expr, &subst);
                effective_attributes.push(new_attr);
            }
        } else {
            // Sin especialización: copiar atributos del padre tal cual
            effective_attributes.extend(parent_flattened.attributes.clone());
        }

        // Añadir atributos propios del hijo (no se sustituyen, usan sus propias expresiones)
        for attr in &type_def.attributes {
            // Verificar duplicados
            if effective_attributes.iter().any(|a| a.name == attr.name) {
                self.add_type_error(
                    format!("Attribute '{}' already defined in parent", attr.name),
                    attr.span,
                );
            } else {
                effective_attributes.push(attr.clone());
            }
        }

        // --- Construir métodos (igual que antes) ---
        let mut methods = parent_flattened.methods.clone();
        for method in &type_def.methods {
            if let Some(existing_pos) = methods.iter().position(|m| {
                m.method.name == method.name && m.method.params.len() == method.params.len()
            }) {
                let old_index = methods[existing_pos].vtable_index;
                methods[existing_pos] = FlattenedMethod {
                    method: method.clone(),
                    vtable_index: old_index,
                    defining_type: type_def.name.clone(),
                };
            } else {
                let new_index = methods.len();
                methods.push(FlattenedMethod {
                    method: method.clone(),
                    vtable_index: new_index,
                    defining_type: type_def.name.clone(),
                });
            }
        }

        // --- Construir el FlattenedType final ---
        let flattened = FlattenedType {
            attributes: effective_attributes,
            methods,
            parent_name: parent_name.clone(),
            params: effective_params,
            parent_init_args, // ya lo tenemos
            type_id: self.next_type_id,
        };
        self.next_type_id += 1;
        self.flattened_types.insert(type_name.to_string(), flattened.clone());
        flattened
    }

    pub fn get_type_def(&self, name: &str) -> Option<&TypeDef> {
        self.type_defs.get(name)
    }

    pub fn get_flattened_types(&self) -> &HashMap<String, FlattenedType> {
        &self.flattened_types
    }

    fn resolve_annotation(&self, ty: &HulkType) -> HulkType {
        match ty {
            HulkType::UserDefined(name) => {
                if self.types.contains_key(name) {
                    HulkType::Class(name.clone())
                } else if self.protocols.contains_key(name) {
                    HulkType::Protocol(name.clone())
                } else {
                    HulkType::Error
                }
            }
            _ => ty.clone(),
        }
    }

    fn is_subtype(&self, sub: &HulkType, sup: &HulkType) -> bool {
        match (sub, sup) {
            // Las variables de tipo son compatibles con todo (se resolverán después)
            (HulkType::Var(_), _) => true,
            (_, HulkType::Var(_)) => true,
            // Object es supertipo de todo
            (_, HulkType::Object) => true,
            // Jerarquía de clases
            (HulkType::Class(sub_name), HulkType::Class(sup_name)) => {
                if sub_name == sup_name { return true; }
                let mut current = sub_name.clone();
                while let Some(info) = self.types.get(&current) {
                    if let Some(parent) = &info.parent {
                        if parent == sup_name { return true; }
                        current = parent.clone();
                    } else {
                        break;
                    }
                }
                false
            }
            // Tipos concretos deben ser iguales
            (a, b) => a == b,
        }
    }

    fn type_conforms_to_protocol(&self, class_name: &str, protocol_name: &str) -> bool {
        let proto = match self.protocols.get(protocol_name) {
            Some(p) => p,
            None => return false,
        };
        let type_info = match self.types.get(class_name) {
            Some(t) => t,
            None => return false,
        };

        for (method_name, proto_method) in &proto.methods {
            let class_method = match type_info.methods.get(method_name) {
                Some(m) => m,
                None => {
                    // Buscar en la jerarquía de padres (métodos heredados)
                    let mut parent = type_info.parent.clone();
                    let mut found = None;
                    while let Some(pname) = parent {
                        if let Some(pinfo) = self.types.get(&pname) {
                            if let Some(m) = pinfo.methods.get(method_name) {
                                found = Some(m);
                                break;
                            }
                            parent = pinfo.parent.clone();
                        } else {
                            break;
                        }
                    }
                    match found {
                        Some(m) => m,
                        None => return false,
                    }
                }
            };

            if class_method.param_types.len() != proto_method.param_types.len() {
                return false;
            }

            // Contravariante: parámetros de la clase deben ser supertipos de los del protocolo
            for (class_p, proto_p) in class_method.param_types.iter().zip(proto_method.param_types.iter()) {
                if !self.conforms_to(proto_p, class_p) {
                    return false;
                }
            }

            // Covariante: retorno de la clase debe ser subtipo del retorno del protocolo
            if !self.conforms_to(&class_method.return_type, &proto_method.return_type) {
                return false;
            }
        }

        true
    }

    /// Determina si `sub` conforma a `sup` según la jerarquía de tipos de HULK.
    pub fn conforms_to(&self, sub: &HulkType, sup: &HulkType) -> bool {
        // 1. Error no conforma a nada (ni siquiera a sí mismo)
        if matches!(sub, HulkType::Error) || matches!(sup, HulkType::Error) {
            return false;
        }
        // 2. Variables de tipo: durante la inferencia, son compatibles con todo
        if matches!(sub, HulkType::Var(_)) || matches!(sup, HulkType::Var(_)) {
            return true;
        }
        // 3. Object es supertipo de todo
        if matches!(sup, HulkType::Object) {
            return true;
        }
        // 4. Tipos primitivos (Number, String, Boolean): solo conforman a sí mismos
        match (sub, sup) {
            (HulkType::Number, HulkType::Number) => true,
            (HulkType::String, HulkType::String) => true,
            (HulkType::Boolean, HulkType::Boolean) => true,
            // 5. Clases nominales
            (HulkType::Class(sub_name), HulkType::Class(sup_name)) => {
                if sub_name == sup_name { return true; }
                // Recorrer cadena de herencia
                let mut current = sub_name.clone();
                while let Some(info) = self.types.get(&current) {
                    if let Some(parent) = &info.parent {
                        if parent == sup_name { return true; }
                        current = parent.clone();
                    } else {
                        break;
                    }
                }
                false
            }
            // 6. Protocolos
            (HulkType::Protocol(sub_name), HulkType::Protocol(sup_name)) => {
                if sub_name == sup_name { return true; }
                let mut current = sub_name.clone();
                while let Some(info) = self.protocols.get(&current) {
                    if let Some(parent) = &info.extends {
                        if parent == sup_name { return true; }
                        current = parent.clone();
                    } else {
                        break;
                    }
                }
                false
            }
            // 7. Una clase puede conformar a un protocolo si lo implementa
            (HulkType::Class(class_name), HulkType::Protocol(proto_name)) => {
                self.type_conforms_to_protocol(class_name, proto_name)
            }
            // 8. Un protocolo no conforma a una clase (excepto Object, ya cubierto)
            _ => false,
        }
    }

    /// Devuelve la lista de ancestros de una clase (desde ella misma hasta Object).
    fn get_ancestors(&self, class_name: &str) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current = class_name.to_string();
        ancestors.push(current.clone());
        while let Some(info) = self.types.get(&current) {
            if let Some(parent) = &info.parent {
                ancestors.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }
        ancestors
    }

    /// Calcula el tipo más específico al que ambos tipos conforman.
    pub fn lowest_common_ancestor(&self, t1: &HulkType, t2: &HulkType) -> Option<HulkType> {
        // Si son iguales, devolver ese tipo
        if t1 == t2 { return Some(t1.clone()); }
        // Si uno conforma al otro, el LCA es el supertipo
        if self.conforms_to(t1, t2) { return Some(t2.clone()); }
        if self.conforms_to(t2, t1) { return Some(t1.clone()); }

        match (t1, t2) {
            (HulkType::Class(c1), HulkType::Class(c2)) => {
                let anc1 = self.get_ancestors(c1);
                let anc2 = self.get_ancestors(c2);
                // Buscar el primer ancestro común desde la raíz (Object) hacia abajo
                for a in anc1 {
                    if anc2.contains(&a) {
                        return Some(HulkType::Class(a));
                    }
                }
                // Si no, Object (pero Object no es clase, es tipo primitivo)
                Some(HulkType::Object)
            }
            (HulkType::Protocol(p1), HulkType::Protocol(p2)) => {
                // Similar para protocolos (menos común, pero se puede implementar)
                // Por simplicidad, devolvemos Protocol(p1) si p1 extiende a p2 o viceversa
                // O simplemente Object si no hay relación
                Some(HulkType::Object)
            }
            _ => Some(HulkType::Object) // Por defecto Object
        }
    }

    fn substitute_expr(&self, expr: &mut Expr, subst: &HashMap<String, Box<Expr>>) {
        match expr {
            Expr::Variable(var) => {
                if let Some(replacement) = subst.get(&var.name) {
                    // Reemplazar la variable por la expresión clonada
                    *expr = replacement.as_ref().clone();
                }
            }
            Expr::BinaryOp(b) => {
                self.substitute_expr(&mut b.left, subst);
                self.substitute_expr(&mut b.right, subst);
            }
            Expr::Call(call) => {
                for arg in &mut call.args {
                    self.substitute_expr(arg, subst);
                }
            }
            Expr::UnaryOp(u) => {
                self.substitute_expr(&mut u.expr, subst);
            }
            Expr::Let(let_expr) => {
                for (_, _, init) in &mut let_expr.bindings {
                    self.substitute_expr(init, subst);
                }
                self.substitute_expr(&mut let_expr.body, subst);
            }
            Expr::DestructiveAssign(a) => {
                self.substitute_expr(&mut a.lhs, subst);
                self.substitute_expr(&mut a.value, subst);
            }
            Expr::Block(b) => {
                for e in &mut b.expressions {
                    self.substitute_expr(e, subst);
                }
            }
            Expr::If(i) => {
                self.substitute_expr(&mut i.condition, subst);
                self.substitute_expr(&mut i.then_branch, subst);
                self.substitute_expr(&mut i.else_branch, subst);
            }
            Expr::While(w) => {
                self.substitute_expr(&mut w.condition, subst);
                self.substitute_expr(&mut w.body, subst);
            }
            Expr::New(n) => {
                for arg in &mut n.args {
                    self.substitute_expr(arg, subst);
                }
            }
            Expr::MethodCall(m) => {
                self.substitute_expr(&mut m.object, subst);
                for arg in &mut m.args {
                    self.substitute_expr(arg, subst);
                }
            }
            Expr::AttributeAccess(a) => {
                self.substitute_expr(&mut a.object, subst);
            }
            // Los nodos que no contienen subexpresiones (Number, String, Bool, Const, Self, Base) no necesitan acción.
            _ => {}
        }
    }

    pub fn resolve_flattened_types(&mut self) {
        // Función auxiliar para resolver tipos en una expresión (similar a la de resolve_ast)
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
                Expr::Is(is) => {
                    if let Some(ty) = &is.ty {
                        is.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut is.expr);
                }
                Expr::As(as_expr) => {
                    if let Some(ty) = &as_expr.ty {
                        as_expr.ty = Some(unifier.resolve(ty));
                    }
                    resolve_expr(unifier, &mut as_expr.expr);
                }
            }
        }

        // Resolver cada FlattenedType
        for (_, flat) in self.flattened_types.iter_mut() {
            flat.params = flat.params.iter()
                .map(|(name, ty)| (name.clone(), Self::resolve_type_for_flatten(&self.unifier, ty)))
                .collect();

            for attr in &mut flat.attributes {
                if let Some(ty) = &attr.ty {
                    attr.ty = Some(Self::resolve_type_for_flatten(&self.unifier, ty));
                }
                if let Some(ann) = &attr.ty_annotation {
                    attr.ty_annotation = Some(Self::resolve_type_for_flatten(&self.unifier, ann));
                }
                resolve_expr(&self.unifier, &mut attr.init_expr);
            }

            for method in &mut flat.methods {
                if let Some(ty) = &method.method.ty {
                    method.method.ty = Some(Self::resolve_type_for_flatten(&self.unifier, ty));
                }
                for param in &mut method.method.params {
                    if let Some(ty) = &param.ty {
                        param.ty = Some(Self::resolve_type_for_flatten(&self.unifier, ty));
                    }
                    if let Some(ann) = &param.ty_annotation {
                        param.ty_annotation = Some(Self::resolve_type_for_flatten(&self.unifier, ann));
                    }
                }
                resolve_expr(&self.unifier, &mut method.method.body);
            }
        }
    }

    fn resolve_type_for_flatten(unifier: &Unifier, ty: &HulkType) -> HulkType {
        let resolved = unifier.resolve(ty);
        if let HulkType::Object = resolved {
            HulkType::Class("Object".to_string())
        } else {
            resolved
        }
    }

    fn resolve_and_normalize(&self, ty: &HulkType) -> HulkType {
        let resolved = self.unifier.resolve(ty);
        match resolved {
            HulkType::Object => HulkType::Class("Object".to_string()),
            _ => resolved,
        }
    }

    fn unify_string_or_number(&mut self, ty: &HulkType, span: Span) {
        // Si es variable de tipo, unificar con String primero (más general)
        if let HulkType::Var(_) = ty {
            if let Err(_) = self.unifier.unify(ty, &HulkType::String) {
                if let Err(msg) = self.unifier.unify(ty, &HulkType::Number) {
                    self.add_type_error("Operand must be String or Number".to_string(), span);
                }
            }
        } else {
            // Si es tipo concreto, verificar compatibilidad
            let ok = ty.is_compatible_with(&HulkType::String) ||
                    ty.is_compatible_with(&HulkType::Number);
            if !ok {
                self.add_type_error("Operand must be String or Number".to_string(), span);
            }
        }
    }

    fn add_string_or_number_constraint(&mut self, ty: &HulkType, span: Span) {
        if let HulkType::Var(id) = ty {
            self.unifier.add_constraint(*id, Constraint::StringOrNumber);
        } else {
            // Para tipos concretos, verificar compatibilidad inmediata
            let ok = ty.is_compatible_with(&HulkType::String) ||
                    ty.is_compatible_with(&HulkType::Number);
            if !ok {
                self.add_type_error("Operand must be String or Number".to_string(), span);
            }
        }
    }
}

impl Visitor for TypeChecker {
    type Result = HulkType;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result {
        self.register_builtin_types();

        // 1. Registrar funciones ANTES de procesar tipos
        for func in &program.functions {
            if self.functions.contains_key(&func.name) {
                self.add_type_error(
                    format!("Duplicate function '{}'", func.name),
                    func.span,
                );
            } else {
                self.functions.insert(func.name.clone(), FunctionInfo {
                    params_len: func.params.len(),
                    param_types: None,
                    return_type: None,
                    is_generic: false,
                });
            }
        }

        // Si hay errores, no continuar
        if !self.errors.is_empty() {
            return HulkType::Error;
        }

        self.prepare_function_vars(program);

        // 2. Procesar tipos (ahora las funciones ya están registradas)
        for type_def in &mut program.types {
            type_def.accept(self);
            self.type_defs.insert(type_def.name.clone(), type_def.clone());
        }

        // 3. Procesar protocolos
        for proto in &mut program.protocols {
            proto.accept(self);
        }

        // 4. Inferir funciones (procesar cuerpos)
        for func in &mut program.functions {
            func.accept(self);
        }

        // 5. Expresión principal
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
                // Para cada operando, si es una variable de tipo, agregar restricción StringOrNumber
                self.add_string_or_number_constraint(&left_ty, expr.left.span());
                self.add_string_or_number_constraint(&right_ty, expr.right.span());

                // El resultado siempre es String
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
                        let mut instantiate = |ty: &HulkType, unifier: &mut Unifier, map: &mut HashMap<usize, HulkType>| -> HulkType {
    let applied = unifier.apply(ty);
    if let HulkType::Var(id) = applied {
        if let Some(new_ty) = map.get(&id) {
            new_ty.clone()
        } else {
            let new_var = unifier.new_var();
            // Obtener y clonar las restricciones
            if let Some(constraints) = unifier.get_constraints(id) {
                let constraints_clone: Vec<Constraint> = constraints.clone();
                if let Some(new_id) = new_var.get_var_id() {
                    for c in constraints_clone {
                        unifier.add_constraint(new_id, c);
                    }
                }
            }
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
                                self.add_type_error(format!("Function '{}' has not been inferred yet", expr.func), expr.span);
                                return HulkType::Error;
                            }
                        } else {
                            self.add_type_error(format!("Function '{}' has not been inferred yet", expr.func), expr.span);
                            return HulkType::Error;
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

                    // Verificar argumentos con conformidad (y también unificar para inferencia)
                    for (arg, expected) in expr.args.iter_mut().zip(param_types.iter()) {
                        let arg_ty = arg.accept(self);
                        // Verificar conformidad
                        if !self.conforms_to(&arg_ty, expected) {
                            self.add_type_error(
                                format!("Argument type {:?} does not conform to parameter type {:?}", arg_ty, expected),
                                arg.span(),
                            );
                        }
                        // Aún unificar para la inferencia (puede ser necesario)
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
            let mut var_ty = init_ty.clone();
            if let Some(ann_ty) = ann {
                let resolved_ann = self.resolve_annotation(ann_ty);
                if !self.conforms_to(&init_ty, &resolved_ann) {
                    self.add_type_error(
                        format!("Cannot initialize variable of type {:?} with value of type {:?}", resolved_ann, init_ty),
                        init_expr.span(),
                    );
                    var_ty = HulkType::Error;
                } else {
                    // Si la anotación es Protocol y init_ty es Class que implementa el protocolo,
                    // reemplazar la anotación por la clase concreta.
                    if let HulkType::Protocol(proto_name) = &resolved_ann {
                        if let HulkType::Class(class_name) = &init_ty {
                            if self.type_conforms_to_protocol(class_name, proto_name) {
                                // Reemplazar la anotación en el AST
                                *ann = Some(HulkType::Class(class_name.clone()));
                                var_ty = init_ty.clone();
                            } else {
                                self.add_type_error(
                                    format!("Type '{}' does not implement protocol '{}'", class_name, proto_name),
                                    expr.span,
                                );
                                var_ty = HulkType::Error;
                            }
                        } else {
                            // La expresión no es una clase concreta; no podemos resolver
                            var_ty = resolved_ann; // mantiene Protocol
                        }
                    } else {
                        var_ty = resolved_ann;
                    }
                }
            } else {
                var_ty = self.unifier.resolve(&init_ty);
            }
            self.declare_var(name.clone(), var_ty);
        }
        let body_ty = expr.body.accept(self);
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
        let value_ty = expr.value.accept(self);

        // Si lhs es de tipo Protocol, verificar conformance del valor
        if let HulkType::Protocol(proto_name) = &lhs_ty {
            if let HulkType::Class(class_name) = &value_ty {
                if !self.type_conforms_to_protocol(class_name, proto_name) {
                    self.add_type_error(
                        format!("Type '{}' does not conform to protocol '{}'", class_name, proto_name),
                        expr.span,
                    );
                }
            } else {
                self.add_type_error("Cannot assign non‑class value to protocol variable".to_string(), expr.span);
            }
            expr.ty = Some(lhs_ty.clone());
            return lhs_ty;
        }

        // Caso normal (clase, tipos concretos)
        // Verificar conformidad (en lugar de is_compatible_with)
        if lhs_ty != HulkType::Error && !self.conforms_to(&value_ty, &lhs_ty) {
            self.add_type_error(
                format!("Cannot assign {:?} to expression of type {:?}", value_ty, lhs_ty),
                expr.span,
            );
        }
        expr.ty = Some(value_ty.clone());
        value_ty
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {
        let mut last_ty = HulkType::Number;
        for e in &mut expr.expressions {
            last_ty = e.accept(self);
        }
        expr.ty = Some(last_ty.clone());
        last_ty
    }

    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        let cond_ty = expr.condition.accept(self);
        if !self.conforms_to(&cond_ty, &HulkType::Boolean) {
            self.add_type_error("If condition must be Boolean".to_string(), expr.condition.span());
        }
        let then_ty = expr.then_branch.accept(self);
        let else_ty = expr.else_branch.accept(self);
        // Calcular LCA
        let result_ty = match self.lowest_common_ancestor(&then_ty, &else_ty) {
            Some(ty) => ty,
            None => {
                self.add_type_error(
                    format!("Incompatible types in if branches: {:?} and {:?}", then_ty, else_ty),
                    expr.span,
                );
                HulkType::Object // fallback
            }
        };
        expr.ty = Some(result_ty.clone());
        result_ty
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        let cond_ty = expr.condition.accept(self);
        if !self.conforms_to(&cond_ty, &HulkType::Boolean) {
            self.add_type_error("While condition must be Boolean".to_string(), expr.condition.span());
        }
        let body_ty = expr.body.accept(self);
        expr.ty = Some(body_ty.clone());
        body_ty
    }

    fn visit_function_def(&mut self, func: &mut FunctionDef) -> Self::Result {
        let param_vars = self.param_vars.get(&func.name).expect("No param vars").clone();
        let ret_var = self.return_var.get(&func.name).expect("No return var").clone();

        self.enter_scope();

        // Primero, declarar los parámetros en el ámbito con sus variables de tipo
        // y **unificar las anotaciones de tipo** inmediatamente.
        for (param, var_ty) in func.params.iter_mut().zip(param_vars.iter()) {
            // Unificar la anotación de tipo (si existe) con la variable de tipo
            if let Some(ann) = &param.ty_annotation {
                let resolved_ann = match ann {
                    HulkType::UserDefined(name) => HulkType::Class(name.clone()),
                    _ => ann.clone(),
                };
                if let Err(msg) = self.unifier.unify(var_ty, &resolved_ann) {
                    self.add_type_error(msg, param.span);
                }
            }
            // Guardar la variable de tipo en el AST (sin resolver aún)
            param.ty = Some(var_ty.clone());
            self.declare_var(param.name.clone(), var_ty.clone());
        }

        // Analizar el cuerpo de la función (ahora 'n' ya está unificado con Number)
        let body_ty = func.body.accept(self);

        // Verificar el tipo de retorno con la anotación (si existe)
        if let Some(ret_ann) = &func.ty_annotation {
            let resolved_ret = self.resolve_annotation(ret_ann);
            if !self.conforms_to(&body_ty, &resolved_ret) {
                self.add_type_error(
                    format!("Return type {:?} does not conform to annotated type {:?}", body_ty, resolved_ret),
                    func.span,
                );
            }
        }

        // Unificar el tipo del cuerpo con la variable de retorno (para inferencia)
        if let Err(msg) = self.unifier.unify(&body_ty, &ret_var) {
            self.add_type_error(msg, func.span);
        }

        // Guardar el tipo de retorno de la función (resuelto después de la unificación)
        func.ty = Some(ret_var.clone());

        // Calcular si la función es genérica
        let is_generic = param_vars.iter().any(|t| {
            matches!(self.unifier.apply(t), HulkType::Var(_))
        }) || matches!(self.unifier.apply(&ret_var), HulkType::Var(_));

        func.is_generic = is_generic;

        // Actualizar la información de la función para las llamadas
        if let Some(info) = self.functions.get_mut(&func.name) {
            info.param_types = Some(param_vars.clone());
            info.return_type = Some(ret_var.clone());
            info.is_generic = is_generic;
        }

        // (Opcional) Ya no necesitas el segundo bucle, porque las anotaciones ya se unificaron.
        // Pero si quieres mantenerlo para otros propósitos, puedes dejarlo vacío.

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
        if let Some(ref pname) = parent_name {
            if pname == "Number" || pname == "String" || pname == "Boolean" {
                self.add_type_error(format!("Cannot inherit from primitive type '{}'", pname), type_def.span);
            }

        }

        // Crear variables de tipo para los parámetros formales
        let param_vars: Vec<HulkType> = if type_def.params.is_empty() && parent_name.is_some() {
            // Heredar parámetros del padre
            if let Some(parent_def) = self.type_defs.get(parent_name.as_ref().unwrap()) {
                // Copiar los nombres de parámetros del padre a type_def.params
                type_def.params = parent_def.params.clone();
                type_def.param_types = parent_def.param_types.clone();
            }
            if let Some(parent_info) = self.types.get(parent_name.as_ref().unwrap()) {
                parent_info.param_vars.clone()
            } else {
                vec![]
            }
        } else {
            (0..type_def.params.len()).map(|_| self.unifier.new_var()).collect()
        };

        // Unificar parámetros formales con sus anotaciones (si existen)
        for (var_ty, ann_ty) in param_vars.iter().zip(type_def.param_types.iter()) {
            // Si ann_ty no es Object, significa que había una anotación explícita
            if !matches!(ann_ty, HulkType::Object) {
                let resolved_ann = self.resolve_annotation(ann_ty);
                if let Err(msg) = self.unifier.unify(var_ty, &resolved_ann) {
                    self.add_type_error(
                        format!("Parameter type mismatch: {}", msg),
                        type_def.span,
                    );
                }
            }
        }

        // Registrar el tipo con sus variables de parámetro
        let mut type_info = TypeInfo {
            parent: parent_name.clone(),
            param_vars: param_vars.clone(),
            attributes: HashMap::new(),
            attr_order: Vec::new(),
            methods: HashMap::new(),
            own_attributes: HashSet::new(), 
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

        // Procesar atributos explícitos (visit_attribute los añade a attributes)
        for attr in &mut type_def.attributes {
            attr.accept(self);
        }

        // Crear atributos implícitos para parámetros sin atributo explícito
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

        // *** AÑADIR: marcar atributos propios en own_attributes ***
        if let Some(type_info) = self.types.get_mut(&type_def.name) {
            for attr in &type_def.attributes {
                type_info.own_attributes.insert(attr.name.clone());
            }
        }

        // *** NUEVO: Visitar argumentos del constructor padre MIENTRAS LOS PARÁMETROS AÚN ESTÁN EN EL ÁMBITO ***
        if let Some(parent) = &mut type_def.parent {
            for arg in &mut parent.args {
                arg.accept(self);
            }
        }

        // Ahora sí, salir del ámbito de parámetros
        self.exit_scope();

        // Procesar métodos (ellos crean su propio ámbito)
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
            // CONVERTIR la anotación a tipo concreto (Class o Protocol)
            let resolved_ann = self.resolve_annotation(ann);
            if let Err(msg) = self.unifier.unify(&init_ty, &resolved_ann) {
                self.add_type_error(msg, attr.span);
            }
        }
        let final_ty = self.resolve_and_normalize(&init_ty);
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
        self.current_method = Some(method.name.clone());
        self.enter_scope();

        let self_ty = HulkType::Class(current_type_name.clone());
        self.declare_var("self".to_string(), self_ty.clone());

        let param_vars: Vec<HulkType> = (0..method.params.len())
            .map(|_| self.unifier.new_var())
            .collect();
        let ret_var = self.unifier.new_var();
        self.current_method_params = Some(param_vars.clone());

        let method_info = MethodInfo {
            param_types: param_vars.clone(),
            return_type: ret_var.clone(),
        };
        if let Some(type_name) = &self.current_type {
            if let Some(type_info) = self.types.get_mut(type_name) {
                type_info.methods.insert(method.name.clone(), method_info);
            }
        }

        for (param, var_ty) in method.params.iter_mut().zip(param_vars.iter()) {
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

        if let Some(ret_ann) = &method.ty_annotation {
            let resolved_ret_ann = match ret_ann {
                HulkType::UserDefined(name) => HulkType::Class(name.clone()),
                _ => ret_ann.clone(),
            };
            if let Err(msg) = self.unifier.unify(&ret_var, &resolved_ret_ann) {
                self.add_type_error(msg, method.span);
            }
        }

        let body_ty = method.body.accept(self);
        if let Err(msg) = self.unifier.unify(&body_ty, &ret_var) {
            self.add_type_error(msg, method.span);
        }

        let resolved_params: Vec<HulkType> = param_vars.iter()
            .map(|v| self.unifier.resolve(v))
            .collect();
        let resolved_ret = self.unifier.resolve(&ret_var);
        method.ty = Some(resolved_ret.clone());

        if let Some(type_name) = &self.current_type {
            if let Some(type_info) = self.types.get_mut(type_name) {
                if let Some(m_info) = type_info.methods.get_mut(&method.name) {
                    m_info.param_types = resolved_params;
                    m_info.return_type = resolved_ret.clone();
                }
            }
        }
        self.current_method = None;
        self.current_method_params = None;
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
        let obj_ty = expr.object.accept(self);

        let type_name = match obj_ty {
            HulkType::Class(name) => name,
            HulkType::Protocol(name) => name,
            _ => {
                self.add_type_error(
                    format!("Method call on non‑class or non‑protocol object (type: {:?})", obj_ty),
                    expr.span,
                );
                return HulkType::Error;
            }
        };

        let method_info = if let Some(proto_info) = self.protocols.get(&type_name) {
            match proto_info.methods.get(&expr.method) {
                Some(m) => m.clone(),
                None => {
                    self.add_type_error(format!("Method '{}' not found in protocol '{}'", expr.method, type_name), expr.span);
                    return HulkType::Error;
                }
            }
        } else if let Some(class_info) = self.types.get(&type_name) {
            match class_info.methods.get(&expr.method) {
                Some(m) => m.clone(),
                None => {
                    self.add_type_error(format!("Method '{}' not found in type '{}'", expr.method, type_name), expr.span);
                    return HulkType::Error;
                }
            }
        } else {
            self.add_type_error(format!("Type or protocol '{}' not found", type_name), expr.span);
            return HulkType::Error;
        };

        if expr.args.len() != method_info.param_types.len() {
            self.add_type_error(
                format!("Method '{}' expects {} arguments", expr.method, method_info.param_types.len()),
                expr.span,
            );
            return HulkType::Error;
        }

        let mut arg_tys = Vec::new();
        for arg in &mut expr.args {
            arg_tys.push(arg.accept(self));
        }

        for (arg_ty, param_ty) in arg_tys.iter().zip(method_info.param_types.iter()) {
            // Verificar conformidad
            if !self.conforms_to(arg_ty, param_ty) {
                self.add_type_error(
                    format!("Argument type {:?} does not conform to parameter type {:?}", arg_ty, param_ty),
                    expr.span,
                );
            }
            // Unificar para inferencia
            if let Err(msg) = self.unifier.unify(arg_ty, param_ty) {
                self.add_type_error(msg, expr.span);
            }
        }

        let ret_ty = method_info.return_type.clone();
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
    
    fn visit_attribute_access(&mut self, expr: &mut AttributeAccessExpr) -> Self::Result {
        let mut obj_ty = expr.object.accept(self);
        if let HulkType::Var(_) = obj_ty {
            obj_ty = self.unifier.resolve(&obj_ty);
        }

        // Determinar si el objeto es "self"
        let is_self = match &*expr.object {
            Expr::SelfExpr(_) => true,
            _ => false,
        };

        if !is_self {
            // Acceso a atributo desde fuera de la clase → error
            self.add_type_error(
                format!("Cannot access attribute '{}' on non-self object", expr.attribute),
                expr.span,
            );
            return HulkType::Error;
        }

        // El objeto es self, obtener la clase actual
        let current_type_name = match self.current_type.as_ref() {
            Some(name) => name,
            None => {
                self.add_type_error("Cannot use self outside a type definition".to_string(), expr.span);
                return HulkType::Error;
            }
        };

        let type_info = match self.types.get(current_type_name) {
            Some(info) => info,
            None => {
                self.add_type_error(format!("Type '{}' not found", current_type_name), expr.span);
                return HulkType::Error;
            }
        };

        // Verificar que el atributo sea propio de la clase actual (no heredado)
        if !type_info.own_attributes.contains(&expr.attribute) {
            self.add_type_error(
                format!(
                    "Attribute '{}' is not defined in this class (cannot access inherited private attributes)",
                    expr.attribute
                ),
                expr.span,
            );
            return HulkType::Error;
        }

        // Ahora buscar el tipo del atributo (puede estar en attributes, que incluye heredados)
        if let Some(attr_ty) = type_info.attributes.get(&expr.attribute) {
            expr.ty = Some(attr_ty.clone());
            return attr_ty.clone();
        }

        // Esto no debería pasar porque own_attributes garantiza que existe
        self.add_type_error(
            format!("Attribute '{}' not found in type '{}'", expr.attribute, current_type_name),
            expr.span,
        );
        HulkType::Error
    }
    
    fn visit_protocol_def(&mut self, proto: &mut ProtocolDef) -> Self::Result {
        if self.protocols.contains_key(&proto.name) {
            self.add_type_error(format!("Duplicate protocol '{}'", proto.name), proto.span);
            return HulkType::Object;
        }

        // Verificar padre
        if let Some(ref parent) = proto.extends {
            if !self.protocols.contains_key(parent) {
                self.add_type_error(format!("Parent protocol '{}' not found", parent), proto.span);
            }
        }

        let mut methods = HashMap::new();
        for m in &proto.methods {
            let param_types: Vec<HulkType> = m.params.iter()
                .map(|p| {
                    p.ty_annotation.as_ref()
                        .map(|ann| self.resolve_annotation(ann))
                        .unwrap_or(HulkType::Error)
                })
                .collect();
            let ret_ty = m.return_ty.as_ref()
                .map(|ann| self.resolve_annotation(ann))
                .unwrap_or(HulkType::Error);
            methods.insert(m.name.clone(), MethodInfo {
                param_types,
                return_type: ret_ty,
            });
        }

        // Si extiende, copiar métodos del padre (sin sobrescritura)
        if let Some(ref parent) = proto.extends {
            // Clonar la información del padre para no mantener un préstamo
            if let Some(parent_info) = self.protocols.get(parent).cloned() {
                for (name, info) in &parent_info.methods {
                    if methods.contains_key(name) {
                        self.add_type_error(
                            format!("Method '{}' already defined in parent protocol", name),
                            proto.span,
                        );
                    } else {
                        methods.insert(name.clone(), info.clone());
                    }
                }
            }
        }

        self.protocols.insert(proto.name.clone(), ProtocolInfo {
            methods,
            extends: proto.extends.clone(),
        });

        HulkType::Object
    }

    fn visit_protocol_method(&mut self, _method: &mut ProtocolMethod) -> Self::Result {
        HulkType::Object
    }
    
    fn visit_base(&mut self, expr: &mut BaseExpr) -> Self::Result {
        let current_type_name = match self.current_type.as_ref() {
            Some(name) => name,
            None => {
                self.add_type_error("'base' used outside of a type".to_string(), expr.span);
                expr.ty = Some(HulkType::Error);
                return HulkType::Error;
            }
        };

        let type_info = match self.types.get(current_type_name) {
            Some(info) => info,
            None => {
                self.add_type_error(format!("Type '{}' not found", current_type_name), expr.span);
                expr.ty = Some(HulkType::Error);
                return HulkType::Error;
            }
        };

        let parent_name = match type_info.parent.as_ref() {
            Some(name) => name,
            None => {
                self.add_type_error(format!("Type '{}' has no parent", current_type_name), expr.span);
                expr.ty = Some(HulkType::Error);
                return HulkType::Error;
            }
        };

        let method_name = match self.current_method.as_ref() {
            Some(name) => name,
            None => {
                self.add_type_error("base used outside method".to_string(), expr.span);
                expr.ty = Some(HulkType::Error);
                return HulkType::Error;
            }
        };

        let current_params = match self.current_method_params.as_ref() {
            Some(params) => params,
            None => {
                self.add_type_error("base used without method params".to_string(), expr.span);
                expr.ty = Some(HulkType::Error);
                return HulkType::Error;
            }
        };

        let parent_info = match self.types.get(parent_name) {
            Some(info) => info,
            None => {
                self.add_type_error(format!("Parent type '{}' not found", parent_name), expr.span);
                expr.ty = Some(HulkType::Error);
                return HulkType::Error;
            }
        };

        // Buscar el método en el padre con el MISMO nombre
        match parent_info.methods.get(method_name) {
            Some(parent_method) => {
                // Verificar cantidad de parámetros
                if parent_method.param_types.len() != current_params.len() {
                    self.add_type_error(
                        format!(
                            "Method '{}' in parent '{}' has {} parameters, but current method has {}",
                            method_name,
                            parent_name,
                            parent_method.param_types.len(),
                            current_params.len()
                        ),
                        expr.span,
                    );
                    expr.ty = Some(HulkType::Error);
                    return HulkType::Error;
                }

                // Verificar tipos de parámetros
                for (i, (parent_p, current_p)) in parent_method.param_types.iter()
                    .zip(current_params.iter()).enumerate() 
                {
                    if let Err(msg) = self.unifier.unify(parent_p, current_p) {
                        self.add_type_error(
                            format!("Parameter {} type mismatch with parent: {}", i + 1, msg),
                            expr.span,
                        );
                        expr.ty = Some(HulkType::Error);
                        return HulkType::Error;
                    }
                }

                // Todo ok: usar el tipo de retorno del padre
                let ret_ty = self.unifier.resolve(&parent_method.return_type);
                expr.ty = Some(ret_ty.clone());
                expr.base_type = Some(parent_name.clone());
                expr.method_name = Some(method_name.clone());
                ret_ty
            }
            None => {
                self.add_type_error(
                    format!("Method '{}' not found in parent type '{}'", method_name, parent_name),
                    expr.span,
                );
                expr.ty = Some(HulkType::Error);
                HulkType::Error
            }
        }
    }
        
    fn visit_is(&mut self, expr: &mut IsExpr) -> Self::Result {
        // 1. Obtener el tipo de la expresión
        let expr_ty = expr.expr.accept(self);

        // 2. Resolver el tipo dado (convertir UserDefined a Class o Protocol)
        let type_given = self.resolve_annotation(&HulkType::UserDefined(expr.type_name.clone()));

        // 3. Si el tipo no se pudo resolver (error), reportar y continuar
        if type_given == HulkType::Error {
            self.add_type_error(
                format!("Type '{}' not found", expr.type_name),
                expr.span,
            );
            expr.ty = Some(HulkType::Boolean);
            return HulkType::Boolean;
        }

        // 4. Asignar tipo Boolean al AST
        expr.ty = Some(HulkType::Boolean);
        HulkType::Boolean
    }

    fn visit_as(&mut self, expr: &mut AsExpr) -> Self::Result {
        // 1. Obtener el tipo de la expresión
        let expr_ty = expr.expr.accept(self);

        // 2. Resolver el tipo dado
        let type_given = self.resolve_annotation(&HulkType::UserDefined(expr.type_name.clone()));

        if type_given == HulkType::Error {
            self.add_type_error(
                format!("Type '{}' not found", expr.type_name),
                expr.span,
            );
            expr.ty = Some(HulkType::Error);
            return HulkType::Error;
        }

        // 3. Verificar que el downcast sea posible estáticamente:
        //    el tipo dado debe ser subtipo del tipo estático de la expresión.
        if !self.conforms_to(&type_given, &expr_ty) {
            self.add_type_error(
                format!(
                    "Cannot downcast expression of type {:?} to type {:?}",
                    expr_ty, type_given
                ),
                expr.span,
            );
            // Aunque haya error, asignamos el tipo dado para no romper el análisis
            expr.ty = Some(type_given.clone());
            return type_given;
        }

        // 4. Asignar el tipo dado como resultado
        expr.ty = Some(type_given.clone());
        type_given
    }

}
