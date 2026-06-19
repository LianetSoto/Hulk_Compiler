use crate::ast::{Node, Visitor};
use crate::error::Span;
use crate::semantic::types::HulkType;

// MAIN EXPRESSIONS ENUM
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(NumberExpr),
    BinaryOp(BinaryOpExpr),
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
    New(NewExpr),
    MethodCall(MethodCallExpr),
    SelfExpr(SelfExpr),
    Base(BaseExpr),
    AttributeAccess(AttributeAccessExpr),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Number(n) => n.span,
            Expr::BinaryOp(b) => b.span,
            // Expr::Print(p) => p.span,
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
            Expr::New(new_expr) => new_expr.span,
            Expr::MethodCall(method_call_expr) => method_call_expr.span,
            Expr::SelfExpr(self_expr) => self_expr.span,
            Expr::Base(base_expr) => base_expr.span,
            Expr::AttributeAccess(attr)=> attr.span,
        }
    }

    pub fn get_type(&self) -> Option<&HulkType> {
        match self {
            Expr::Number(n) => n.ty.as_ref(),
            Expr::BinaryOp(b) => b.ty.as_ref(),
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
            Expr::New(new_expr) => new_expr.ty.as_ref(),
            Expr::MethodCall(method_call_expr) => method_call_expr.ty.as_ref(),
            Expr::SelfExpr(self_expr) => self_expr.ty.as_ref(),
            Expr::Base(base_expr) => base_expr.ty.as_ref(),
            Expr::AttributeAccess(attr) => attr.ty.as_ref(),
        }
    }

    pub fn ty_mut(&mut self) -> &mut Option<HulkType> {
        match self {
            Expr::Number(e) => &mut e.ty,
            Expr::BinaryOp(e) => &mut e.ty,
            Expr::String(e) => &mut e.ty,
            Expr::Call(e) => &mut e.ty,
            Expr::Const(e) => &mut e.ty,
            Expr::Bool(e) => &mut e.ty,
            Expr::UnaryOp(e) => &mut e.ty,
            Expr::Variable(e) => &mut e.ty,
            Expr::Let(e) => &mut e.ty,
            Expr::DestructiveAssign(e) => &mut e.ty,
            Expr::Block(e) => &mut e.ty,
            Expr::If(e) => &mut e.ty,
            Expr::While(e) => &mut e.ty,
            Expr::New(e) => &mut e.ty,
            Expr::MethodCall(e) => &mut e.ty,
            Expr::SelfExpr(e) => &mut e.ty,
            Expr::Base(e) => &mut e.ty,
            Expr::AttributeAccess(e) => &mut e.ty,
        }
    }
}

impl Node for Expr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        match self {
            Expr::Number(n) => n.accept(visitor),
            Expr::BinaryOp(b) => b.accept(visitor),
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
            Expr::New(new_expr) => new_expr.accept(visitor),
            Expr::MethodCall(method_call_expr) => method_call_expr.accept(visitor),
            Expr::SelfExpr(self_expr) => self_expr.accept(visitor),
            Expr::Base(base_expr) => base_expr.accept(visitor),
            Expr::AttributeAccess(attr) => attr.accept(visitor),
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

// binary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Pow, Concat, 
    Eq, Neq, Lt, Gt, Leq, Geq,
    And, Or, Mod, ConcatSpace,
}

// BINARY OP EXPR 
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
    pub bindings: Vec<(String, Option<HulkType>,Box<Expr>)>,
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
    pub lhs: Box<Expr>,
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
pub struct NewExpr {
    pub type_name: String,
    pub args: Vec<Box<Expr>>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for NewExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_new(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpr {
    pub object: Box<Expr>,
    pub method: String,
    pub args: Vec<Box<Expr>>,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for MethodCallExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_method_call(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfExpr {
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for SelfExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_self(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BaseExpr {
    pub span: Span,
    pub ty: Option<HulkType>,
    pub base_type: Option<String>, // Parent type that provides the implementation.
    pub method_name: Option<String>, // Name of the invoked method.
}

impl Node for BaseExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_base(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeAccessExpr {
    pub object: Box<Expr>,
    pub attribute: String,
    pub span: Span,
    pub ty: Option<HulkType>,
}

impl Node for AttributeAccessExpr {
    fn accept<V: Visitor>(&mut self, visitor: &mut V) -> V::Result {
        visitor.visit_attribute_access(self)
    }
}
