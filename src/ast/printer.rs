use crate::ast::*;
use crate::semantic::types::HulkType;

pub struct PrettyPrinter {
    indent: usize,
    output: String,
}

impl PrettyPrinter {
    pub fn new() -> Self {
        Self { indent: 0, output: String::new() }
    }

    fn write_line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn type_str(ty: &Option<HulkType>) -> String {
        match ty {
            Some(t) => format!(" : {:?}", t),
            None => String::new(),
        }
    }

    fn type_opt(ty: &Option<HulkType>) -> String {
        match ty {
            Some(t) => format!("{:?}", t),
            None => "?".to_string(),
        }
    }

    pub fn into_string(self) -> String {
        self.output
    }
}

impl Visitor for PrettyPrinter {
    type Result = ();

    fn visit_program(&mut self, program: &mut Program) {
        self.write_line("Program {");
        self.indent += 1;

        // Print protocols
        for proto in &mut program.protocols {
            proto.accept(self);
        }

        // Print types
        for ty in &mut program.types {
            ty.accept(self);
        }

        // Print functions
        for func in &mut program.functions {
            func.accept(self);
        }

        // Print main expression
        self.write_line("main_expr:");
        self.indent += 1;
        program.main_expr.accept(self);
        self.indent -= 1;

        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_function_def(&mut self, func: &mut FunctionDef) {
        // Construir representación de parámetros
        let params_str: Vec<String> = func.params.iter()
            .map(|p| {
                let ann = match &p.ty_annotation {
                    Some(t) => format!(": {:?}", t),
                    None => String::new(),
                };
                let inf = match &p.ty {
                    Some(t) => format!(" [infer: {:?}]", t),
                    None => String::new(),
                };
                format!("{}{}{}", p.name, ann, inf)
            })
            .collect();
        let params_display = params_str.join(", ");

        // Anotación de retorno
        let ret_ann = match &func.ty_annotation {
            Some(t) => format!(" -> {:?}", t),
            None => String::new(),
        };
        // Tipo inferido de retorno
        let ret_inf = match &func.ty {
            Some(t) => format!(" [infer: {:?}]", t),
            None => String::new(),
        };

        self.write_line(&format!("FunctionDef {{ name: '{}', generic: {}, params: [{}]{}{}", 
            func.name, func.is_generic, params_display, ret_ann, ret_inf));
        self.indent += 1;
        self.write_line("body:");
        self.indent += 1;
        func.body.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_number(&mut self, n: &mut NumberExpr) {
        let ty_str = Self::type_str(&n.ty);
        self.write_line(&format!("Number({}){}", n.value, ty_str));
    }

    fn visit_binary_op(&mut self, b: &mut BinaryOpExpr) {
        let ty_str = Self::type_str(&b.ty);
        self.write_line(&format!("BinaryOp {{ op: {:?}{}", b.op, ty_str));
        self.indent += 1;
        self.write_line("left:");
        self.indent += 1;
        b.left.accept(self);
        self.indent -= 1;
        self.write_line("right:");
        self.indent += 1;
        b.right.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_string(&mut self, expr: &mut StringExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("String({:?}){}", expr.value, ty_str));
    }

    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("Call({}, args: [{}", expr.func, ty_str));
        self.indent += 1;
        for (i, arg) in expr.args.iter_mut().enumerate() {
            if i > 0 {
                self.write_line(",");
            }
            arg.accept(self);
        }
        if !expr.args.is_empty() {
            self.write_line("");
        }
        self.indent -= 1;
        self.write_line("])");
    }

