use crate::ast::{BinaryOpExpr, ExprStmt, Node, NumberExpr, PrintExpr, Program, Visitor, StringExpr, CallExpr, ConstExpr};

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

    fn visit_program(&mut self, p: &Program) {
        self.write_line("Program {");
        self.indent += 1;
        for stmt in &p.statements {
            stmt.accept(self);
        }
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_expr_stmt(&mut self, s: &ExprStmt) {
        self.write_line("ExprStmt {");
        self.indent += 1;
        s.expr.accept(self);
        self.indent -= 1;
        self.write_line("}");
    }

    fn visit_number(&mut self, n: &NumberExpr) {
        self.write_line(&format!("Number({})", n.value));
    }

    fn visit_binary_op(&mut self, b: &BinaryOpExpr) {
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

    fn visit_print(&mut self, p: &PrintExpr) {
        self.write_line("Print {");
        self.indent += 1;
        p.argument.accept(self);
        self.indent -= 1;
        self.write_line("}");
    }
    
    fn visit_string(&mut self, expr: &StringExpr) -> Self::Result {
        self.write_line(&format!("String({:?})", expr.value));
    }

    fn visit_call(&mut self, expr: &CallExpr) -> Self::Result {
        self.write_line(&format!("Call({}, args: [", expr.func));
        self.indent += 1;
        for (i, arg) in expr.args.iter().enumerate() {
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

    fn visit_const(&mut self, expr: &ConstExpr) -> Self::Result {
        self.write_line(&format!("Const({})", expr.name));
    }
}