use crate::ast::{Stmt, Node, Visitor};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

impl Node for Program {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_program(self)
    }
}