    fn visit_const(&mut self, expr: &mut ConstExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("Const({}){}", expr.name, ty_str));
    }

    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("Bool({}){}", expr.value, ty_str));
    }

    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result {
        let op_name = match expr.op {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "-",
        };
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("UnaryOp({}){}", op_name, ty_str));
        self.indent += 1;
        expr.expr.accept(self);
        self.indent -= 1;
    }

    fn visit_variable(&mut self, expr: &mut VariableExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("Variable({}){}", expr.name, ty_str));
    }

    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result {
        self.write_line("Let {");
        self.indent += 1;
        self.write_line("bindings: [");
        self.indent += 1;
        for (name, ty_ann, value) in &mut expr.bindings {
            let ty_str = match ty_ann {
                Some(t) => format!(": {:?}", t),
                None => String::new(),
            };
            self.write_line(&format!("{}{} =", name, ty_str));
            self.indent += 1;
            value.accept(self);
            self.indent -= 1;
        }
        self.indent -= 1;
        self.write_line("]");
        self.write_line("body:");
        self.indent += 1;
        expr.body.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> Self::Result {
        self.write_line("Assign {");
        self.indent += 1;
        self.write_line("lhs:");
        self.indent += 1;
        expr.lhs.accept(self);
        self.indent -= 1;
        self.write_line("value:");
        self.indent += 1;
        expr.value.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("Block {{{}", ty_str));
        self.indent += 1;
        for e in &mut expr.expressions {
            e.accept(self);
        }
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("If {{{}", ty_str));
        self.indent += 1;
        self.write_line("condition:");
        self.indent += 1;
        expr.condition.accept(self);
        self.indent -= 1;
        self.write_line("then:");
        self.indent += 1;
        expr.then_branch.accept(self);
        self.indent -= 1;
        self.write_line("else:");
        self.indent += 1;
        expr.else_branch.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("While {{{}", ty_str));
        self.indent += 1;
        self.write_line("condition:");
        self.indent += 1;
        expr.condition.accept(self);
        self.indent -= 1;
        self.write_line("body:");
        self.indent += 1;
        expr.body.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }
    
    fn visit_type_def(&mut self, ty: &mut TypeDef) {
        let parent_str = match &ty.parent {
            Some(parent) => format!(" inherits {}(...)", parent.name),
            None => String::new(),
        };
        self.write_line(&format!("TypeDef {{ name: '{}'{}", ty.name, parent_str));
        self.indent += 1;
        
        // Attributes
        if !ty.attributes.is_empty() {
            self.write_line("attributes: [");
            self.indent += 1;
            for attr in &mut ty.attributes {
                attr.accept(self);
            }
            self.indent -= 1;
            self.write_line("]");
        }

        // Methods
        if !ty.methods.is_empty() {
            self.write_line("methods: [");
            self.indent += 1;
            for method in &mut ty.methods {
                method.accept(self);
            }
            self.indent -= 1;
            self.write_line("]");
        }
        
        self.indent -= 1;
        self.write_line("}");
    }
    
    fn visit_attribute(&mut self, attr: &mut Attribute) -> Self::Result {
        let ann_str = match &attr.ty_annotation {
            Some(ty) => format!(": {:?}", ty),
            None => String::new(),
        };
        let inf_str = match &attr.ty {
            Some(ty) => format!(" [infer: {:?}]", ty),
            None => String::new(),
        };
        self.write_line(&format!("Attribute {{ name: '{}{}{}', init:", attr.name, ann_str, inf_str));
        self.indent += 1;
        attr.init_expr.accept(self);
        self.indent -= 1;
        self.write_line("}");
    }
    
    fn visit_method(&mut self, m: &mut Method) -> Self::Result {
        let type_info = match &m.type_name {
            Some(t) => format!(" (in type '{}')", t),
            None => String::new(),
        };
        let params_str: Vec<String> = m.params.iter()
            .map(|p| {
                let ann = match &p.ty_annotation {
                    Some(ty) => format!(": {:?}", ty),
                    None => String::new(),
                };
                let inf = match &p.ty {
                    Some(ty) => format!(" [infer: {:?}]", ty),
                    None => String::new(),
                };
                format!("{}{}{}", p.name, ann, inf)
            })
            .collect();
        let ret_ann = match &m.ty_annotation {
            Some(ty) => format!(" -> {:?}", ty),
            None => String::new(),
        };
        let ret_inf = match &m.ty {
            Some(ty) => format!(" [infer: {:?}]", ty),
            None => String::new(),
        };
        self.write_line(&format!("Method {{ name: '{}'{}, params: [{}]{}{}", 
            m.name, type_info, params_str.join(", "), ret_ann, ret_inf));
        self.indent += 1;
        self.write_line("body:");
        self.indent += 1;
        m.body.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }
    
    fn visit_new(&mut self, e: &mut NewExpr) -> Self::Result {
        self.write_line(&format!("New {{ type: '{}', args: [", e.type_name));
        self.indent += 1;
        for (i, arg) in e.args.iter_mut().enumerate() {
            if i > 0 {
                self.write_line(",");
            }
            arg.accept(self);
        }
        if !e.args.is_empty() {
            self.write_line("");
        }
        self.indent -= 1;
        self.write_line("] }");
    }
    
    fn visit_method_call(&mut self, e: &mut MethodCallExpr) -> Self::Result {
        self.write_line("MethodCall {");
        self.indent += 1;
        self.write_line("object:");
        self.indent += 1;
        e.object.accept(self);
        self.indent -= 1;
        self.write_line(&format!("method: '{}', args: [", e.method));
        self.indent += 1;
        for (i, arg) in e.args.iter_mut().enumerate() {
            if i > 0 {
                self.write_line(",");
            }
            arg.accept(self);
        }
        if !e.args.is_empty() {
            self.write_line("");
        }
        self.indent -= 1;
        self.write_line("]");
        self.indent -= 1;
        self.write_line("}");
    }
    
    fn visit_self(&mut self, e: &mut SelfExpr) -> Self::Result {
        self.write_line("Self");
    }

    fn visit_base(&mut self, expr: &mut BaseExpr) -> Self::Result {
        let base_info = match (&expr.base_type, &expr.method_name) {
            (Some(bt), Some(mn)) => format!(" (base_type: {}, method: {})", bt, mn),
            _ => String::new(),
        };
        let ty_str = Self::type_str(&expr.ty);
        self.write_line(&format!("Base{}{}", base_info, ty_str));
    }
    
    fn visit_attribute_access(&mut self, e: &mut AttributeAccessExpr) -> Self::Result {
        self.write_line("AttributeAccess {");
        self.indent += 1;
        self.write_line("object:");
        self.indent += 1;
        e.object.accept(self);
        self.indent -= 1;
        self.write_line(&format!("attribute: '{}'", e.attribute));
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_protocol_def(&mut self, proto: &mut ProtocolDef) -> Self::Result {
        let extends_str = match &proto.extends {
            Some(parent) => format!(" extends {}", parent),
            None => String::new(),
        };
        self.write_line(&format!("ProtocolDef {{ name: '{}'{}", proto.name, extends_str));
        self.indent += 1;
        if !proto.methods.is_empty() {
            self.write_line("methods: [");
            self.indent += 1;
            for m in &mut proto.methods {
                m.accept(self);
            }
            self.indent -= 1;
            self.write_line("]");
        }
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_protocol_method(&mut self, m: &mut ProtocolMethod) -> Self::Result {
        let params_str: Vec<String> = m.params.iter()
            .map(|p| {
                let ann = match &p.ty_annotation {
                    Some(ty) => format!(": {:?}", ty),
                    None => String::new(),
                };
                format!("{}{}", p.name, ann)
            })
            .collect();
        let ret_str = match &m.return_ty {
            Some(ty) => format!(": {:?}", ty),
            None => String::new(),
        };
        self.write_line(&format!("ProtocolMethod {{ {}({}){} }}", m.name, params_str.join(", "), ret_str));
    }

    
}