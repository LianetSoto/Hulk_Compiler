use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;
use crate::semantic::HulkType;
use crate::ast::{Node, Visitor, Program, ExprStmt, 
    NumberExpr, BinaryOpExpr, PrintExpr, BinOp, StringExpr,
    CallExpr, ConstExpr, UnaryOpExpr, BoolExpr, UnaryOp};
use crate::error::{CompilerError};

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

    pub fn compile(&mut self, program: &mut Program) -> Result<(), CompilerError> {
        program.accept(self)?;
        Ok(())
    }

    pub fn write_to_file(&self, filename: &str) -> Result<(), CompilerError> {
        self.module.print_to_file(filename)
            .map_err(|e| CompilerError::IoError(e.to_string()))
    }

    fn get_true_str(&self) -> PointerValue<'ctx> {
        if let Some(gv) = self.module.get_global("true_str") {
            return gv.as_pointer_value();
        }
        self.builder
            .build_global_string_ptr("true", "true_str")
            .unwrap()
            .as_pointer_value()
    }

    fn get_false_str(&self) -> PointerValue<'ctx> {
        if let Some(gv) = self.module.get_global("false_str") {
            return gv.as_pointer_value();
        }
        self.builder
            .build_global_string_ptr("false", "false_str")
            .unwrap()
            .as_pointer_value()
    }

    fn declare_strcmp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("strcmp") {
            return f;
        }
        let i8_ptr = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let i32 = self.context.i32_type();
        let fn_type = i32.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
        self.module.add_function("strcmp", fn_type, None)
    }

    /// Declares the external C library function `sin`.
    fn declare_sin(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("sin") {
            return f;
        }
        let f64 = self.context.f64_type();
        let fn_type = f64.fn_type(&[f64.into()], false);
        self.module.add_function("sin", fn_type, None)
    }

    /// Declares the external C library function `cos`.
    fn declare_cos(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("cos") {
            return f;
        }
        let f64 = self.context.f64_type();
        let fn_type = f64.fn_type(&[f64.into()], false);
        self.module.add_function("cos", fn_type, None)
    }

    /// Declares the external C library function `sqrt`.
    fn declare_sqrt(&self) -> FunctionValue<'ctx> {
            if let Some(f) = self.module.get_function("sqrt") {
                return f;
            }
            let f64 = self.context.f64_type();
            let fn_type = f64.fn_type(&[f64.into()], false);
            self.module.add_function("sqrt", fn_type, None)
        }

    /// Declares the external C library function `exp`.
    fn declare_exp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("exp") {
            return f;
        }
        let f64 = self.context.f64_type();
        let fn_type = f64.fn_type(&[f64.into()], false);
        self.module.add_function("exp", fn_type, None)
    }

    /// Declares the external C library function `rand`.
    /// `rand()` returns an `int` in C, but we cast it to `f64` for HULK.
    fn declare_rand(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("rand") {
            return f;
        }
        let i32 = self.context.i32_type();
        let fn_type = i32.fn_type(&[], false);
        self.module.add_function("rand", fn_type, None)
    }

    /// Declares the external C library function `log`.
        fn declare_log(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("log") {
            return f;
        }
        let f64 = self.context.f64_type();
        let fn_type = f64.fn_type(&[f64.into()], false);
        self.module.add_function("log", fn_type, None)
    }

    /// Declares the C standard library function `srand`, which seeds the random
    /// number generator used by `rand`.
    /// Signature: `void @srand(i32 %seed)`
    fn declare_srand(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("srand") {
            return f;
        }
        let void_type = self.context.void_type();
        let i32_type = self.context.i32_type();
        let fn_type = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("srand", fn_type, None)
    }

    /// Declares the C standard library function `time`, which returns the current
    /// calendar time as a `time_t` (typically `i64`).
    /// Signature: `i64 @time(i64* %tloc)`
    fn declare_time(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("time") {
            return f;
        }
        let i64_type = self.context.i64_type();
        let i64_ptr = i64_type.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i64_type.fn_type(&[i64_ptr.into()], false);
        self.module.add_function("time", fn_type, None)
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
    type Result = Result<BasicValueEnum<'ctx>, CompilerError>;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result {
        // Cambia el tipo de retorno de main a i32
        let i32_type = self.context.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_type, None);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);
        self.current_function = Some(main_fn);

        // Seed the C random number generator with the current time so that
        // `rand()` returns a different sequence on each program execution
        
        {
            // Call `time(NULL)` → returns the current time as `i64`.
            let time_fn = self.declare_time();
            let null_ptr = self.context.i64_type().ptr_type(inkwell::AddressSpace::default()).const_null();
            let current_time = self.builder
                .build_call(time_fn, &[null_ptr.into()], "cur_time")
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: None,
                })?
                .try_as_basic_value().left()
                .and_then(|v| v.into_int_value().into())
                .ok_or_else(|| CompilerError::CodegenError {
                    msg: "time call did not return an integer".to_string(),
                    span: None,
                })?;

            // `srand` expects an `i32` seed. Truncate the `i64` time to `i32`.
            let seed = self.builder.build_int_truncate(current_time, i32_type, "seed")
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: None,
                })?;

            // Call `srand(seed)`.
            let srand_fn = self.declare_srand();
            self.builder.build_call(srand_fn, &[seed.into()], "")
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: None,
                })?;
        }

        // Ejecuta todas las sentencias
        for stmt in &mut program.statements {
            stmt.accept(self)?;
        }

        // Siempre retorna 0 (éxito)
        let zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&zero))
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: None,
            })?;

        self.current_function = None;
        Ok(zero.into())
    }

    fn visit_expr_stmt(&mut self, stmt: &mut ExprStmt) -> Self::Result {
        stmt.expr.accept(self)
    }

    fn visit_number(&mut self, expr: &mut NumberExpr) -> Self::Result {
        let f64_type = self.context.f64_type();
        Ok(f64_type.const_float(expr.value).into())
    }

    /// Generates LLVM IR for a string literal expression.
    
    // This method does not evaluate the string; instead, it emits a global constant
    // containing the string's bytes (UTF-8 + null terminator) and returns a pointer
    // to that constant. The returned pointer can be used wherever a string is expected,
    // such as when calling `printf` with the `%s` format specifier.

    // # Steps
    // 1. Call `build_global_string_ptr` to create an LLVM global constant of type
    //    `[N x i8]` initialized with the string's UTF-8 bytes and a null terminator.
    //    This also returns an `i8*` pointer to the first byte.
    // 2. Convert the returned `GlobalValue` to a `PointerValue` and then to the
    //    generic `BasicValueEnum` expected by the visitor.
    //
    // # Errors
    // Returns `CodegenError` if LLVM fails to create the global constant (e.g., out
    // of memory or invalid string content).

    fn visit_string(&mut self, expr: &mut StringExpr) -> Self::Result {
        let ptr = self.builder
            .build_global_string_ptr(&expr.value, "str")
            .map_err(|e| CompilerError::CodegenError {
                msg: format!("failed to create string constant: {}", e),
                span: Some(expr.span),
            })?;
        Ok(ptr.as_pointer_value().into())
    }

    /// Generates LLVM IR for a built‑in mathematical constant (`PI` or `E`).

    fn visit_const(&mut self, expr: &mut ConstExpr) -> Self::Result {
        // Get the LLVM type for 64‑bit floating‑point numbers 
        let f64_type = self.context.f64_type();

        // Map the constant name (`"PI"` or `"E"`) to its numeric value using Rust's built‑in constants.
        let value = match expr.name.as_str() {
            "PI" => std::f64::consts::PI,
            "E"  => std::f64::consts::E,
            _ => unreachable!("ICE: unknown constant '{}' escaped type checker", expr.name),
        };

        Ok(f64_type.const_float(value).into())
    }

    /// Generates LLVM IR for a boolean literal (`true` or `false`).
    
    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result {
        // Get the LLVM type for a 1‑bit integer (`i1`).
        let bool_type = self.context.bool_type();

        // Get the LLVM type for a 1‑bit integer (`i1`).
        let value = bool_type.const_int(if expr.value { 1 } else { 0 }, false);

        Ok(value.into())
    }

    /// Generates LLVM IR for the `print` expression in HULK.
     
    // This method delegates to the C standard library function `printf` to perform the
    // actual output. It selects the appropriate format specifier and argument conversion
    // based on the static type of the expression being printed.

    /// # Supported Types and Output Format
    /// - `String`   → printed as plain text followed by a newline (`%s\n`).
    /// - `Number`   → printed as a floating‑point number followed by a newline (`%f\n`).
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

    fn visit_print(&mut self, expr: &mut PrintExpr) -> Self::Result {

        // 1. Generate code for the argument expression.
        let value = expr.argument.accept(self)?;

        // 2. Retrieve the static type of the argument (inferred by the type checker).
        let arg_ty = expr.argument.get_type().ok_or_else(|| CompilerError::CodegenError {
            msg: "type not inferred for print argument".to_string(),
            span: Some(expr.span),
        })?;

        // 3. Obtain the `printf` function (declare it if not already present).
        let printf_fn = self.declare_printf();

        // 4. Handle each supported type.
        let format_str = match arg_ty {

            // 4a. Create (or reuse) a global constant for the format string "%s\n".
            HulkType::String | HulkType::Boolean => self.builder.build_global_string_ptr("%s\n", "fmt_str"),

            // 4b. Create a global constant for the format string `"%f\n"`.
            HulkType::Number => self.builder.build_global_string_ptr("%f\n", "fmt_num"),

            // Otherwise
            _ => return Err(CompilerError::CodegenError {
                msg: "cannot print this type".to_string(),
                span: Some(expr.span),
            }),
        }.map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;

        // 4b. Retrieve and convert the argument value
        let llvm_arg = if *arg_ty == HulkType::String {
            value.into_pointer_value().into()

        } else if *arg_ty == HulkType::Boolean {

            // For a boolean, we have an `i1` value. We need to select between the
            // global string constants "true" and "false" and pass the chosen pointer.
            let bool_val = value.into_int_value();
            
            let true_ptr = self.get_true_str();
            let false_ptr = self.get_false_str();

            // let value = if bool_val { true_ptr } else { false_ptr };

            // Use LLVM's `select` instruction: if bool_val is true → true_ptr, else false_ptr.
            let selected_ptr = self
                .builder
                .build_select(bool_val, true_ptr, false_ptr, "bool_str")
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: Some(expr.span),
                })?;

            // Convert the selected pointer to the generic argument type.
            selected_ptr.into()
        }else {
            // For Number, we have an `f64` value.
            value.into_float_value().into()
        };

        self.builder.build_call(printf_fn, &[format_str.as_pointer_value().into(), llvm_arg], "printf_call")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;

        Ok(value)
    }


    fn visit_binary_op(&mut self, expr: &mut BinaryOpExpr) -> Self::Result {


        let lhs_val = expr.left.accept(self)?;
        let rhs_val = expr.right.accept(self)?;


        match expr.op {

            // Arithmetic operators (expect Number, return Number)

            BinOp::Add => {

                let lhs = lhs_val.into_float_value();
                let rhs = rhs_val.into_float_value();

                let val = self.builder.build_float_add(lhs, rhs, "addtmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(val.into())
            }

            BinOp::Sub => {

                let lhs = lhs_val.into_float_value();
                let rhs = rhs_val.into_float_value();

                let val = self.builder.build_float_sub(lhs, rhs, "subtmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(val.into())
            }

            BinOp::Mul => {

                let lhs = lhs_val.into_float_value();
                let rhs = rhs_val.into_float_value();

                let val = self.builder.build_float_mul(lhs, rhs, "multmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(val.into())
            }

            BinOp::Div => {

                let lhs = lhs_val.into_float_value();
                let rhs = rhs_val.into_float_value();

                let val = self.builder.build_float_div(lhs, rhs, "divtmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(val.into())
            }

            BinOp::Pow => { //Revisar

                let lhs = lhs_val.into_float_value();
                let rhs = rhs_val.into_float_value();

                let pow_fn = self.module.get_function("llvm.pow.f64").unwrap_or_else(|| {
                    let f64 = self.context.f64_type();
                    let pow_type = f64.fn_type(&[f64.into(), f64.into()], false);
                    self.module.add_function("llvm.pow.f64", pow_type, None)
                });
                let call_site = self.builder.build_call(pow_fn, &[lhs.into(), rhs.into()], "powtmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                let result = call_site.try_as_basic_value().left()
                    .ok_or_else(|| CompilerError::CodegenError {
                        msg: "Failed to convert pow call to basic value".to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(result)
            }

            // Comparison operators (expect Number, return Boolean)
            
            BinOp::Lt | BinOp::Gt | BinOp::Leq  | BinOp::Geq => {

                let lhs = lhs_val.into_float_value();
                let rhs = rhs_val.into_float_value();

                let pred = match expr.op {
                    BinOp::Lt => inkwell::FloatPredicate::OLT,
                    BinOp::Gt => inkwell::FloatPredicate::OGT,
                    BinOp::Leq => inkwell::FloatPredicate::OLE,
                    BinOp::Geq => inkwell::FloatPredicate::OGE,
                    _ => unreachable!(),
                };

                let result = self.builder.build_float_compare(pred, lhs, rhs, "cmptmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(result.into())
            }

            // Logical AND/OR: both operands are guaranteed to be Boolean (i1) by the type checker.

            BinOp::And => {
                let lhs = lhs_val.into_int_value();
                let rhs = rhs_val.into_int_value();
                
                let val = self.builder.build_and(lhs, rhs, "andtmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(val.into())
            }

            BinOp::Or => {
                let lhs = lhs_val.into_int_value();
                let rhs = rhs_val.into_int_value();
                let val = self.builder.build_or(lhs, rhs, "ortmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(val.into())
            }

            BinOp::Eq | BinOp::Neq => {

                let ty = expr.left.get_type().ok_or_else(|| CompilerError::CodegenError {
                    msg: "type not inferred for left operand".to_string(),
                    span: Some(expr.span),
                })?;

                match ty {

                    HulkType::Number => {
                        let lhs = lhs_val.into_float_value();
                        let rhs = rhs_val.into_float_value();

                        let pred = if matches!(expr.op, BinOp::Eq) {
                        inkwell::FloatPredicate::OEQ
                        } else {
                            inkwell::FloatPredicate::ONE
                        };

                        let val = self.builder.build_float_compare(pred, lhs, rhs, "eqcmp")
                        .map_err(|e| CompilerError::CodegenError {
                            msg: e.to_string(),
                            span: Some(expr.span),
                        })?;

                        Ok(val.into())
                    }

                    HulkType::Boolean => {
                        let lhs = lhs_val.into_int_value();
                        let rhs = rhs_val.into_int_value();

                        let pred = if matches!(expr.op, BinOp::Eq) {
                        inkwell::IntPredicate::EQ
                        } else {
                            inkwell::IntPredicate::NE
                        };

                        let val = self.builder.build_int_compare(pred, lhs, rhs, "eqcmp")
                        .map_err(|e| CompilerError::CodegenError {
                            msg: e.to_string(),
                            span: Some(expr.span),
                        })?;

                        Ok(val.into())
                    }

                    HulkType::String => { //Revisar
                        // For strings we use the standard C library function `strcmp`.
                        let strcmp_fn = self.declare_strcmp();
                        let lhs_ptr = lhs_val.into_pointer_value();
                        let rhs_ptr = rhs_val.into_pointer_value();

                        let call_site = self.builder
                            .build_call(strcmp_fn, &[lhs_ptr.into(), rhs_ptr.into()], "strcmp")
                            .map_err(|e| CompilerError::CodegenError {
                                msg: e.to_string(),
                                span: Some(expr.span),
                            })?;
                        let cmp_result = call_site.try_as_basic_value().left()
                            .and_then(|v| v.into_int_value().into())
                            .ok_or_else(|| CompilerError::CodegenError {
                                msg: "strcmp did not return an integer".to_string(),
                                span: Some(expr.span),
                            })?;

                        // strcmp returns 0 if equal, <0 or >0 if different.
                        // We need to convert that to a boolean (i1).
                        let zero = self.context.i32_type().const_int(0, false);
                        let val = if matches!(expr.op, BinOp::Eq) {
                            self.builder.build_int_compare(inkwell::IntPredicate::EQ, cmp_result, zero, "streq")
                        } else {
                            self.builder.build_int_compare(inkwell::IntPredicate::NE, cmp_result, zero, "strne")
                        }.map_err(|e| CompilerError::CodegenError {
                            msg: e.to_string(),
                            span: Some(expr.span),
                        })?;
                        Ok(val.into())
                    }

                    _ => Err(CompilerError::CodegenError {
                        msg: format!("equality not implemented for type {:?}", ty),
                        span: Some(expr.span),
                    }),
                }

            }

            BinOp::Concat => Err(CompilerError::CodegenError {
                msg: "Not yet implemented".to_string(),
                span: Some(expr.span),
            }),

        }
    }

    /// Generates LLVM IR for a unary operation.

    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result {
        let operand = expr.expr.accept(self)?;
        match expr.op {
            UnaryOp::Not => {

                // # Logical NOT implementation
                // Since LLVM does not have a dedicated "logical not" instruction, we implement it
                // using the XOR (`xor`) instruction with the constant `1`.
                //   - `1 xor 1 = 0`  (true  → false)
                //   - `0 xor 1 = 1`  (false → true)

                let operand_bool = operand.into_int_value();

                // Obtain the LLVM `i1` type and create a constant integer `1` of type `i1`.
                let bool_type = self.context.bool_type();
                let one = bool_type.const_int(1, false);

                // Build the XOR instruction: 
                let result = self.builder.build_xor(operand_bool, one, "nottmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(result.into())
            }
            UnaryOp::Neg => {

            let operand_float = operand.into_float_value();
            let result = self.builder.build_float_neg(operand_float, "negtmp")
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: Some(expr.span),
                })?;
            Ok(result.into())}
        }
    }
    
    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result {

        // The type checker guarantees that the function exists and the arguments are valid.
        match expr.func.as_str() {

            // 1‑argument mathematical functions (sin, cos, sqrt, exp)
            "sin" | "cos" | "sqrt" | "exp" => {
                // Evaluate the single argument. The type checker ensures it is a Number.
                let arg = expr.args[0].accept(self)?.into_float_value();

                // Select the appropriate external C function.
                let func = match expr.func.as_str() {
                    "sin"  => self.declare_sin(),
                    "cos"  => self.declare_cos(),
                    "sqrt" => self.declare_sqrt(),
                    "exp"  => self.declare_exp(),
                    _ => unreachable!(),
                };

                // Build the call: `call f64 @sin(f64 %arg)`
                let call_site = self.builder
                    .build_call(func, &[arg.into()], "calltmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;

                // Extract the returned float value.
                let result = call_site.try_as_basic_value().left()
                    .ok_or_else(|| CompilerError::CodegenError {
                        msg: format!("{} call did not return a value", expr.func),
                        span: Some(expr.span),
                    })?;

                Ok(result.into())
            }

            // `rand()` – 0 arguments, returns a random integer cast to f64.
            "rand" => {
                let rand_fn = self.declare_rand();
                let call_site = self.builder
                    .build_call(rand_fn, &[], "randtmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                let rand_int = call_site.try_as_basic_value().left()
                    .and_then(|v| v.into_int_value().into())
                    .ok_or_else(|| CompilerError::CodegenError {
                        msg: "rand call did not return an integer".to_string(),
                        span: Some(expr.span),
                    })?;

                // Convert the i32 to f64 using a signed integer to float cast.
                let f64_type = self.context.f64_type();
                let rand_float = self.builder
                    .build_signed_int_to_float(rand_int, f64_type, "randf64")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;

                // Divide by `RAND_MAX` to normalize the value to the interval [0, 1].
                // `RAND_MAX` is typically 2147483647 on glibc systems.
                let rand_max = f64_type.const_float(2147483647.0);
                let result = self.builder
                    .build_float_div(rand_float, rand_max, "rand_norm")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;

                Ok(result.into())
            }

            // `log` – 2 arguments (base, value).
            "log" => {
                
                // logarithm with specified base: log(base, value) = log(value) / log(base)
                let base = expr.args[0].accept(self)?.into_float_value();
                let value = expr.args[1].accept(self)?.into_float_value();
                let log_fn = self.declare_log();

                // Compute log(value)
                let log_val = self.builder
                    .build_call(log_fn, &[value.into()], "log_val")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?
                    .try_as_basic_value().left()
                    .and_then(|v| v.into_float_value().into())
                    .ok_or_else(|| CompilerError::CodegenError {
                        msg: "log(value) call failed".to_string(),
                        span: Some(expr.span),
                    })?;

                // Compute log(base)
                let log_base = self.builder
                    .build_call(log_fn, &[base.into()], "log_base")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?
                    .try_as_basic_value().left()
                    .and_then(|v| v.into_float_value().into())
                    .ok_or_else(|| CompilerError::CodegenError {
                        msg: "log(base) call failed".to_string(),
                        span: Some(expr.span),
                    })?;

                // Divide: log(value) / log(base)
                let result = self.builder.build_float_div(log_val, log_base, "log_result")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(result.into())
            }

            _ => unreachable!("ICE: unknown built‑in function '{}' escaped type checker", expr.func),
        }
    }
        
}