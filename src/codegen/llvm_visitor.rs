use super::llvm::LlvmCodeGen;
use crate::ast::*;
use crate::error::CompilerError;
use inkwell::values::BasicValueEnum;
use crate::semantic::HulkType;
use inkwell::types::{BasicTypeEnum};

impl<'ctx> Visitor for LlvmCodeGen<'ctx> {
    type Result = Result<BasicValueEnum<'ctx>, CompilerError>;

    /// Generates LLVM IR for the entire HULK program.
    fn visit_program(&mut self, program: &mut Program) -> Self::Result {

        // Register every user‑defined type and declare its methods
        for type_def in &mut program.types {

            // Build the LLVM struct for this type 
            type_def.accept(self)?;  

            // Declare the LLVM function for every method.
            let owner = &type_def.name;
            let struct_ty = self.type_structs[owner];
            let self_ptr_ty: BasicTypeEnum = struct_ty
                .ptr_type(inkwell::AddressSpace::default())
                .into();

            for method in &type_def.methods {
                let func_name = format!("{}.{}", owner, method.name);

                let mut param_types = vec![self_ptr_ty];
                for p in &method.params {
                    let hulk_ty = p.ty.as_ref().unwrap();
                    param_types.push(self.hulk_type_to_llvm_type(hulk_ty)?);
                }

                let ret_ty = self.hulk_type_to_llvm_type(
                    method.ty.as_ref().unwrap_or(&HulkType::Number),
                )?;

                self.declare_function_generic(&func_name, ret_ty, &param_types);
            }
        }
    
        // Declare all global function (built‑ins and user‑defined)
        let user_map = self.declare_all_functions(&program.functions)?;
        self.user_functions = user_map;

        // Compile the bodies of all global functions
        for func in &mut program.functions {
            func.accept(self)?;
        }

        // Compile the bodies of all type-methods
        for type_def in &mut program.types {
            for method in &mut type_def.methods {
                method.type_name = Some(type_def.name.clone()); 
                method.accept(self)?;   
            }
        }

        // Generate the `main` entry point
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

        let params: Vec<(String, BasicTypeEnum<'ctx>)> = func.params.iter()
        .map(|p| {
            let hulk_ty = p.ty.as_ref().unwrap();
            let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?; 
            Ok((p.name.clone(), llvm_ty))
        })
        .collect::<Result<_, CompilerError>>()?;

        self.compile_llvm_function(function, params, &mut func.body, func.span)?;

        Ok(self.context.f64_type().const_float(0.0).into())
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
        // Special cases – functions that require custom handling
        match expr.func.as_str() {
            "rand" => return self.compile_rand_call(expr),
            "log"  => return self.compile_log_call(expr),
            "print" => return self.compile_print_call(expr),
            _ => { /* continue with generic lookup */ }
        }

        // Generic lookup in the LLVM module (simple built-ins + user functions)
        let func = self.module.get_function(&expr.func)
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!("undefined function '{}'", expr.func),
                span: Some(expr.span),
            })?;

        // Compile each argument expression recursively
        let mut args = Vec::new();
        for arg in &mut expr.args {
            args.push(arg.accept(self)?.into());
        }

