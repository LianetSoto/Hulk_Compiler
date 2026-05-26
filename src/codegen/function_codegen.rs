use super::llvm::LlvmCodeGen;
use crate::ast::*;
use std::collections::HashMap;
use inkwell::values::{FunctionValue};
use crate::error::{Span, CompilerError};
use crate::semantic::HulkType;
use inkwell::types::{FunctionType, BasicTypeEnum, BasicMetadataTypeEnum};
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmCodeGen<'ctx> {
    
    /// Declares all built‑in and user functions in the LLVM module.
    /// Returns a map of user function names to their `FunctionValue`s.
    pub(crate) fn declare_all_functions(
        &self,
        user_funcs: &[FunctionDef],
    ) -> Result<HashMap<String, FunctionValue<'ctx>>, CompilerError> {
        // 1. Built‑ins 
        self.declare_all_builtins();

        // 2. User functions
        let mut map = HashMap::new();
        for func in user_funcs {
            let f = self.declare_user_function_signature(func, None)?;
            map.insert(func.name.clone(), f);
        }
        Ok(map)
    }

    fn declare_all_builtins(&self) {
        self.declare_sin();
        self.declare_cos();
        self.declare_sqrt();
        self.declare_exp();
        self.declare_log();
        self.declare_rand();
        self.declare_fmod();
        // self.declare_printf();
        // self.declare_strcmp();
        // self.declare_sprintf();
        // self.declare_strlen();
        // self.declare_malloc();
        // self.declare_memcpy();
    }

    /// Declares the LLVM function signature for a user‑defined HULK function.
    ///
    /// If `llvm_name` is `Some`, that name is used for the LLVM function
    /// Otherwise `func.name` is used.
    /// The returned `FunctionValue` can later be used to compile the function body.
    pub(crate) fn declare_user_function_signature(
        &self,
        func: &FunctionDef,
        llvm_name: Option<&str>,
    ) -> Result<FunctionValue<'ctx>, CompilerError> {
        let param_types: Result<Vec<BasicTypeEnum<'ctx>>, CompilerError> = func.params.iter()
            .map(|p| self.hulk_type_to_llvm_type(p.ty.as_ref().unwrap()))
            .collect();
        let param_types = param_types?;

        let ret_type = self.hulk_type_to_llvm_type(func.ty.as_ref().unwrap_or(&HulkType::Number))?;

        let name = llvm_name.unwrap_or(&func.name);
        Ok(self.declare_function_generic(name, ret_type, &param_types))
    }

    /// Low‑level helper that adds a function declaration to the LLVM module.
    /// It takes the LLVM name, the return type and a list of parameter types.
    pub(crate) fn declare_function_generic(
        &self,
        llvm_name: &str,
        ret_type: BasicTypeEnum<'ctx>,
        param_types: &[BasicTypeEnum<'ctx>],
    ) -> FunctionValue<'ctx> {
        let fn_type = self.build_fn_type(ret_type, param_types);
        self.module.add_function(llvm_name, fn_type, None)
    }

    /// Builds an LLVM function type from a return type and a list of parameter types.
    /// This helper centralises the creation of the `FunctionType` that is required
    /// when declaring a function with `add_function`.  It converts every parameter
    /// type from `BasicTypeEnum` to `BasicMetadataTypeEnum` (the type expected by
    /// the `fn_type` method) and then dispatches on the concrete return type to
    /// call the appropriate `fn_type` constructor (Float, Int or Pointer).
    fn build_fn_type(
        &self,
        ret_type: BasicTypeEnum<'ctx>,
        param_types: &[BasicTypeEnum<'ctx>],
    ) -> FunctionType<'ctx> {
        // Convert each parameter to the required metadata enum variant.
        let params: Vec<BasicMetadataTypeEnum<'ctx>> = param_types
            .iter()
            .map(|t| (*t).into())
            .collect();

        // The return type decides which `.fn_type()` method we call.
        match ret_type {
            BasicTypeEnum::FloatType(f)   => f.fn_type(&params, false),
            BasicTypeEnum::IntType(i)     => i.fn_type(&params, false),
            BasicTypeEnum::PointerType(p) => p.fn_type(&params, false),
            _ => unimplemented!("unsupported return type for function"),
        }
    }

    pub(crate) fn compile_llvm_function(
        &mut self,
        function: FunctionValue<'ctx>,
        params: Vec<(String, BasicTypeEnum<'ctx>)>, 
        body: &mut Box<Expr>,
        span: Span,
    ) -> Result<(), CompilerError> {

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Remember the current scope chain and the enclosing function, then set
        // up a fresh scope for the function's own variables.
        let old_scopes = std::mem::take(&mut self.scopes);
        self.scopes = vec![HashMap::new()];
        let old_function = self.current_function.replace(function);

        // Allocate stack storage for each parameter and store the incoming value.
        for (i, (name, llvm_ty)) in params.iter().enumerate() {
            let param_val = function.get_nth_param(i as u32).unwrap();
            let alloca = self.builder.build_alloca(*llvm_ty, name)
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
            self.builder.build_store(alloca, param_val)
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
            self.insert_var(name.clone(), alloca);
        }

        // Generate the IR for the body expression.
        let body_val = body.accept(self)?;

        // The result of the function is the value produced by its body.
        self.builder.build_return(Some(&body_val))
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        // Restore the previous scope chain and the surrounding function context.
        self.scopes = old_scopes;
        self.current_function = old_function;
        Ok(())
    }

    // This method delegates to the C standard library function `printf` to perform the
    // actual output. It selects the appropriate format specifier and argument conversion
    // based on the static type of the expression being printed.

    /// # Supported Types and Output Format
    /// - `String`   → printed as plain text followed by a newline (`%s\n`).
    /// - `Number`   → printed as a floating‑point number followed by a newline (`%g\n`).
    /// - `Boolean`  → printed as the word `true` or `false` followed by a newline (`%s\n`).
    
    // # LLVM Concepts Used
    // - **Global string constants**: `build_global_string_ptr` creates a global constant
    //   array of bytes (e.g., `"%s\n\00"`) and returns an `i8*` pointer to its first element.
    //   This is necessary because `printf` expects a pointer to a null‑terminated format
    //   string, not an immediate value.
    // - **`printf` declaration**: The function is lazily declared with the signature
    //   `i32 @printf(i8*, ...)`. It is assumed that the target platform provides a standard
    //   C library.
    // - **`select` instruction**: For `Boolean` values, the LLVM `select` instruction is
    //   used to choose between a pointer to the global constant `"true"` and a pointer to
    //   `"false"`, based on the `i1` boolean value.

    pub(crate) fn compile_print_call(
        &mut self,
        expr: &mut CallExpr,
    ) -> Result<BasicValueEnum<'ctx>, CompilerError> {
        
        // 1. Generate code for the argument expression.
        let value = expr.args[0].accept(self)?;

        // 2. Retrieve the static type of the argument (inferred by the type checker).
        let arg_ty = expr.args[0].get_type().ok_or_else(|| CompilerError::CodegenError {
            msg: "type not inferred for print argument".to_string(),
            span: Some(expr.span),
        })?;

        let printf_fn = self.declare_printf();

        let format_str = match arg_ty {
            HulkType::String | HulkType::Boolean =>
                self.builder.build_global_string_ptr("%s\n", "fmt_str"),
            HulkType::Number =>
                self.builder.build_global_string_ptr("%g\n", "fmt_num"),
            _ => return Err(CompilerError::CodegenError {
                msg: "cannot print this type".to_string(),
                span: Some(expr.span),
            }),
        }
        .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;

        let llvm_arg = if *arg_ty == HulkType::String {
            value.into_pointer_value().into()
        } else if *arg_ty == HulkType::Boolean {
            let bool_val = value.into_int_value();
            let true_ptr = self.get_true_str();
            let false_ptr = self.get_false_str();
            self.builder
                .build_select(bool_val, true_ptr, false_ptr, "bool_str")
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?
                .into()
        } else {
            value.into_float_value().into()
        };

        self.builder
            .build_call(printf_fn, &[format_str.as_pointer_value().into(), llvm_arg], "printf_call")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;

        Ok(value)   
    }

    pub(crate) fn compile_rand_call(&mut self, expr: &mut CallExpr) -> Result<BasicValueEnum<'ctx>, CompilerError> {
        let rand_fn = self.module.get_function("rand")
            .expect("rand not declared");
        let call_site = self.builder.build_call(rand_fn, &[], "randtmp")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;
        let rand_int = call_site.try_as_basic_value().left()
            .and_then(|v| v.into_int_value().into())
            .ok_or_else(|| CompilerError::CodegenError { msg: "rand call did not return an integer".into(), span: Some(expr.span) })?;

        let f64_type = self.context.f64_type();
        let rand_float = self.builder.build_signed_int_to_float(rand_int, f64_type, "randf64")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;
        let rand_max = f64_type.const_float(2147483647.0);
        let result = self.builder.build_float_div(rand_float, rand_max, "rand_norm")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;
        Ok(result.into())
    }

    pub(crate) fn compile_log_call(&mut self, expr: &mut CallExpr) -> Result<BasicValueEnum<'ctx>, CompilerError> {
        let log_fn = self.module.get_function("log")
            .expect("log not declared"); 

        let base = expr.args[0].accept(self)?.into_float_value();
        let value = expr.args[1].accept(self)?.into_float_value();

        let log_val = self.builder.build_call(log_fn, &[value.into()], "log_val")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_float_value().into())
            .ok_or_else(|| CompilerError::CodegenError { msg: "log(value) call failed".into(), span: Some(expr.span) })?;

        let log_base = self.builder.build_call(log_fn, &[base.into()], "log_base")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_float_value().into())
            .ok_or_else(|| CompilerError::CodegenError { msg: "log(base) call failed".into(), span: Some(expr.span) })?;

        let result = self.builder.build_float_div(log_val, log_base, "log_result")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;
        Ok(result.into())
    }

    /// Declares a math function that takes one `f64` and returns `f64`. (reusable helper)
    fn declare_math_function(&self, name: &str) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let f64 = self.context.f64_type();
        let fn_type = f64.fn_type(&[f64.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    // External C function declarations (standard library)

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

    /// Declares the C standard library function `fmod` (floating‑point remainder).
    /// Signature: `double @fmod(double, double)`
    pub(crate) fn declare_fmod(&self) -> FunctionValue<'ctx> {
        self.declare_math_function("fmod")
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

}