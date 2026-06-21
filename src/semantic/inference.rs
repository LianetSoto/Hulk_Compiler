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
    let a = self.apply(a);
    let b = self.apply(b);
    match (&a, &b) {
        (HulkType::Var(id1), HulkType::Var(id2)) if id1 == id2 => Ok(()),
        (HulkType::Var(id), _) => self.bind(*id, b),
        (_, HulkType::Var(id)) => self.bind(*id, a),
        (HulkType::Number, HulkType::Number) => Ok(()),
        (HulkType::String, HulkType::String) => Ok(()),
        (HulkType::Boolean, HulkType::Boolean) => Ok(()),
        (HulkType::Class(c1), HulkType::Class(c2)) if c1 == c2 => Ok(()),
        (HulkType::Protocol(p1), HulkType::Protocol(p2)) if p1 == p2 => Ok(()),
        (HulkType::Var(id), HulkType::Protocol(_)) => self.bind(*id, b),
        (HulkType::Protocol(_), HulkType::Var(id)) => self.bind(*id, a),
        (HulkType::Object, HulkType::Object) => Ok(()),
        // Si Object es supertipo, podrías permitir (sub, Object)
        _ => Err(format!("Cannot unify {:?} and {:?}", a, b)),
    }
}
    pub fn bind(&mut self, id: usize, ty: HulkType) -> Result<(), String> {
        // Evita ciclos: si ty es Var(id2) y id2 == id, error
        if let HulkType::Var(id2) = &ty {
            if *id2 == id {
                return Err("Occurs check failed".to_string());
            }
            // Si id2 ya tiene sustitución, podrías seguirla, pero con tipos planos basta.
        }
        self.subs.insert(id, ty);
        Ok(())
    }

    pub fn resolve(&self, ty: &HulkType) -> HulkType {
        let applied = self.apply(ty);
        match applied {
            HulkType::Var(id) if !self.subs.contains_key(&id) => applied,
            other => other,
        }
}
}