        // Build the LLVM call instruction
        let call_site = self.builder.build_call(func, &args, "calltmp")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // The call returns a value; extract it (left side of the call result)
        Ok(call_site.try_as_basic_value().left().unwrap().into())
    }

    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result {
        // Introduce a new variable scope
        self.push_scope();

        // Process bindings left‑to‑right so later initializers can see earlier variables
        for (name, _, init_expr) in &mut expr.bindings {
            // Evaluate the initializer expression
            let init_val = init_expr.accept(self)?;

            // Determine the LLVM type from the inferred HULK type (stored in the AST node)
            let hulk_ty = init_expr.get_type()
                .ok_or_else(|| CompilerError::CodegenError {
                    msg: "type not inferred for let binding".to_string(),
                    span: Some(expr.span),
                })?;
            let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?;

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
                let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?;

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
        // 1. Get a pointer to the storage location (lvalue)
        let ptr = self.eval_lvalue(&mut *expr.lhs)?;

        // 2. Evaluate the right‑hand side
        let new_val = expr.value.accept(self)?;

        // 3. Store the new value into the location
        self.builder.build_store(ptr, new_val)
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        // 4. Assignment returns the assigned value (HULK semantics)
        Ok(new_val.into())
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
        let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?;

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
        let default_val = self.default_value_for_type(hulk_ty)?;
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
        let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?; 

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

    // Types

    fn visit_type_def(&mut self, type_def: &mut TypeDef) -> Self::Result {

        let struct_ty = self.build_struct_type(type_def)?;

        self.type_structs.insert(type_def.name.clone(), struct_ty);
        self.type_defs.insert(type_def.name.clone(), type_def.clone()); 

        Ok(self.context.f64_type().const_float(0.0).into())
    }
    
    /// Generates LLVM IR for the `new` expression 
    /// Allocates a heap object, initializes its fields, and returns a typed pointer.
    fn visit_new(&mut self, expr: &mut NewExpr) -> Self::Result {

        // Look up the type definition and the LLVM struct
        let type_def_ref = self.type_defs.get(&expr.type_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("unknown type '{}'", expr.type_name),
                span: Some(expr.span),
            }
        })?;
        let mut type_def = type_def_ref.clone();                    // mutable copy for attribute iteration
        let struct_ty = self.type_structs[&expr.type_name]; // LLVM struct type (e.g., %Point)

        // Allocate raw memory with malloc
        let malloc_fn = self.declare_malloc();
        let size_val = struct_ty.size_of()
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!("unable to determine size of type '{}'", expr.type_name),
                span: Some(expr.span),
            })?;
        let obj_ptr = self.builder.build_call(malloc_fn, &[size_val.into()], "new_obj")
            .map_err(|e| CompilerError::CodegenError {
                msg: format!("malloc failed: {}", e),
                span: Some(expr.span),
            })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_pointer_value().into())
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "malloc did not return a pointer".to_string(),
                span: Some(expr.span),
            })?;

        // Cast the raw i8* to the concrete struct pointer
        let typed_ptr = self.builder.build_pointer_cast(
            obj_ptr,
            struct_ty.ptr_type(inkwell::AddressSpace::default()),
            "typed_obj",
        )
        .map_err(|e| CompilerError::CodegenError {
            msg: format!("bitcast failed: {}", e),
            span: Some(expr.span),
        })?;

        // Push a scope for constructor arguments 
        self.push_scope();

        // Evaluate constructor arguments and store them in local variables
        for (param_name, arg_expr) in type_def.params.iter().zip(&mut expr.args) {
            let arg_val = arg_expr.accept(self)?;
            let hulk_ty = arg_expr.get_type().ok_or_else(|| CompilerError::CodegenError {
                msg: "type not inferred for argument".to_string(),
                span: Some(expr.span),
            })?;
            let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?;
            let alloca = self.builder.build_alloca(llvm_ty, param_name)
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: Some(expr.span),
                })?;
            self.builder.build_store(alloca, arg_val)
                .map_err(|e| CompilerError::CodegenError {
                    msg: e.to_string(),
                    span: Some(expr.span),
                })?;
            self.insert_var(param_name.clone(), alloca);
        }

        // Initialize each attribute using struct_gep + store 
        for (i, attr) in type_def.attributes.iter_mut().enumerate() {
            let init_val = attr.init_expr.accept(self)?;
            let field_ptr = self.builder.build_struct_gep(
                struct_ty,
                typed_ptr,
                i as u32,
                &format!("field_{}", i),
            )
            .map_err(|e| CompilerError::CodegenError {
                msg: format!("GEP failed: {}", e),
                span: Some(expr.span),
            })?;
            self.builder.build_store(field_ptr, init_val)
                .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(expr.span) })?;
        }

        // Pop the argument scope and return the typed pointer 
        self.pop_scope();
        Ok(typed_ptr.into())
    }

    fn visit_attribute_access(&mut self, expr: &mut AttributeAccessExpr) -> Self::Result {
        // Get a pointer to the field and its HULK type.
        let (ptr, attr_hulk_ty) = self.get_attribute_ptr(expr)?;

        // Convert the HULK type to the corresponding LLVM type.
        let llvm_ty = self.hulk_type_to_llvm_type(&attr_hulk_ty)?;

        // Load the value from the field.
        let loaded = self.builder.build_load(llvm_ty, ptr, &expr.attribute)
            .map_err(|err| CompilerError::CodegenError {
                msg: err.to_string(),
                span: Some(expr.span),
            })?;

        Ok(loaded.into())
    }
    
    /// Compiles the body of a method belonging to a user‑defined type.
    ///
    /// Methods are lowered to regular LLVM functions whose name follows the
    /// convention `{TypeName}.{methodName}` (e.g. `Point.getX`).  The first
    /// parameter of every such function is always a pointer to the struct
    /// that represents the receiver (`self`).  This method recovers the
    /// already‑declared `FunctionValue` from the module, builds the
    /// parameter list expected by `compile_llvm_function` (starting with
    /// `self` followed by the declared method parameters), and delegates
    /// the actual body compilation to that shared helper.
    ///
    /// # Conventions
    /// - The method must have been declared in a previous pass (e.g. during
    ///   `visit_program`) with the name `{TypeName}.{methodName}`.
    /// - The first argument of the LLVM function is `%TypeName*` (the
    ///   receiver object).
    /// - Subsequent arguments correspond to the method's formal parameters
    ///   in declaration order.
    ///
    /// # Returns
    /// A dummy `BasicValueEnum` (constant `f64 0.0`) because the return
    /// value of `Visitor::visit_method` is not used outside this call;
    /// the actual return value of the method is emitted by the `ret`
    /// instruction inside `compile_llvm_function`.
    fn visit_method(&mut self, method: &mut Method) -> Self::Result {
      
        let owner_type = method.type_name.as_ref().unwrap();
        let func_name = format!("{}.{}", owner_type, method.name);
        let func = self.module.get_function(&func_name).expect("undeclared method");
        
        let mut params = Vec::new();
     
        let struct_ty = self.type_structs[owner_type];
        let ptr_ty = struct_ty.ptr_type(inkwell::AddressSpace::default()).into();
        params.push(("self".to_string(), ptr_ty));
        for param in &method.params {
            let llvm_ty = self.hulk_type_to_llvm_type(param.ty.as_ref().unwrap())?;
            params.push((param.name.clone(), llvm_ty));
        }

        self.compile_llvm_function(func, params, &mut method.body, method.span)?;
        Ok(self.context.f64_type().const_float(0.0).into())
    }
    
    /// Generates LLVM IR for a method call expression (`obj.method(args)`).
    ///
    /// In HULK every method receives an implicit first argument `self` that
    /// points to the instance.  This method therefore:
    /// 1. Evaluates the object expression to obtain a pointer to the struct.
    /// 2. Determines the concrete type of the object (using the type annotation
    ///    left by the `TypeChecker`).
    /// 3. Looks up the LLVM function whose name follows the convention
    ///    `{TypeName}.{methodName}` (e.g. `Point.getX`).
    /// 4. Builds the argument list: first the object pointer, then the
    ///    remaining method arguments (evaluated recursively).
    /// 5. Emits a `call` instruction and returns the result.
    ///
    /// # Errors
    /// - If the type of the object cannot be determined.
    /// - If the method is not found in the LLVM module.
    /// - If any argument fails to compile.
    fn visit_method_call(&mut self, expr: &mut MethodCallExpr) -> Self::Result {

        // 1. Evaluate the object expression
        let obj_val = expr.object.accept(self)?;

        // 2. Retrieve the static type of the object (HULK type)
        let hulk_ty = expr.object.get_type().ok_or_else(|| CompilerError::CodegenError {
            msg: "type not inferred for object".to_string(),
            span: Some(expr.span),
        })?;

        // Extract the class name.  We assume HULK types for user‑defined
        // classes are represented as `HulkType::Class(name)`.
        let type_name = match hulk_ty {
            HulkType::Class(name) => name.as_str(),
            _ => {
                return Err(CompilerError::CodegenError {
                    msg: format!("expected an object type, found {:?}", hulk_ty),
                    span: Some(expr.span),
                });
            }
        };

        // 3. Look up the LLVM function for the method
        let func_name = format!("{}.{}", type_name, expr.method);
        let func = self.module.get_function(&func_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("method '{}' not found in type '{}'", expr.method, type_name),
                span: Some(expr.span),
            }
        })?;

        // 4. Build the argument list: self + user arguments
        let obj_ptr = obj_val.into_pointer_value();
        let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = vec![obj_ptr.into()];

        for arg in &mut expr.args {
            args.push(arg.accept(self)?.into());
        }

        // 5. Emit the call
        let call_site = self.builder.build_call(func, &args, "calltmp")
            .map_err(|e| CompilerError::CodegenError {
                msg: e.to_string(),
                span: Some(expr.span),
            })?;

        Ok(call_site.try_as_basic_value().left().unwrap().into())
    }
    
    /// Generates LLVM IR for the `self` expression.
    ///
    /// `self` may refer either to the current instance (a pointer to the
    /// object) or to a local variable that shadows the original `self`.
    fn visit_self(&mut self, e: &mut SelfExpr) -> Self::Result {

        // Look up the storage pointer for `self` in the current scope.
        let ptr = self
            .lookup_var("self")
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "self variable not found in current scope".to_string(),
                span: Some(e.span),
            })?;

        // Retrieve the HULK type that the TypeChecker assigned to this
        // occurrence of `self`.
        let hulk_ty = e.ty.as_ref().ok_or_else(|| CompilerError::CodegenError {
            msg: "type not inferred for self".to_string(),
            span: Some(e.span),
        })?;

        // Convert the HULK type to its LLVM representation.
        let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?;

        // Load the value from the storage pointer.
        let loaded = self
            .builder
            .build_load(llvm_ty, ptr, "self")
            .map_err(|err| CompilerError::CodegenError {
                msg: err.to_string(),
                span: Some(e.span),
            })?;

        Ok(loaded.into())
    }
    
    fn visit_base(&mut self, e: &mut BaseExpr) -> Self::Result {
        todo!()
    }

    fn visit_attribute(&mut self, attr: &mut Attribute) -> Self::Result {
        todo!()
    }

    fn visit_protocol_def(&mut self, _proto: &mut ProtocolDef) -> Self::Result {
    // La generación de código para protocolos no está implementada aún.
    // Por ahora no hacemos nada.
    todo!()
}

fn visit_protocol_method(&mut self, _method: &mut ProtocolMethod) -> Self::Result {
    todo!()
}

fn visit_base_call(&mut self, _expr: &mut BaseCallExpr) -> Self::Result {
    // Stub: no se usa porque no generamos código de base()
    todo!()
}
}