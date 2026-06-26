pub mod node;
pub mod program;
pub mod expr;
pub mod function;
pub mod visitor;
pub mod printer;
pub mod type_def;

pub use node::Node;
pub use program::Program;
pub use expr::{BinOp, BinaryOpExpr, Expr, NumberExpr, StringExpr, CallExpr, ConstExpr, BoolExpr, UnaryOpExpr, UnaryOp, VariableExpr, LetExpr, DestructiveAssignExpr, BlockExpr,IfExpr,WhileExpr,NewExpr, MethodCallExpr, SelfExpr, BaseExpr, AttributeAccessExpr, IsExpr, AsExpr, ForExpr};
pub use function::{FunctionDef, Params};
pub use visitor::Visitor;
pub use printer::PrettyPrinter;
pub use type_def::{TypeDef, Attribute, Method, TypeMember, MethodParam, Parent, ProtocolDef, ProtocolMethod};

// Add this enum
#[derive(Debug, Clone, PartialEq)]
pub enum TopLevel {
    Type(TypeDef),
    Protocol(ProtocolDef),
    Function(FunctionDef),
}