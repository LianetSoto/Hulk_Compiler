use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;
use crate::semantic::HulkType;
use crate::ast::{Program, Node};
use crate::error::{CompilerError};

pub struct LlvmCodeGen<'ctx> {
    pub(crate) context: &'ctx Context, 
    pub(crate) module: Module<'ctx>,
    pub(crate) builder: Builder<'ctx>,
    pub(crate) scopes: Vec<HashMap<String, PointerValue<'ctx>>>,
    pub(crate) current_function: Option<FunctionValue<'ctx>>,
}

impl<'ctx> LlvmCodeGen<'ctx> {

    // Constructor and compilation entry points

    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            scopes: vec![HashMap::new()],
            current_function: None,
        }
    }

    pub fn compile(&mut self, program: &mut Program) -> Result<(), CompilerError> {
        program.accept(self)?;
        Ok(())
    }

    pub fn write_to_file(&self, filename: &str) -> Result<(), CompilerError> {
        self.module.print_to_file(filename)
            .map_err(|e| CompilerError::IoError(e.to_string()))
    }

    // Scope management (variable symbol table)

    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn insert_var(&mut self, name: String, ptr: PointerValue<'ctx>) {
        self.scopes.last_mut().unwrap().insert(name, ptr);
    }

    /// Look up a variable from innermost to outermost scope.
    pub(crate) fn lookup_var(&self, name: &str) -> Option<PointerValue<'ctx>> {
        for scope in self.scopes.iter().rev() {
            if let Some(&ptr) = scope.get(name) {
                return Some(ptr);
            }
        }
        None
    }

    // Type helpers (HULK type → LLVM type / default value)

    /// Convert a HULK type to the corresponding LLVM type.
    pub(crate) fn hulk_type_to_llvm_type(&self, ty: &HulkType) -> inkwell::types::BasicTypeEnum<'ctx> {
        match ty {
            HulkType::Number => self.context.f64_type().into(),
            HulkType::String => self.context.i8_type()
                .ptr_type(inkwell::AddressSpace::default()).into(),
            HulkType::Boolean => self.context.bool_type().into(),
            _ => self.context.f64_type().into(), // fallback for unknown types
        }
    }

    /// Return a sensible default LLVM value for a given HULK type.
    pub(crate) fn default_value_for_type(&self, ty: &HulkType) -> BasicValueEnum<'ctx> {
        match ty {
            HulkType::Number => self.context.f64_type().const_float(0.0).into(),
            HulkType::String => {
                let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
                i8_ptr.const_null().into()
            }
            HulkType::Boolean => self.context.bool_type().const_int(0, false).into(),
            _ => self.context.f64_type().const_float(0.0).into(), // fallback
        }
    }

    // String helpers (true/false constants)

    /// Returns a pointer to the global constant "true".
    pub(crate) fn get_true_str(&self) -> PointerValue<'ctx> {
        if let Some(gv) = self.module.get_global("true_str") {
            return gv.as_pointer_value();
        }
        self.builder
            .build_global_string_ptr("true", "true_str")
            .unwrap()
            .as_pointer_value()
    }

    /// Returns a pointer to the global constant "false".
    pub(crate) fn get_false_str(&self) -> PointerValue<'ctx> {
        if let Some(gv) = self.module.get_global("false_str") {
            return gv.as_pointer_value();
        }
        self.builder
            .build_global_string_ptr("false", "false_str")
            .unwrap()
            .as_pointer_value()
    }

    // Math function declaration (reusable helper)

    /// Declares a math function that takes one `f64` and returns `f64`.
    fn declare_math_function(&self, name: &str) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let f64 = self.context.f64_type();
        let fn_type = f64.fn_type(&[f64.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    // External C function declarations (standard library)

    /// Declares `i32 @strcmp(i8*, i8*)`.
    pub(crate) fn declare_strcmp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("strcmp") {
            return f;
        }
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let i32 = self.context.i32_type();
        let fn_type = i32.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
        self.module.add_function("strcmp", fn_type, None)
    }

    /// Declares `double @sin(double)`.
    pub(crate) fn declare_sin(&self) -> FunctionValue<'ctx> {
        self.declare_math_function("sin")
    }

    /// Declares `double @cos(double)`.
    pub(crate) fn declare_cos(&self) -> FunctionValue<'ctx> {
        self.declare_math_function("cos")
    }

    /// Declares `double @sqrt(double)`.
    pub(crate) fn declare_sqrt(&self) -> FunctionValue<'ctx> {
        self.declare_math_function("sqrt")
    }

    /// Declares `double @exp(double)`.
    pub(crate) fn declare_exp(&self) -> FunctionValue<'ctx> {
        self.declare_math_function("exp")
    }

    /// Declares `double @log(double)`.
    pub(crate) fn declare_log(&self) -> FunctionValue<'ctx> {
        self.declare_math_function("log")
    }

    /// Declares `void @srand(i32)`.
    pub(crate) fn declare_srand(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("srand") {
            return f;
        }
        let void_type = self.context.void_type();
        let i32_type = self.context.i32_type();
        let fn_type = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("srand", fn_type, None)
    }

    /// Declares `i64 @time(i64*)`.
    pub(crate) fn declare_time(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("time") {
            return f;
        }
        let i64_type = self.context.i64_type();
        let i64_ptr = i64_type.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i64_type.fn_type(&[i64_ptr.into()], false);
        self.module.add_function("time", fn_type, None)
    }

    /// Declares `i32 @rand()`.
    pub(crate) fn declare_rand(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("rand") {
            return f;
        }
        let i32 = self.context.i32_type();
        let fn_type = i32.fn_type(&[], false);
        self.module.add_function("rand", fn_type, None)
    }

    /// Declares `i32 @printf(i8*, ...)`.
    pub(crate) fn declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let printf_type = self.context.i32_type().fn_type(&[i8_ptr.into()], true);
        self.module.add_function("printf", printf_type, None)
    }

    // Random generator seeding

    /// Seeds the C random number generator with the current time so that
    /// `rand()` returns a different sequence on each program execution.
    pub(crate) fn seed_random_generator(&self) -> Result<(), CompilerError> {
        let time_fn = self.declare_time();

        let null_ptr = self.context.i64_type()
            .ptr_type(inkwell::AddressSpace::default())
            .const_null();
        let current_time = self.builder
            .build_call(time_fn, &[null_ptr.into()], "cur_time")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: None,
            })?
            .try_as_basic_value()
            .left()
            .and_then(|v| v.into_int_value().into())
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "time call did not return an integer".to_string(),
                span: None,
            })?;

        let i32_type = self.context.i32_type();
        let seed = self.builder
            .build_int_truncate(current_time, i32_type, "seed")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: None,
            })?;

        let srand_fn = self.declare_srand();
        self.builder.build_call(srand_fn, &[seed.into()], "")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: None,
            })?;

        Ok(())
    }
}