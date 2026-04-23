use crate::ast::{AssignExpr, BinaryOpExpr, BlockExpr, BoolExpr, CallExpr, ConstExpr, ExprStmt, ForExpr, IfExpr, LetExpr, Node, NumberExpr, PrintExpr, Program, StringExpr, UnaryOp, UnaryOpExpr, VariableExpr, Visitor, WhileExpr};

pub struct PrettyPrinter {
    indent: usize,
    output: String,
}

impl PrettyPrinter {
    pub fn new() -> Self {
        Self { indent: 0, output: String::new() }
    }

    fn write_line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
        self.output.push_str(line);
        self.output.push('\n');
    }

    pub fn into_string(self) -> String {
        self.output
    }
}

impl Visitor for PrettyPrinter {
    type Result = ();

    fn visit_program(&mut self, p: &mut Program) {
        self.write_line("Program {");
        self.indent += 1;
        for stmt in &mut p.statements {
            stmt.accept(self);
        }
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_expr_stmt(&mut self, s: &mut ExprStmt) {
        self.write_line("ExprStmt {");
        self.indent += 1;
        s.expr.accept(self);
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_number(&mut self, n: &mut NumberExpr) {
        self.write_line(&format!("Number({})", n.value));
    }

    fn visit_binary_op(&mut self, b: &mut BinaryOpExpr) {
        self.write_line(&format!("BinaryOp {{ op: {:?}", b.op));
        self.indent += 1;
        self.write_line("left:");
        self.indent += 1;
        b.left.accept(self);
        self.indent -= 1;
        self.write_line("right:");
        self.indent += 1;
        b.right.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_print(&mut self, p: &mut PrintExpr) {
        self.write_line("Print {");
        self.indent += 1;
        p.argument.accept(self);
        self.indent -= 1;
        self.write_line("}");
    }
    
    fn visit_string(&mut self, expr: &mut StringExpr) -> Self::Result {
        self.write_line(&format!("String({:?})", expr.value));
    }

    fn visit_call(&mut self, expr: &mut CallExpr) -> Self::Result {
        self.write_line(&format!("Call({}, args: [", expr.func));
        self.indent += 1;
        for (i, arg) in expr.args.iter_mut().enumerate() {
            if i > 0 {
                self.write_line(",");
            }
            arg.accept(self);
        }
        if !expr.args.is_empty() {
            self.write_line("");
        }
        self.indent -= 1;
        self.write_line("])");
    }

    fn visit_const(&mut self, expr: &mut ConstExpr) -> Self::Result {
        self.write_line(&format!("Const({})", expr.name));
    }

    fn visit_bool(&mut self, expr: &mut BoolExpr) -> Self::Result {
        self.write_line(&format!("Bool({})", expr.value));
    }

    fn visit_unary_op(&mut self, expr: &mut UnaryOpExpr) -> Self::Result {
        let op_name = match expr.op {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "-",
        };
        self.write_line(&format!("UnaryOp({})", op_name));
        self.indent += 1;
        expr.expr.accept(self);
        self.indent -= 1;
    }

    fn visit_variable(&mut self, expr: &mut VariableExpr) -> Self::Result {
        self.write_line(&format!("Variable({})", expr.name));
    }

    fn visit_let(&mut self, expr: &mut LetExpr) -> Self::Result {
        self.write_line("Let {");
        self.indent += 1;
        for (name, init) in &mut expr.bindings {
            self.write_line(&format!("{} =", name));
            self.indent += 1;
            init.accept(self);
            self.indent -= 1;
        }
        self.write_line("body:");
        self.indent += 1;
        expr.body.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_assign(&mut self, expr: &mut AssignExpr) -> Self::Result {
        self.write_line(&format!("Assign({} :=)", expr.name));
        self.indent += 1;
        expr.value.accept(self);
        self.indent -= 1;
    }

    fn visit_block(&mut self, expr: &mut BlockExpr) -> Self::Result {
        self.write_line("Block {");
        self.indent += 1;
        for e in &mut expr.expressions {
            e.accept(self);
        }
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_if(&mut self, expr: &mut IfExpr) -> Self::Result {
        self.write_line("If {");
        self.indent += 1;
        self.write_line("condition:");
        self.indent += 1;
        expr.condition.accept(self);
        self.indent -= 1;
        self.write_line("then:");
        self.indent += 1;
        expr.then_branch.accept(self);
        self.indent -= 1;
        for (i, (cond, body)) in expr.elif_branches.iter_mut().enumerate() {
            self.write_line(&format!("elif_{}:", i));
            self.indent += 1;
            self.write_line("condition:");
            self.indent += 1;
            cond.accept(self);
            self.indent -= 1;
            self.write_line("body:");
            self.indent += 1;
            body.accept(self);
            self.indent -= 1;
            self.indent -= 1;
        }
        if let Some(else_expr) = &mut expr.else_branch {
            self.write_line("else:");
            self.indent += 1;
            else_expr.accept(self);
            self.indent -= 1;
        }
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_while(&mut self, expr: &mut WhileExpr) -> Self::Result {
        self.write_line("While {");
        self.indent += 1;
        self.write_line("condition:");
        self.indent += 1;
        expr.condition.accept(self);
        self.indent -= 1;
        self.write_line("body:");
        self.indent += 1;
        expr.body.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_for(&mut self, expr: &mut ForExpr) -> Self::Result {
        self.write_line(&format!("For({} in)", expr.var));
        self.indent += 1;
        self.write_line("iterable:");
        self.indent += 1;
        expr.iterable.accept(self);
        self.indent -= 1;
        self.write_line("body:");
        self.indent += 1;
        expr.body.accept(self);
        self.indent -= 1;
        self.indent -= 1;
        self.write_line(")");
    }
}
