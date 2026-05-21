use super::llvm::LlvmCodeGen;
use crate::ast::*;
use crate::error::CompilerError;
use inkwell::values::BasicValueEnum;
use crate::semantic::HulkType;
use std::collections::HashMap;

impl<'ctx> Visitor for LlvmCodeGen<'ctx> {
    type Result = Result<BasicValueEnum<'ctx>, CompilerError>;

    /// Generates LLVM IR for the entire HULK program.
    fn visit_program(&mut self, program: &mut Program) -> Self::Result {

        // Declare every function (built‑ins and user‑defined) in the module.
        let user_map = self.declare_all_functions(&program.functions);
        self.user_functions = user_map;

        // Compile each user function body.
        for func in &mut program.functions {
            func.accept(self)?;
        }

        // entry point (main) 
        let i32_type = self.context.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_type, None);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);
        self.current_function = Some(main_fn);

        // Seed the C random generator so that `rand()` is non‑deterministic.
        self.seed_random_generator()?;

        // Compile the global entry‑point expression.
        let _ = program.main_expr.accept(self)?;

        let zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&zero))
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        self.current_function = None;
        Ok(zero.into())
    }

    /// Compiles the body of a user‑defined function.
    fn visit_function_def(&mut self, func: &mut FunctionDef) -> Self::Result {

        // Retrieve the LLVM function value that was registered in the first pass.
        let function = self.user_functions[&func.name];

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Remember the current scope chain and the enclosing function, then set
        // up a fresh scope for the function's own variables.
        let old_scopes = std::mem::take(&mut self.scopes);
        self.scopes = vec![HashMap::new()];
        let old_function = self.current_function.replace(function);

        // Allocate stack storage for each parameter and store the incoming value.
        // The type checker guarantees that every parameter has an inferred type.
        for (i, param) in func.params.iter().enumerate() {
            let param_val = function.get_nth_param(i as u32).unwrap();
            let hulk_ty = param.ty.as_ref().expect("parameter type not inferred");
            let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty);
            let alloca = self.builder.build_alloca(llvm_ty, &param.name)
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(func.span) })?;
            self.builder.build_store(alloca, param_val)
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(func.span) })?;
            self.insert_var(param.name.clone(), alloca);
        }

        // Generate the IR for the body expression.
        let body_val = func.body.accept(self)?;

        // The result of the function is the value produced by its body.
        self.builder.build_return(Some(&body_val))
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(func.span),
            })?;

        // Restore the previous scope chain and the surrounding function context.
        self.scopes = old_scopes;
        self.current_function = old_function;

        Ok(body_val.into())
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

    /// Generates LLVM IR for a number literal
    fn visit_number(&mut self, expr: &mut NumberExpr) -> Self::Result {
        let f64_type = self.context.f64_type();
        Ok(f64_type.const_float(expr.value).into())
    }

    /// Generates LLVM IR for a boolean literal (`true` or `false`).
    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result {
        // Get the LLVM type for a 1‑bit integer (`i1`).
        let bool_type = self.context.bool_type();

        // Get the LLVM type for a 1‑bit integer (`i1`).
        let value = bool_type.const_int(if expr.value { 1 } else { 0 }, false);

        Ok(value.into())
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

            BinOp::Mod => {
                let lhs = lhs_val.into_float_value();
                let rhs = rhs_val.into_float_value();

                let fmod_fn = self.declare_fmod();
                let call_site = self.builder
                    .build_call(fmod_fn, &[lhs.into(), rhs.into()], "modtmp")
                    .map_err(|e| CompilerError::CodegenError {
                        msg: e.to_string(),
                        span: Some(expr.span),
                    })?;
                let result = call_site.try_as_basic_value().left()
                    .ok_or_else(|| CompilerError::CodegenError {
                        msg: "fmod call did not return a value".to_string(),
                        span: Some(expr.span),
                    })?;
                Ok(result.into())
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
        // Casos especiales – funciones que necesitan adaptación
        match expr.func.as_str() {
            "rand" => return self.compile_rand_call(expr),
            "log"  => return self.compile_log_call(expr),
            "print" => return self.compile_print_call(expr), 
            _ => { /* continuar con la búsqueda genérica */ }
        }

        // Búsqueda genérica en el módulo LLVM (built‑ins simples + usuario)
        let func = self.module.get_function(&expr.func)
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!("undefined function '{}'", expr.func),
                span: Some(expr.span),
            })?;

        let mut args = Vec::new();
        for arg in &mut expr.args {
            args.push(arg.accept(self)?.into());
        }

        let call_site = self.builder.build_call(func, &args, "calltmp")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        Ok(call_site.try_as_basic_value().left().unwrap().into())
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
        // match self.lookup_var(&expr.name) {
        //     Some(ptr) => {
        //         // Evaluate the right‑hand side
        //         let new_val = expr.value.accept(self)?;

        //         // Store the new value into the variable's location
        //         self.builder.build_store(ptr, new_val)
        //             .map_err(|e| CompilerError::CodegenError {
        //                 msg: e.to_string(),
        //                 span: Some(expr.span),
        //             })?;

        //         Ok(new_val)   // assignment returns the assigned value in HULK
        //     }
        //     None => Err(CompilerError::CodegenError {
        //         msg: format!("cannot assign to undefined variable '{}'", expr.name),
        //         span: Some(expr.span),
        //     }),
        // }
        todo!()
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
        
    /// Generates LLVM IR for an `if` expression.
    ///
    /// HULK `if` is an expression, so it produces a value.  Both branches
    /// (`then` and `else`) are mandatory.  The `elif` construct is desugared
    /// by the parser into nested `if` expressions, therefore this method only
    /// needs to handle a simple `if‑else`.
    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        // 1. Retrieve the HULK type inferred by the TypeChecker and convert
        //    it to the corresponding LLVM type (f64, i8*, i1, etc.).
        let hulk_ty = expr.ty.as_ref().ok_or_else(|| CompilerError::CodegenError {
            msg: "type not inferred for if expression".to_string(),
            span: Some(expr.span),
        })?;
        let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty); // returns BasicTypeEnum

        // 2. Create the basic blocks that form the control‑flow skeleton.
        let then_block = self.context.append_basic_block(
            self.current_function.unwrap(), "if.then");
        let else_block = self.context.append_basic_block(
            self.current_function.unwrap(), "if.else");
        let merge_block = self.context.append_basic_block(
            self.current_function.unwrap(), "if.merge");

        // 3. Evaluate the condition and emit a conditional branch.
        let cond_val = expr.condition.accept(self)?;
        let cond_i1 = cond_val.into_int_value();
        self.builder.build_conditional_branch(cond_i1, then_block, else_block)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // 4. "then" branch
        self.builder.position_at_end(then_block);
        let then_val = expr.then_branch.accept(self)?;
        // Capture the actual block that contains the then body. We store it so we
        // can feed the phi node later.
        let then_block_phi = self.builder.get_insert_block().unwrap();

        // Jump to the merge point.
        self.builder.build_unconditional_branch(merge_block)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // 5. "else" branch
        self.builder.position_at_end(else_block);
        let else_val = expr.else_branch.accept(self)?;
        let else_block_phi = self.builder.get_insert_block().unwrap(); // analogous to then
        self.builder.build_unconditional_branch(merge_block)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // 6. Merge block – build the phi instruction.
        //    A phi node selects a value depending on which basic block we
        //    came from.  We add one entry for each predecessor:
        //      - (then_val, then_block_phi)
        //      - (else_val, else_block_phi)
        self.builder.position_at_end(merge_block);
        let phi = self.builder.build_phi(llvm_ty, "if.phi")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        phi.add_incoming(&[
            (&then_val, then_block_phi),
            (&else_val, else_block_phi),
        ]);

        Ok(phi.as_basic_value().into())
    }   
    
    fn visit_type_def(&mut self, ty: &mut TypeDef) -> Self::Result {
        todo!()
    }
    
    fn visit_attribute(&mut self, attr: &mut Attribute) -> Self::Result {
        todo!()
    }
    
    fn visit_method(&mut self, m: &mut Method) -> Self::Result {
        todo!()
    }
    
    fn visit_new(&mut self, e: &mut NewExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_method_call(&mut self, e: &mut MethodCallExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_self(&mut self, e: &mut SelfExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_base(&mut self, e: &mut BaseExpr) -> Self::Result {
        todo!()
    }
    
    fn visit_attribute_access(&mut self, e: &mut expr::AttributeAccessExpr) -> Self::Result {
        todo!()
    }

  
}