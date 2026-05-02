pub mod node;
pub mod program;
pub mod stmt;
pub mod expr;
pub mod visitor;
pub mod printer;

// -----------------------------------------------------------------------------
// RE‑EXPORTACIONES PARA ACCESO CONVENIENTE
// -----------------------------------------------------------------------------
pub use node::Node;
pub use program::Program;
pub use stmt::{ExprStmt, Stmt};
pub use expr::{BinOp, BinaryOpExpr, Expr, NumberExpr, PrintExpr, StringExpr, CallExpr, ConstExpr, BoolExpr, UnaryOpExpr, UnaryOp, VariableExpr, LetExpr, DestructiveAssignExpr, BlockExpr,IfExpr,WhileExpr,ForExpr};
pub use visitor::Visitor;
pub use printer::PrettyPrinter;