use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;
use crate::ast::{Node, Visitor, Program, ExprStmt, NumberExpr, BinaryOpExpr, PrintExpr, BinOp, Expr, Stmt};

pub struct LlvmCodeGen<'ctx> {
    context: &'ctx Context, 
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    named_values: HashMap<String, PointerValue<'ctx>>,
    current_function: Option<FunctionValue<'ctx>>,
}

impl<'ctx> LlvmCodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            named_values: HashMap::new(),
            current_function: None,
        }
    }

    pub fn compile(&mut self, program: &Program) -> Result<(), String> {
        program.accept(self)?;
        Ok(())
    }

    pub fn write_to_file(&self, filename: &str) -> Result<(), String> {
        self.module.print_to_file(filename).map_err(|e| e.to_string())
    }

    pub fn print_ir(&self) {
        self.module.print_to_stderr();
    }

    fn declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let printf_type = self.context.i32_type().fn_type(&[i8_ptr.into()], true);
        self.module.add_function("printf", printf_type, None)
    }
}


impl<'ctx> Visitor for LlvmCodeGen<'ctx> {
    type Result = Result<BasicValueEnum<'ctx>, String>;

    fn visit_program(&mut self, program: &Program) -> Self::Result {
        let f64_type = self.context.f64_type();
        let main_type = f64_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_type, None);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);
        self.current_function = Some(main_fn);

        let mut last_value: Option<BasicValueEnum> = None;
        for stmt in &program.statements {
            last_value = Some(stmt.accept(self)?);
        }

        let ret_val = last_value.unwrap_or_else(|| f64_type.const_float(0.0).into());
        self.builder.build_return(Some(&ret_val)).map_err(|e| e.to_string())?;

        self.current_function = None;
        Ok(ret_val)
    }

    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Result {
        stmt.expr.accept(self)
    }

    fn visit_number(&mut self, expr: &NumberExpr) -> Self::Result {
        let f64_type = self.context.f64_type();
        Ok(f64_type.const_float(expr.value).into())
    }

    fn visit_binary_op(&mut self, expr: &BinaryOpExpr) -> Self::Result {
        let lhs = expr.left.accept(self)?.into_float_value();
        let rhs = expr.right.accept(self)?.into_float_value();

        match expr.op {
            BinOp::Add => {
                let val = self.builder.build_float_add(lhs, rhs, "addtmp").map_err(|e| e.to_string())?;
                Ok(val.into())
            }
            BinOp::Sub => {
                let val = self.builder.build_float_sub(lhs, rhs, "subtmp").map_err(|e| e.to_string())?;
                Ok(val.into())
            }
            BinOp::Mul => {
                let val = self.builder.build_float_mul(lhs, rhs, "multmp").map_err(|e| e.to_string())?;
                Ok(val.into())
            }
            BinOp::Div => {
                let val = self.builder.build_float_div(lhs, rhs, "divtmp").map_err(|e| e.to_string())?;
                Ok(val.into())
            }
            BinOp::Pow => {
                let pow_fn = self.module.get_function("llvm.pow.f64").unwrap_or_else(|| {
                    let f64 = self.context.f64_type();
                    let pow_type = f64.fn_type(&[f64.into(), f64.into()], false);
                    self.module.add_function("llvm.pow.f64", pow_type, None)
                });
                let call_site = self.builder.build_call(pow_fn, &[lhs.into(), rhs.into()], "powtmp")
                    .map_err(|e| e.to_string())?;
                let result = call_site.try_as_basic_value().left()
                    .ok_or("Failed to convert pow call to basic value")?;
                Ok(result)
            }
            BinOp::Concat => todo!(),
        }
    }

    fn visit_print(&mut self, expr: &PrintExpr) -> Self::Result {
        let value = expr.argument.accept(self)?.into_float_value();
        let printf_fn = self.declare_printf();

        let format_str = self.builder.build_global_string_ptr("%f\n", "fmt")
            .map_err(|e| e.to_string())?;

        self.builder.build_call(printf_fn, &[format_str.as_pointer_value().into(), value.into()], "printf_call")
            .map_err(|e| e.to_string())?;

        Ok(value.into())
    }
    
    fn visit_string(&mut self, expr: &crate::ast::StringExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_call(&mut self, expr: &crate::ast::CallExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_const(&mut self, expr: &crate::ast::ConstExpr) -> Self::Result {
        todo!()
    }
}

