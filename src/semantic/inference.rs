// inference.rs (o dentro de type_checker.rs)
use std::collections::HashMap;
use crate::semantic::types::HulkType;

#[derive(Default)]
pub struct Unifier {
    subs: HashMap<usize, HulkType>,
    next_var: usize,
}

impl Unifier {
    pub fn new_var(&mut self) -> HulkType {
        let id = self.next_var;
        self.next_var += 1;
        HulkType::Var(id)
    }

    // Aplica la sustitución actual a un tipo
    pub fn apply(&self, ty: &HulkType) -> HulkType {
        match ty {
            HulkType::Var(id) => {
                if let Some(sub) = self.subs.get(id) {
                    self.apply(sub)   // recursión por si la sustitución apunta a otra var
                } else {
                    ty.clone()
                }
            }
            _ => ty.clone(),
        }
    }

    // Unifica dos tipos, registrando sustituciones
    pub fn unify(&mut self, a: &HulkType, b: &HulkType) -> Result<(), String> {
        let a_resolved = self.apply(a);
        let b_resolved = self.apply(b);
        match (a_resolved, b_resolved) {
            (HulkType::Var(id), other) | (other, HulkType::Var(id)) => {
                self.subs.insert(id, other);
                Ok(())
            }
            (HulkType::Number, HulkType::Number) => Ok(()),
            (HulkType::String, HulkType::String) => Ok(()),
            (HulkType::Boolean, HulkType::Boolean) => Ok(()),
            (HulkType::Object, _) | (_, HulkType::Object) => Ok(()),
            (a_ty, b_ty) => Err(format!("Cannot unify {:?} and {:?}", a_ty, b_ty)),
        }
    }

    pub fn resolve(&self, ty: &HulkType) -> HulkType {
        let applied = self.apply(ty);
        match applied {
            HulkType::Var(id) if !self.subs.contains_key(&id) => HulkType::Object,
            other => other,
        }
    }
}