use crate::ast::{Expr, Node, Visitor};
use crate::error::Span;
use crate::semantic::types::HulkType;

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub name: String,
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
    pub ty_annotation: Option<HulkType>,
    pub init_expr: Box<Expr>,
    pub span: Span,
}

impl Node for Attribute {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_attribute(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    pub name: String,
    pub params: Vec<MethodParam>,
    pub return_ty: Option<HulkType>,
    pub body: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>, // tipo de retorno inferido
}

impl Node for Method {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_method(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodParam {
    pub name: String,
    pub ty_annotation: Option<HulkType>,
    pub span: Span,
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
}