use crate::ast::{Node, Visitor};

// ENUM PRINCIPAL DE EXPRESIONES
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(NumberExpr),
    BinaryOp(BinaryOpExpr),
    Print(PrintExpr),
    String(StringExpr),
    Call(CallExpr),
    Const(ConstExpr),
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberExpr {
    pub value: f64,
}

impl Node for NumberExpr {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_number(self)
    }
}

// OPERADORES BINARIOS
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Pow, Concat
}

// BINARY OP EXPR (operación binaria)
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOpExpr {
    pub left: Box<Expr>,
    pub op: BinOp,
    pub right: Box<Expr>,
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
}

impl Node for PrintExpr {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_print(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringExpr {
    pub value: String,
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
}

impl Node for CallExpr { 
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Result { 
        v.visit_call(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstExpr {
    pub name: String,
}

impl Node for ConstExpr { 
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Result { 
        v.visit_const(self) 
    } 
}