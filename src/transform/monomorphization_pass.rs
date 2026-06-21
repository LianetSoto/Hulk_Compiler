//! Monomorphization transformation for Hulk's generic functions.
//!
//! This pass converts all generic functions (e.g., `fn foo<T>(x: T) -> T`) into concrete
//! specializations (e.g., `fn foo_Number(x: Number) -> Number`) by:
//! - Cloning the generic function definition for each unique set of concrete argument types.
//! - Replacing all occurrences of type variables (`Var(id)`) with the actual concrete types.
//! - Renaming the call sites to point to the specialized functions.
//! - Removing the original generic definitions (they are no longer needed for code generation).
//!
//! # Why monomorphization?
//! - **Performance**: No runtime dispatch overhead; each specialized function uses concrete types.
//! - **Optimisation**: The backend (LLVM) can aggressively optimise concrete code.
//! - **Simplicity**: Code generation becomes trivial – only concrete functions remain.
//!
//! # Pipeline
//! ```text
//! ┌─────────────┐    ┌──────────────────┐     ┌──────────┐
//! │   Semantic  │ -> │ Monomorphisation │ ->  │  CodeGen │
//! │   Analysis  │    │       Pass       │     │  (LLVM)  │
//! └─────────────┘    └──────────────────┘     └──────────┘
//! ```
//! The semantic analysis must annotate every expression with its concrete type
//! (or `Var` for generic parameters). This pass reads those annotations, generates
//! specialised copies, and rewrites the AST accordingly. The output AST contains
//! only non‑generic functions, ready for LLVM IR generation.

use crate::ast::*;
use crate::error::CompilerError;
use std::collections::{HashSet, HashMap};
use crate::semantic::HulkType;

pub struct MonomorphizationPass {
    /// Original function definitions (keyed by name) 
    pub(crate) original_functions: HashMap<String, FunctionDef>,
    /// Already specialised functions, mapping a mangled name (e.g., `foo$Number`)
    /// to the concrete `FunctionDef`.
    pub(crate) specialized: HashMap<String, FunctionDef>,
    // Track in-progress specializations
    pub(crate) instantiating: HashSet<String>, 
}

impl MonomorphizationPass {
    pub fn new() -> Self {
        Self {
            original_functions: HashMap::new(),
            specialized: HashMap::new(),
            instantiating: HashSet::new(),
        }
    }

    /// Runs the monomorphization pass on the given `Program`.
    ///
    /// This method must be called after semantic analysis (type checking) and before
    /// code generation.
    pub fn run(&mut self, program: &mut Program) -> Result<(), CompilerError>{

        // Cache the original definitions (they will be cloned when needed).
        self.original_functions = program.functions
            .iter()
            .map(|f| (f.name.clone(), f.clone()))
            .collect();

        // Start the transformation by visiting the whole program.
        program.accept(self)?;

        Ok(())
    }

    // Helpers 

    /// Generates a unique name for a concrete specialization of a generic function.
    ///
    /// The name is built by concatenating the original function name with the string
    /// representation of each concrete type argument, separated by `$`.
    /// For example: `foo$Number$Bool`.
    pub(crate) fn mangle_specialization(func_name: &str, types: &[HulkType]) -> String {
        let mut parts = vec![func_name.to_string()];
        for ty in types {
            parts.push(Self::type_to_string(ty));
        }
        parts.join("$")
    }

    fn type_to_string(ty: &HulkType) -> String {
        match ty {
            HulkType::Number => "Number".to_string(),
            HulkType::Boolean => "Bool".to_string(),
            HulkType::String => "String".to_string(),
            HulkType::Object => "Object".to_string(),
            HulkType::Class(name) | HulkType::Protocol(name) => name.clone(),
            HulkType::UserDefined(name) => format!("UserDefined{}", name),
            HulkType::Var(id) => format!("Var{}", id),
            HulkType::Error => "Error".to_string(),
        }
    }

    /// Applies a type substitution to a generic function definition.
    ///
    /// This function replaces every occurrence of type variables (represented by `HulkType::Var(id)`)
    /// in the function's parameters, return type, and body expression with the concrete types
    /// provided in the substitution map (`subst`). The substitution is performed in-place,
    /// mutating the `FunctionDef` to become a concrete specialization.
    ///
    /// # Parameters
    /// - `func`: A mutable reference to the function definition to transform.
    /// - `subst`: A mapping from type variable indices (`usize`) to the concrete `HulkType`
    ///   that should replace them. For example, `{0: HulkType::Number, 1: HulkType::Bool}`.
    ///
    /// # Side effects
    /// - Modifies `func.params[*].ty`, `func.ty`, and recursively all types inside `func.body`.
    /// - Does not change the function's name or its generic flag; that is the caller's responsibility.
    ///
    /// # Example
    /// If `func` represents `fn foo<T, U>(x: T, y: U) -> T` and `subst = {0: Number, 1: String}`,
    /// after substitution the function becomes effectively `fn foo(x: Number, y: String) -> Number`.
    
