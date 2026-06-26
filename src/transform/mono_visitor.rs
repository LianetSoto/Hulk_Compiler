use super::MonomorphizationPass;
use crate::ast::*;
use crate::semantic::HulkType;
use crate::error::CompilerError;
use std::collections::HashMap;

impl Visitor for MonomorphizationPass {
    type Result = Result<(), CompilerError>;

    fn visit_program(&mut self, program: &mut Program) -> Self::Result {
        // Transform the main expression (may call generic functions)
        program.main_expr.accept(self)?;

        // Transform all concrete function bodies (they may also call generic ones)
        let concrete_names: Vec<String> = program.functions
            .iter()
            .filter(|f| !f.is_generic)
            .map(|f| f.name.clone())
            .collect();
        for name in concrete_names {
            if let Some(func) = program.functions.iter_mut().find(|f| f.name == name) {
                func.accept(self)?;
            }
        }

        // Traverse type definitions – their attribute initializers and method bodies
        //    may also contain generic function calls.
        for type_def in &mut program.types {
            type_def.accept(self)?;
        }

        // Append all newly specialized functions to the program
        for (_, func) in self.specialized.drain() {
            program.functions.push(func);
        }

        // Remove the original generic definitions (they are no longer needed)
        program.functions.retain(|f| !f.is_generic);

        Ok(())
    }

    /// This method is the core of monomorphisation. It transforms every call to a generic
    /// function (e.g., `foo(5)`) into a call to a concrete specialised version
    /// (e.g., `foo$Number(5)`). The concrete version is created on‑demand and cached.
    
    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result {
        // 1. Visit arguments recursively – inner calls are specialised first.
        for arg in &mut expr.args {
            arg.accept(self)?;
        }

        // 2. Look up the called function – either from the original generic definitions
        //    or from already created specialisations.
        let func_def = if let Some(f) = self.original_functions.get(&expr.func) {
            f.clone()
        } else if let Some(f) = self.specialized.get(&expr.func) {
            f.clone()
        } else {
            // Built‑in function (e.g., print) – nothing to monomorphize.
            return Ok(());
        };

        // 3. Only handle generic functions.
        if func_def.is_generic {
            // 4. Collect the concrete argument types (already annotated by the semantic analyser).
            let concrete_arg_types: Vec<HulkType> = expr.args
                .iter()
                .map(|a| a.get_type().expect("Argument without type").clone())
                .collect();

            // 5. Generate a mangled name for this specialization (e.g., "foo$Number").
            let mangled_name = Self::mangle_specialization(&expr.func, &concrete_arg_types);

            // 6. Recursion guard: if we are already instantiating this same specialization,
            //    we have a recursive cycle. Report an error and stop.
            if self.instantiating.contains(&mangled_name) {
                return Err(CompilerError::MonomorphizationError{
                    msg: format!(
                        "infinite recursion detected while instantiating `{}`",
                        mangled_name
                    ),
                    span: expr.span,
                });
            }

            // 7. If the specialization does not exist yet, create it.
            if !self.specialized.contains_key(&mangled_name) {
                // Mark this specialization as being processed to detect cycles.
                self.instantiating.insert(mangled_name.clone());

                // Clone the original generic function.
                let mut concrete_func = func_def.clone();

                // Convert the clone into a concrete version: rename and clear generic flag.
                concrete_func.name = mangled_name.clone();
                concrete_func.is_generic = false;

                // Build a substitution map (type variable id → concrete type) based on
                // the order of parameters and the concrete argument types.
                let mut subst_map = HashMap::new();
                for (i, param) in func_def.params.iter().enumerate() {
                    if let Some(HulkType::Var(id)) = param.ty.as_ref() {
                        subst_map.insert(*id, concrete_arg_types[i].clone());
                    }
                }

                // Replace all type variables in the function by the concrete types.
                Self::substitute_in_function(&mut concrete_func, &subst_map);

                // Recursively specialise any calls inside the body of this new concrete function.
                concrete_func.accept(self)?;

                // Store the specialization for future reuse.
                self.specialized.insert(mangled_name.clone(), concrete_func.clone());

                // Remove from the recursion guard set now that processing is complete.
                self.instantiating.remove(&mangled_name);
            }

            // 8. Redirect the original call to the concrete specialization.
            expr.func = mangled_name;
        }

        Ok(())
    }

