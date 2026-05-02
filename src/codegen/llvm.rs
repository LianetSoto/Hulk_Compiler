use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;
use crate::semantic::HulkType;
use crate::ast::{Program, Node};
use crate::error::{CompilerError, Span};

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

    /// Declares the C standard library function `malloc`, which allocates a block
    /// of memory of the given size and returns a pointer to it.
    /// Signature: `i8* @malloc(i64 %size)`
    pub(crate) fn declare_malloc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("malloc") {
            return f;
        }
        let i64 = self.context.i64_type();
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let fn_type = i8_ptr.fn_type(&[i64.into()], false);
        self.module.add_function("malloc", fn_type, None)
    }

    /// Declares the C standard library function `strlen`, which returns the length
    /// of a null-terminated string.
    /// Signature: `i64 @strlen(i8* %str)`
    pub(crate) fn declare_strlen(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("strlen") {
            return f;
        }
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let i64 = self.context.i64_type();
        let fn_type = i64.fn_type(&[i8_ptr.into()], false);
        self.module.add_function("strlen", fn_type, None)
    }

    /// Declares the C standard library function `memcpy`, which copies `n` bytes
    /// from source to destination memory areas.
    /// Signature: `i8* @memcpy(i8* %dest, i8* %src, i64 %n)`
    pub(crate) fn declare_memcpy(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("memcpy") {
            return f;
        }
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let i64 = self.context.i64_type();
        let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into(), i64.into()], false);
        self.module.add_function("memcpy", fn_type, None)
    }

    /// Declares the C standard library function `sprintf`, which writes formatted
    /// output to a string buffer.
    /// Signature: `i32 @sprintf(i8* %buffer, i8* %format, ...)`
    pub(crate) fn declare_sprintf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("sprintf") {
            return f;
        }

        // Get the type `i8*` (pointer to an 8‑bit integer, i.e. `char*` in C).
        // This is the type of the first two arguments of `sprintf`:
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());

        // 3. Get the type `f64` (64‑bit floating‑point).  We include it here
        //    because our codegen will always pass a `Number` (f64) as the value
        //    to be formatted.  It is added as a **fixed** parameter so that LLVM
        //    can verify the type of the third argument. 
        let f64 = self.context.f64_type();

        // Get the type `i32` (32‑bit integer).  This is the return type of
        // `sprintf` – it returns the number of characters written (excluding
        // the null terminator).
        let i32 = self.context.i32_type();

        // 5. Build the function type.
        //    - The first argument is a slice of fixed parameter types:
        //         [i8_ptr, i8_ptr, f64]
        //      meaning: `char* buffer, const char* format, double value`
        //    - The boolean `true` indicates that the function is **variadic**
        //      This allows us to pass only the float when we use it, but
        //      the signature still accepts more if needed.
        let fn_type = i32.fn_type(&[i8_ptr.into(), i8_ptr.into(), f64.into()], true);
        self.module.add_function("sprintf", fn_type, None)
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

    /// Converts an LLVM value to a string pointer (`i8*`), suitable for string
    /// concatenation.
    /// - If the value is already a pointer (`HulkType::String`), returns it unchanged.
    /// - If the value is a float (`HulkType::Number`), formats it into a newly allocated
    ///   buffer using `sprintf` and returns that buffer.
    pub(crate) fn value_to_string_ptr(&self, val: BasicValueEnum<'ctx>, span: Span,) 
    -> Result<PointerValue<'ctx>, CompilerError> {

        if val.is_pointer_value() {
            // It's a string literal or already a pointer → use directly
            return Ok(val.into_pointer_value());
        }

        // ---------- Number → string conversion ----------
        let float_val = val.into_float_value();
        let sprintf_fn = self.declare_sprintf();

        // Allocate a temporary buffer (32 bytes is enough for most numbers)
        let buffer_size = self.context.i64_type().const_int(32, false);
        let malloc_fn = self.declare_malloc();
        let buffer = self.builder
            .build_call(malloc_fn, &[buffer_size.into()], "buf")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(span),
            })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_pointer_value().into())
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "malloc for number formatting failed".to_string(),
                span: Some(span),
            })?;

        // Create the format string "%g" (compact floating-point representation)
        let fmt = self.builder
            .build_global_string_ptr("%g", "fmt_num")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(span),
            })?;

        // Call sprintf(buffer, "%g", float_val)
        self.builder
            .build_call(
                sprintf_fn,
                &[
                    buffer.into(),              // destination buffer
                    fmt.as_pointer_value().into(), // format string
                    float_val.into(),           // value to format
                ],
                "",
            )
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(span),
            })?;

        Ok(buffer)
    }

    /// Concatenates two values (which may be strings or numbers) into a newly
    /// allocated buffer, optionally inserting a separator string between them.
    pub(crate) fn concat_strings(
        &self,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        separator: Option<&str>,
        span: Span,
    ) -> Result<PointerValue<'ctx>, CompilerError> {
        // Convert both operands to string pointers
        let lhs_ptr = self.value_to_string_ptr(lhs, span)?;
        let rhs_ptr = self.value_to_string_ptr(rhs, span)?;

        // Declare needed libc functions
        let strlen_fn = self.declare_strlen();
        let malloc_fn = self.declare_malloc();
        let memcpy_fn = self.declare_memcpy();

        // Compute lengths
        let lhs_len = self.builder
            .build_call(strlen_fn, &[lhs_ptr.into()], "lhs_len")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_int_value().into())
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "strlen did not return an integer".to_string(),
                span: Some(span),
            })?;

        let rhs_len = self.builder
            .build_call(strlen_fn, &[rhs_ptr.into()], "rhs_len")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_int_value().into())
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "strlen did not return an integer".to_string(),
                span: Some(span),
            })?;

        // Handle separator (if any) – both branches must return a PointerValue
        let (sep_ptr, sep_len_val) = if let Some(sep_str) = separator {
            let global = self.builder
                .build_global_string_ptr(sep_str, "sep")
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
            let ptr = global.as_pointer_value();   // extract PointerValue from GlobalValue
            let len = self.builder
                .build_call(strlen_fn, &[ptr.into()], "sep_len")
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?
                .try_as_basic_value().left()
                .and_then(|v| v.into_int_value().into())
                .ok_or_else(|| CompilerError::CodegenError {
                    msg: "strlen for separator failed".to_string(),
                    span: Some(span),
                })?;
            (ptr, len)
        } else {
            let null_ptr = self.context.i8_type()
                .ptr_type(inkwell::AddressSpace::default())
                .const_null();
            (null_ptr, self.context.i64_type().const_int(0, false))
        };

        // Total size = lhs_len + sep_len + rhs_len + 1 (null terminator)
        let one = self.context.i64_type().const_int(1, false);
        let total_size = self.builder
            .build_int_add(lhs_len, sep_len_val, "add_sep")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        let total_size = self.builder
            .build_int_add(total_size, rhs_len, "add_rhs")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        let total_size = self.builder
            .build_int_add(total_size, one, "add_null")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        // Allocate buffer
        let buffer = self.builder
            .build_call(malloc_fn, &[total_size.into()], "concat_buf")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_pointer_value().into())
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "malloc for concatenation failed".to_string(),
                span: Some(span),
            })?;

        // Copy left string
        self.builder
            .build_call(memcpy_fn, &[buffer.into(), lhs_ptr.into(), lhs_len.into()], "")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        // Copy separator (if not empty)
        if separator.is_some() {
            let dest_sep = unsafe {
                self.builder.build_gep(self.context.i8_type(), buffer, &[lhs_len], "dest_sep")
            }.map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
            self.builder
                .build_call(memcpy_fn, &[dest_sep.into(), sep_ptr.into(), sep_len_val.into()], "")
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        }

        // Copy right string (position = lhs_len + sep_len)
        let dest_rhs_offset = self.builder
            .build_int_add(lhs_len, sep_len_val, "dest_rhs_offset")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        let dest_rhs = unsafe {
            self.builder.build_gep(self.context.i8_type(), buffer, &[dest_rhs_offset], "dest_rhs")
        }.map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        self.builder
            .build_call(memcpy_fn, &[dest_rhs.into(), rhs_ptr.into(), rhs_len.into()], "")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        // Null-terminate: buffer[total_size - 1] = 0
        let null_byte = self.context.i8_type().const_int(0, false);
        let last_byte_offset = self.builder
            .build_int_sub(total_size, one, "last_byte_offset")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        let end_minus_one = unsafe {
            self.builder.build_gep(self.context.i8_type(), buffer, &[last_byte_offset], "end_minus_one")
        }.map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        self.builder.build_store(end_minus_one, null_byte)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        Ok(buffer)
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