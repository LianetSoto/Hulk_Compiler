use crate::ast::{Node, Visitor};
use crate::error::Span;
use crate::semantic::types::HulkType;

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
    UnaryOp(UnaryOpExpr),
    Variable(VariableExpr),
    Let(LetExpr),
    DestructiveAssign(DestructiveAssignExpr),
    Block(BlockExpr),
    If(IfExpr),
    While(WhileExpr),
    For(ForExpr),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Number(n) => n.span,
            Expr::BinaryOp(b) => b.span,
            Expr::Print(p) => p.span,
            Expr::String(s) => s.span,
            Expr::Bool(b) => b.span,
            Expr::Const(c) => c.span,
            Expr::Call(c) => c.span,
            Expr::UnaryOp(u) => u.span,
            Expr::Variable(v) => v.span,
            Expr::Let(l) => l.span,
            Expr::DestructiveAssign(a) => a.span,
            Expr::Block(b) => b.span,
            Expr::If(i) => i.span,
            Expr::While(w) => w.span,
            Expr::For(f) => f.span,
        }
    }

    pub fn get_type(&self) -> Option<&HulkType> {
        match self {
            Expr::Number(n) => n.ty.as_ref(),
            Expr::BinaryOp(b) => b.ty.as_ref(),
            Expr::Print(p) => p.ty.as_ref(),
            Expr::String(s) => s.ty.as_ref(),
            Expr::Call(c) => c.ty.as_ref(),
            Expr::Const(c) => c.ty.as_ref(),
            Expr::Bool(b) => b.ty.as_ref(),
            Expr::UnaryOp(u) => u.ty.as_ref(),
            Expr::Variable(v) => v.ty.as_ref(),
            Expr::Let(l) => l.ty.as_ref(),
            Expr::DestructiveAssign(a) => a.ty.as_ref(),
            Expr::Block(b) => b.ty.as_ref(),
            Expr::If(i) => i.ty.as_ref(),
            Expr::While(w) => w.ty.as_ref(),
            Expr::For(f) => f.ty.as_ref(),
        }
    }
}

impl Node for Expr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        match self {
            Expr::Number(n) => n.accept(visitor),
            Expr::BinaryOp(b) => b.accept(visitor),
            Expr::Print(p) => p.accept(visitor),
            Expr::Call(call_expr) => call_expr.accept(visitor),
            Expr::Const(const_expr) => const_expr.accept(visitor),
            Expr::String(string_expr) => string_expr.accept(visitor),
            Expr::Bool(bool_expr) => bool_expr.accept(visitor),
            Expr::UnaryOp(unary_op_expr) => unary_op_expr.accept(visitor),
            Expr::Variable(v) => v.accept(visitor),
            Expr::Let(l) => l.accept(visitor),
            Expr::DestructiveAssign(a) => a.accept(visitor),
            Expr::Block(b) => b.accept(visitor),
            Expr::If(i) => i.accept(visitor),
            Expr::While(w) => w.accept(visitor),
            Expr::For(f) => f.accept(visitor),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberExpr {
    pub value: f64,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for NumberExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_number(self)
    }
}

// OPERADORES BINARIOS
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Pow, Concat, 
    Eq, Neq, Lt, Gt, Leq, Geq,
    And, Or, Mod, ConcatSpace,
}

// BINARY OP EXPR (operación binaria)
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOpExpr {
    pub left: Box<Expr>,
    pub op: BinOp,
    pub right: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for BinaryOpExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_binary_op(self)
    }
}

// PRINT EXPR (llamada a print)
#[derive(Debug, Clone, PartialEq)]
pub struct PrintExpr {
    pub argument: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for PrintExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_print(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringExpr {
    pub value: String,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for StringExpr { 
    fn accept<V: Visitor>(&mut self, v: &mut V) -> V::Result { 
        v.visit_string(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub func: String,
    pub args: Vec<Box<Expr>>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for CallExpr { 
    fn accept<V: Visitor>(&mut self, v: &mut V) -> V::Result { 
        v.visit_call(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstExpr {
    pub name: String,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for ConstExpr { 
    fn accept<V: Visitor>(&mut self, v: &mut V) -> V::Result { 
        v.visit_const(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoolExpr {
    pub value: bool,
    pub span: Span,
    pub ty: Option<HulkType>,
}
impl Node for BoolExpr { 
    fn accept<V: Visitor>(&mut self, v: &mut V) -> V::Result { 
        v.visit_bool(self) 
    } 
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOpExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg
}
impl Node for UnaryOpExpr{ 
    fn accept<V: Visitor>(&mut self, v: &mut V) -> V::Result { 
        v.visit_unary_op(self) 
    } 
}
#[derive(Debug, Clone, PartialEq)]
pub struct VariableExpr {
    pub name: String,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for VariableExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_variable(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetExpr {
    pub bindings: Vec<(String, Box<Expr>)>,
    pub body: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for LetExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_let(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestructiveAssignExpr {
    pub name: String,
    pub value: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for DestructiveAssignExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_assign(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub expressions: Vec<Box<Expr>>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for BlockExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_block(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub then_branch: Box<Expr>,
    pub elif_branches: Vec<(Box<Expr>, Box<Expr>)>, // (condition, body)
    pub else_branch: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for IfExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_if(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileExpr {
    pub condition: Box<Expr>,
    pub body: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for WhileExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_while(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForExpr {
    pub var: String,
    pub iterable: Box<Expr>,
    pub body: Box<Expr>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for ForExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_for(self)
    }
}
