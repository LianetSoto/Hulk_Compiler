use super::llvm::LlvmCodeGen;
use crate::ast::*;
use crate::error::CompilerError;
use inkwell::values::BasicValueEnum;
use crate::semantic::HulkType;

impl<'ctx> Visitor for LlvmCodeGen<'ctx> {
    type Result = Result<BasicValueEnum<'ctx>, CompilerError>;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result {
        
        let i32_type = self.context.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_type, None);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);
        self.current_function = Some(main_fn);

        // Seed the random number generator 
        self.seed_random_generator()?;

       
        for stmt in &mut program.statements {
            stmt.accept(self)?;
        }

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
    
    // This method emits a global constant
    // containing the string's bytes (UTF-8 + null terminator) and returns a pointer
    // to that constant. The returned pointer can be used wherever a string is expected

    // # Steps
    // 1. Call `build_global_string_ptr` to create an LLVM global constant of type
    //    `[N x i8]` initialized with the string's UTF-8 bytes and a null terminator.
    //    This also returns an `i8*` pointer to the first byte.
    // 2. Convert the returned `GlobalValue` to a `PointerValue` and then to the
    //    generic `BasicValueEnum` expected by the visitor.

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

            // 4b. Create a global constant for the format string `"%g\n"`.
            HulkType::Number => self.builder.build_global_string_ptr("%g\n", "fmt_num"),

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

            BinOp::Concat | BinOp::ConcatSpace=> {
                let sep = if let BinOp::ConcatSpace = expr.op {
                    Some(" ")
                } else {
                    None
                };
                let result_ptr = self.concat_strings(lhs_val, rhs_val, sep, expr.span)?;
                Ok(result_ptr.into())
            }

            BinOp::Mod => todo!()

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

    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result {
        // Introduce a new variable scope
        self.push_scope();

        // Process bindings left‑to‑right so later initializers can see earlier variables
        for (name, init_expr) in &mut expr.bindings {
            // Evaluate the initializer expression
            let init_val = init_expr.accept(self)?;

            // Determine the LLVM type from the inferred HULK type (stored in the AST node)
            let hulk_ty = init_expr.get_type()
                .ok_or_else(|| CompilerError::CodegenError {
                    msg: "type not inferred for let binding".to_string(),
                    span: Some(expr.span),
                })?;
            let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty);

            // Allocate stack space for the variable
            let alloca = self.builder.build_alloca(llvm_ty, &name)
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: Some(expr.span),
                })?;

            // Store the initial value
            self.builder.build_store(alloca, init_val)
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: Some(expr.span),
                })?;

            // Register the variable in the current scope
            self.insert_var(name.clone(), alloca);
        }

        // Generate code for the body of the let expression
        let body_val = expr.body.accept(self)?;

        // Remove the scope introduced by this let
        self.pop_scope();

        Ok(body_val)
    }

    fn visit_variable(&mut self, expr: &mut VariableExpr) -> Self::Result {
        // Look up the variable in the scope stack
        match self.lookup_var(&expr.name) {
            Some(ptr) => {
                // Determine the type to load (use the annotated HULK type)
                let hulk_ty = expr.ty.as_ref().ok_or_else(|| CompilerError::CodegenError {
                    msg: format!("type not inferred for variable '{}'", expr.name),
                    span: Some(expr.span),
                })?;
                let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty);

                // Load the value from the pointer
                let value = self.builder.build_load(llvm_ty, ptr, &expr.name)
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(value.into())
            }
            None => Err(CompilerError::CodegenError {
                msg: format!("undefined variable '{}'", expr.name),
                span: Some(expr.span),
            }),
        }
    }

    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> Self::Result {
        // Look up the variable (TypeChecker already verified it exists)
        match self.lookup_var(&expr.name) {
            Some(ptr) => {
                // Evaluate the right‑hand side
                let new_val = expr.value.accept(self)?;

                // Store the new value into the variable's location
                self.builder.build_store(ptr, new_val)
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;

                Ok(new_val)   // assignment returns the assigned value in HULK
            }
            None => Err(CompilerError::CodegenError {
                msg: format!("cannot assign to undefined variable '{}'", expr.name),
                span: Some(expr.span),
            }),
        }
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {
        let mut last_value: Option<BasicValueEnum<'ctx>> = None;

        for e in &mut expr.expressions {
            last_value = Some(e.accept(self)?);
        }

        Ok(last_value.unwrap_or_else(|| self.context.f64_type().const_float(0.0).into()))
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        // Retrieve the static type of the while expression (already inferred by the
        // TypeChecker). It tells us what LLVM type to use for the accumulated value.
        let hulk_ty = expr.ty.as_ref().ok_or_else(|| CompilerError::CodegenError {
            msg: "type not inferred for while expression".to_string(),
            span: Some(expr.span),
        })?;
        let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty);

        // Create the basic blocks that will form the loop structure.
        let cond_block = self.context.append_basic_block(
            self.current_function.unwrap(),
            "while.cond",
        );
        let body_block = self.context.append_basic_block(
            self.current_function.unwrap(),
            "while.body",
        );
        let end_block = self.context.append_basic_block(
            self.current_function.unwrap(),
            "while.end",
        );

        // Allocate a stack slot for the loop's return value and initialise it with
        // a sensible default (0 for numbers, empty string for strings, false for
        // booleans).  This avoids needing a phi node and works for all primitive
        // types.
        let default_val = self.default_value_for_type(hulk_ty);
        let last_val_alloca = self.builder.build_alloca(llvm_ty, "while.last")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;
        self.builder.build_store(last_val_alloca, default_val)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // Jump from the current block into the condition block.
        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // ---- CONDITION BLOCK ----
        self.builder.position_at_end(cond_block);
        let cond_val = expr.condition.accept(self)?;
        let cond_i1 = cond_val.into_int_value(); // type checker guarantees Boolean

        self.builder.build_conditional_branch(cond_i1, body_block, end_block)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // ---- BODY BLOCK ----
        self.builder.position_at_end(body_block);
        let body_val = expr.body.accept(self)?;

        // Store the body value into the loop‑result slot (this captures the value
        // of the last iteration).
        self.builder.build_store(last_val_alloca, body_val)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // Jump back to the condition block.
        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // ---- END BLOCK ----
        self.builder.position_at_end(end_block);

        // Load the accumulated value (or the initial default if the loop never ran).
        let result = self.builder.build_load(llvm_ty, last_val_alloca, "while.result")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        Ok(result.into())
    }
    
    fn visit_if(&mut self, expr: &mut crate::ast::IfExpr) -> Self::Result {
        todo!()
    }
        
    fn visit_for(&mut self, expr: &mut crate::ast::ForExpr) -> Self::Result {
        todo!()
    }

}