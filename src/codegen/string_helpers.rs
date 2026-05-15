use super::llvm::LlvmCodeGen;
use inkwell::values::FunctionValue;
use crate::error::{CompilerError, Span};
use inkwell::values::{BasicValueEnum, PointerValue};

impl<'ctx> LlvmCodeGen<'ctx> {

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

    /// Declares `i32 @printf(i8*, ...)`.
    pub(crate) fn declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let printf_type = self.context.i32_type().fn_type(&[i8_ptr.into()], true);
        self.module.add_function("printf", printf_type, None)
    }

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

}