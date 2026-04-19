use crate::ast::{Node, Visitor};
use crate::error::Span;

// ENUM PRINCIPAL DE EXPRESIONES
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(NumberExpr),
    BinaryOp(BinaryOpExpr),
    Print(PrintExpr),
    String(StringExpr),
    Call(CallExpr),
    Const(ConstExpr),
    Bool(BoolExpr),
    UnaryOp(UnaryOpExpr)
}

impl Node for Expr {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        match self {
            Expr::Number(n) => n.accept(visitor),
            Expr::BinaryOp(b) => b.accept(visitor),
            Expr::Print(p) => p.accept(visitor),
            Expr::Call(call_expr) => call_expr.accept(visitor),
            Expr::Const(const_expr) => const_expr.accept(visitor),
            Expr::String(string_expr) => string_expr.accept(visitor),
            Expr::Bool(bool_expr) => bool_expr.accept(visitor),
            Expr::UnaryOp(unary_op_expr) => unary_op_expr.accept(visitor),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberExpr {
    pub value: f64,
    pub span: Span,
}

impl Node for NumberExpr {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_number(self)
    }
}

// OPERADORES BINARIOS
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Pow, Concat, 
    Eq, Neq, Lt, Gt, Leq, Geq,
    And, Or, 
}

// BINARY OP EXPR (operación binaria)
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOpExpr {
    pub left: Box<Expr>,
    pub op: BinOp,
    pub right: Box<Expr>,
    pub span: Span,
}

impl Node for BinaryOpExpr {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_binary_op(self)
    }
}

// PRINT EXPR (llamada a print)
#[derive(Debug, Clone, PartialEq)]
pub struct PrintExpr {
    pub argument: Box<Expr>,
    pub span: Span,
}

impl Node for PrintExpr {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_print(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringExpr {
    pub value: String,
    pub span: Span,
}

impl Node for StringExpr { 
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Result { 
        v.visit_string(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub func: String,
    pub args: Vec<Box<Expr>>,
    pub span: Span,
}

impl Node for CallExpr { 
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Result { 
        v.visit_call(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstExpr {
    pub name: String,
    pub span: Span,
}

impl Node for ConstExpr { 
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Result { 
        v.visit_const(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoolExpr {
    pub value: bool,
    pub span: Span,
}
impl Node for BoolExpr { 
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Result { 
        v.visit_bool(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOpExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
}
impl Node for UnaryOpExpr{ 
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Result { 
        v.visit_unary_op(self) 
    } 
}