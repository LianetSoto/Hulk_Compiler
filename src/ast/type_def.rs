use crate::ast::{Expr, Node, Visitor};
use crate::error::Span;
use crate::semantic::types::HulkType;

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub name: String,
    pub params: Vec<String>, 
    pub parent: Option<Parent>,
    pub attributes: Vec<Attribute>,
    pub methods: Vec<Method>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for TypeDef {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_type_def(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub init_expr: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
    pub ty_annotation: Option<HulkType>,
}

impl Node for Attribute {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_attribute(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    pub name: String,
    pub type_name: Option<String>,
    pub params: Vec<MethodParam>,
    pub body: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
    pub ty_annotation: Option<HulkType>,
}

impl Node for Method {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_method(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodParam {
    pub name: String,
    pub span: Span,
    pub ty: Option<HulkType>,
    pub ty_annotation: Option<HulkType>,
}

pub enum TypeMember {
    Attribute(Attribute),
    Method(Method),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parent {
    pub name: String,
    pub args: Vec<Box<Expr>>,
    pub span: Span,
    pub ty: Option<HulkType>,
}