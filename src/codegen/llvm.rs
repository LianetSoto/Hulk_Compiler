use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue, GlobalValue};
use std::collections::HashMap;
use crate::semantic::{HulkType, FlattenedType};
use crate::ast::{Program, Node, Expr, TypeDef, Method};
use crate::error::{CompilerError};
use inkwell::types::{StructType, BasicTypeEnum};

pub struct LlvmCodeGen<'ctx> {
    pub(crate) context: &'ctx Context, 
    pub(crate) module: Module<'ctx>,
    pub(crate) builder: Builder<'ctx>,
    pub(crate) scopes: Vec<HashMap<String, PointerValue<'ctx>>>,
    pub(crate) current_function: Option<FunctionValue<'ctx>>,
    pub(crate) user_functions: HashMap<String, FunctionValue<'ctx>>, 
    pub(crate) method_functions: HashMap<String, FunctionValue<'ctx>>,
    pub(crate) type_structs: HashMap<String, StructType<'ctx>>,
    pub(crate) type_defs: HashMap<String, TypeDef>,
    pub(crate) flattened_types: HashMap<String, FlattenedType>,
    pub(crate) vtables: HashMap<String, GlobalValue<'ctx>>,
    pub(crate) current_method: Option<Method>,
    pub(crate) vtable_types: HashMap<String, StructType<'ctx>>,
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
            method_functions: HashMap::new(),
            type_structs: HashMap::new(),
            type_defs: HashMap::new(),
            flattened_types: HashMap::new(),
            vtables: HashMap::new(),
            current_method: None,
            vtable_types: HashMap::new(),
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

    pub fn set_flattened_types(&mut self, types: HashMap<String, FlattenedType>) {
        self.flattened_types = types;
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
    pub(crate) fn hulk_type_to_llvm_type(&self, ty: &HulkType) -> Result<BasicTypeEnum<'ctx>, CompilerError> {
        match ty {
            HulkType::Number => Ok(self.context.f64_type().into()),
            HulkType::Boolean => Ok(self.context.bool_type().into()),
            HulkType::String => Ok(self.context.i8_type()
                .ptr_type(inkwell::AddressSpace::default())
                .into()),
            HulkType::Class(name) => {
                if let Some(st) = self.type_structs.get(name) {
                    Ok(st.ptr_type(inkwell::AddressSpace::default()).into())
                } else {
                    // Fallback to opaque pointer if struct not found 
                    Ok(self.context.i8_type()
                        .ptr_type(inkwell::AddressSpace::default())
                        .into())
                }
            }
            _ => Err(CompilerError::CodegenError {
                msg: format!("unexpected type in code generation: {:?}", ty),
                span: None, 
            }),
        }
    }

    /// Return a sensible default LLVM value for a given HULK type.
    pub(crate) fn default_value_for_type(&self, ty: &HulkType) -> Result<BasicValueEnum<'ctx>, CompilerError> {
        match ty {
            HulkType::Number => Ok(self.context.f64_type().const_float(0.0).into()),
            HulkType::Boolean => Ok(self.context.bool_type().const_int(0, false).into()),
            HulkType::String | HulkType::Class(_)=> {
                let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
                Ok(i8_ptr.const_null().into())
            }
            _ => Err(CompilerError::CodegenError {
                msg: format!("unexpected type in default_value_for_type: {:?}", ty),
                span: None,
            }),
        }
    }

    /// Evaluates an expression that is expected to be assignable (an lvalue)
    /// and returns a pointer to the memory location where a value can be stored.
    pub(crate) fn eval_lvalue(&mut self, expr: &mut Expr) -> Result<PointerValue<'ctx>, CompilerError> {
        match expr {
            // Simple variable
            Expr::Variable(var) => {
                match self.lookup_var(&var.name) {
                    Some(ptr) => Ok(ptr),
                    None => Err(CompilerError::CodegenError {
                        msg: format!("undefined variable '{}'", var.name),
                        span: Some(var.span),
                    }),
                }
            }

            // Attribute access (e.g. self.x, obj.field)
            Expr::AttributeAccess(attr) => {
                let (ptr, _) = self.get_attribute_ptr(attr)?;
                Ok(ptr)
            }

            // Anything else is not assignable (should be caught by TypeChecker)
            _ => Err(CompilerError::CodegenError {
                msg: "left-hand side of assignment is not assignable".to_string(),
                span: Some(expr.span()),
            }),
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