    // Traversal methods 
    // 
    /// These methods simply recurse into the sub‑expressions of the node.
    /// They do not perform any transformation themselves; the only
    /// transformation happens inside `visit_call`.
    /// This ensures that the entire AST is traversed and every call to a
    /// generic function is eventually processed.

    fn visit_number(&mut self, _: &mut NumberExpr) -> Self::Result { Ok(()) }
    fn visit_string(&mut self, _: &mut StringExpr) -> Self::Result { Ok(()) }
    fn visit_bool(&mut self, _: &mut BoolExpr) -> Self::Result { Ok(()) }
    fn visit_const(&mut self, _: &mut ConstExpr) -> Self::Result { Ok(()) }
    fn visit_variable(&mut self, _: &mut VariableExpr) -> Self::Result { Ok(()) }
    fn visit_self(&mut self, _: &mut SelfExpr) -> Self::Result { Ok(()) }
    fn visit_base(&mut self, _: &mut BaseExpr) -> Self::Result { Ok(()) }

    fn visit_binary_op(&mut self, expr: &mut BinaryOpExpr) -> Self::Result {
        expr.left.accept(self)?;
        expr.right.accept(self)?;
        Ok(())
    }

    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result {
        expr.expr.accept(self)?;
        Ok(())
    }

    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result {
        for (_, _,init) in &mut expr.bindings {
            init.accept(self)?;
        }
        expr.body.accept(self)?;
        Ok(())
    }

    fn visit_assign(&mut self, expr: &mut DestructiveAssignExpr) -> Self::Result {
        expr.lhs.accept(self)?;
        expr.value.accept(self)?;
        Ok(())
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {
        for e in &mut expr.expressions {
            e.accept(self)?;
        }
        Ok(())
    }

    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        expr.condition.accept(self)?;
        expr.then_branch.accept(self)?;
        expr.else_branch.accept(self)?;
        Ok(())
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        expr.condition.accept(self)?;
        expr.body.accept(self)?;
        Ok(())
    }

    fn visit_function_def(&mut self, func: &mut FunctionDef) -> Self::Result {
        func.body.accept(self)?;
        Ok(())
    }

    // Type‑related nodes 

    fn visit_type_def(&mut self, type_def: &mut TypeDef) -> Self::Result {
        // Visit every attribute initializer
        for attr in &mut type_def.attributes {
            attr.accept(self)?;
        }
        // Visit every method body
        for method in &mut type_def.methods {
            method.accept(self)?;
        }
        Ok(())
    }

    fn visit_attribute(&mut self, attr: &mut Attribute) -> Self::Result {
        attr.init_expr.accept(self)?;
        Ok(())
    }

    fn visit_new(&mut self, expr: &mut NewExpr) -> Self::Result {
        for arg in &mut expr.args {
            arg.accept(self)?;
        }
        Ok(())
    }

    fn visit_method_call(&mut self, expr: &mut MethodCallExpr) -> Self::Result {
        expr.object.accept(self)?;
        for arg in &mut expr.args {
            arg.accept(self)?;
        }
        Ok(())
    }

    fn visit_attribute_access(&mut self, expr: &mut AttributeAccessExpr) -> Self::Result {
        expr.object.accept(self)?;
        Ok(())
    }

    fn visit_method(&mut self, m: &mut Method) -> Self::Result {
        m.body.accept(self)?;
        Ok(())
    }

    fn visit_is(&mut self, method: &mut IsExpr) -> Self::Result {
        method.expr.accept(self)?;  
        Ok(())
    }

    fn visit_as(&mut self, method: &mut AsExpr) -> Self::Result {
        method.expr.accept(self)?;  
        Ok(())
    }
    
    fn visit_protocol_def(&mut self, proto: &mut ProtocolDef) -> Self::Result {
        todo!()
    }
    
    fn visit_protocol_method(&mut self, method: &mut ProtocolMethod) -> Self::Result {
        todo!()
    }

    fn visit_for(&mut self, method: &mut ForExpr) -> Self::Result {
        todo!()
    }
}