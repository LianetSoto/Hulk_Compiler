use std::collections::HashMap;
use crate::semantic::types::HulkType;

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    StringOrNumber,
    ConformsToProtocol(String), 
}

#[derive(Default)]
pub struct Unifier {
    subs: HashMap<usize, HulkType>,
    next_var: usize,
    constraints: HashMap<usize, Vec<Constraint>>,
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

    // Agrega una restricción a una variable
    pub fn add_constraint(&mut self, id: usize, constraint: Constraint) {
        self.constraints.entry(id).or_default().push(constraint);
    }

    // Verifica que el tipo cumpla todas las restricciones de la variable
    

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
            (HulkType::Object, HulkType::Object) => Ok(()),
            _ => Err(format!("Cannot unify {:?} and {:?}", a, b)),
        }
    }

    pub fn bind(&mut self, id: usize, ty: HulkType) -> Result<(), String> {
        // Evita ciclos
        if let HulkType::Var(id2) = &ty {
            if *id2 == id {
                return Err("Occurs check failed".to_string());
            }
        }
        // Verifica restricciones antes de ligar
        self.check_constraints(id, &ty)?;
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

    // Obtiene una referencia a las restricciones de una variable (si las tiene)
    pub fn get_constraints(&self, id: usize) -> Option<&Vec<Constraint>> {
        self.constraints.get(&id)
    }

    // Dentro de impl Unifier
pub fn check_constraints(&self, id: usize, ty: &HulkType) -> Result<(), String> {
    if let Some(constraints) = self.constraints.get(&id) {
        for c in constraints {
            match c {
                Constraint::StringOrNumber => {
                    if !matches!(ty, HulkType::String | HulkType::Number) {
                        return Err(format!("Type {:?} must be String or Number", ty));
                    }
                }
                Constraint::ConformsToProtocol(_) => {
                    // La verificación se hace en el TypeChecker en la llamada
                }
            }
        }
    }
    Ok(())
}
}