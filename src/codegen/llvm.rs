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
    pub(crate) user_functions: HashMap<String, FunctionValue<'ctx>>,
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
            user_functions: HashMap::new(),
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