    pub(crate) fn substitute_in_function(func: &mut FunctionDef, subst: &HashMap<usize, HulkType>) {
        for param in &mut func.params {
            if let Some(ty) = &mut param.ty {
                *ty = Self::replace_type_var(ty, subst);
            }
        }
        if let Some(ty) = &mut func.ty {
            *ty = Self::replace_type_var(ty, subst);
        }
        Self::substitute_in_expr(&mut func.body, subst);
    }

    /// Recursively traverses an expression and replaces all type variables (`HulkType::Var(id)`)
    /// with concrete types according to a substitution map.
    ///
    /// This function mutates the given expression in-place. For every node that has a type annotation
    /// (accessible via `ty_mut()`), the type is transformed by `replace_type_var` using the provided
    /// substitution map. Then the function recurses into all child subexpressions to ensure that
    /// type variables nested deeper (e.g., inside a `Call`, `Let`, `If`, etc.) are also replaced.
    ///
    /// # Parameters
    /// - `expr`: A mutable reference to the expression tree to transform.
    /// - `subst`: A mapping from type variable indices (e.g., `0`, `1`, …) to the concrete `HulkType`
    ///   that should replace them.
    ///
    /// # Side Effects
    /// - Modifies the `ty` field of every expression node that originally contained a `Var(id)`
    ///   for which `subst` provides a replacement.
    /// - Does not change the structure of the expression tree, only the types attached to nodes.
    /// - No return value; the transformation is done in-place.

    fn substitute_in_expr(expr: &mut Expr, subst: &HashMap<usize, HulkType>) {

        if let Some(ty) = expr.ty_mut() {
            *ty = Self::replace_type_var(ty, subst);
        }

        match expr {
            Expr::Number(_) | Expr::String(_) | Expr::Bool(_) | Expr::Const(_) | Expr::SelfExpr(_) | Expr::Base(_) => {}
            Expr::Variable(_) => {}
            Expr::BinaryOp(binop) => {
                Self::substitute_in_expr(&mut binop.left, subst);
                Self::substitute_in_expr(&mut binop.right, subst);
            }
            Expr::UnaryOp(unary) => {
                Self::substitute_in_expr(&mut unary.expr, subst);
            }
            Expr::Call(call) => {
                for arg in &mut call.args {
                    Self::substitute_in_expr(arg, subst);
                }
            }
            Expr::Let(let_expr) => {
                for (_, _, init) in &mut let_expr.bindings {
                    Self::substitute_in_expr(init, subst);
                }
                Self::substitute_in_expr(&mut let_expr.body, subst);
            }
            Expr::If(if_expr) => {
                Self::substitute_in_expr(&mut if_expr.condition, subst);
                Self::substitute_in_expr(&mut if_expr.then_branch, subst);
                Self::substitute_in_expr(&mut if_expr.else_branch, subst);
            }
            Expr::While(while_expr) => {
                Self::substitute_in_expr(&mut while_expr.condition, subst);
                Self::substitute_in_expr(&mut while_expr.body, subst);
            }
            Expr::Block(block) => {
                for e in &mut block.expressions {
                    Self::substitute_in_expr(e, subst);
                }
            }
            Expr::DestructiveAssign(assign) => {
                Self::substitute_in_expr(&mut assign.lhs, subst);
                Self::substitute_in_expr(&mut assign.value, subst);
            }
            Expr::AttributeAccess(attr) => {
                Self::substitute_in_expr(&mut attr.object, subst);
            }
            Expr::MethodCall(method_call) => {
                Self::substitute_in_expr(&mut method_call.object, subst);
                for arg in &mut method_call.args {
                    Self::substitute_in_expr(arg, subst);
                }
            }
            Expr::New(new_expr) => {
                for arg in &mut new_expr.args {
                    Self::substitute_in_expr(arg, subst);
                }
            }
            Expr::Is(is_expr) => {
                Self::substitute_in_expr(&mut is_expr.expr, subst);
            }
            Expr::As(as_expr) => {
                Self::substitute_in_expr(&mut as_expr.expr, subst);
            }
        }
    }

    /// This is a core helper for monomorphization: it transforms a generic type
    /// parameter (e.g., `Var(0)`) into the actual type (e.g., `Number`) that should
    /// be used in a concrete specialization of a generic function or type.
    ///
    /// If the given `ty` is a `HulkType::Var(id)` and the substitution map `subst`
    /// contains an entry for `id`, this function returns a clone of the concrete type
    /// associated with that ID. Otherwise, it returns a clone of the original `ty`
    /// unchanged.
    ///
    /// # Parameters
    /// - `ty`: The type to examine and potentially replace.
    /// - `subst`: A mapping from type variable indices (e.g., 0, 1, 2) to their
    ///   corresponding concrete `HulkType`s.
    ///
    /// # Returns
    /// A new `HulkType` where the variable (if any) has been replaced by its concrete
    /// type according to `subst`, or the original type (cloned) if no replacement applies.

    fn replace_type_var(ty: &HulkType, subst: &HashMap<usize, HulkType>) -> HulkType {
        match ty {
            HulkType::Var(id) => subst.get(id).cloned().unwrap_or(ty.clone()),
            _ => ty.clone(),
        }
    }

}

