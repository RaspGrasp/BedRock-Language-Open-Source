use crate::ast::{Statement, Expression, TypeKind};
use std::collections::{HashMap, HashSet};

pub struct Pruner {
    constants: HashMap<String, u64>,
    locals: HashSet<String>,
    in_fn: bool,
}

impl Pruner {
    pub fn new() -> Self {
        Pruner {
            constants: HashMap::new(),
            locals: HashSet::new(),
            in_fn: false,
        }
    }


pub fn run(&mut self, program: Vec<Statement>) -> Vec<Statement> {
    let folded: Vec<Statement> = self.fold_block(program);
    let live = self.compute_live_symbols(&folded);
    self.eliminate_dead(folded, &live)
}

    fn compute_live_symbols(&self, stmts: &[Statement]) -> HashSet<String> {
        let mut live = HashSet::new();
        for stmt in stmts {
            self.collect_used_in_stmt(stmt, &mut live);
        }
        live
    }

    fn collect_used_in_stmt(&self, stmt: &Statement, used: &mut HashSet<String>) {
        match stmt {
            Statement::Let(_, expr, _) => self.collect_used_in_expr(expr, used),
            Statement::Root(_, expr, _) => self.collect_used_in_expr(expr, used),
            Statement::Assignment(name, expr) => {
                used.insert(name.clone());
                self.collect_used_in_expr(expr, used);
            }
            Statement::ArrayAssign(name, idx, val) => {
                used.insert(name.clone());
                self.collect_used_in_expr(idx, used);
                self.collect_used_in_expr(val, used);
            }
            Statement::Return(Some(expr)) => self.collect_used_in_expr(expr, used),
            Statement::If(cond, then_b, else_b) => {
                self.collect_used_in_expr(cond, used);
                for s in then_b { self.collect_used_in_stmt(s, used); }
                if let Some(eb) = else_b {
                    for s in eb { self.collect_used_in_stmt(s, used); }
                }
            }
            Statement::While(cond, body) => {
                self.collect_used_in_expr(cond, used);
                for s in body { self.collect_used_in_stmt(s, used); }
            }
            Statement::Loop(body) => {
                for s in body { self.collect_used_in_stmt(s, used); }
            }
            Statement::FunctionDefine(name, _, body, _) => {
                used.insert(name.clone());
                for s in body { self.collect_used_in_stmt(s, used); }
            }
            Statement::Call(name, args) => {
                used.insert(name.clone());
                for a in args { self.collect_used_in_expr(a, used); }
            }
            Statement::Outb(port, val) => {
                self.collect_used_in_expr(port, used);
                self.collect_used_in_expr(val, used);
            }
            Statement::Poke(addr, val) => {
                self.collect_used_in_expr(addr, used);
                self.collect_used_in_expr(val, used);
            }
            Statement::CallPtr(expr) => self.collect_used_in_expr(expr, used),
            Statement::IntHandler(_, body) => {
    for s in body { self.collect_used_in_stmt(s, used); }
}
Statement::IntEnable(expr, _) => {
    self.collect_used_in_expr(expr, used);
}
Statement::IntDisable => {}
_ => {}
        }
    }

    fn collect_used_in_expr(&self, expr: &Expression, used: &mut HashSet<String>) {
        match expr {
            Expression::Variable(name) => { used.insert(name.clone()); }
            Expression::BinaryOp(l, _, r) => {
                self.collect_used_in_expr(l, used);
                self.collect_used_in_expr(r, used);
            }
            Expression::Call(name, args) => {
                used.insert(name.clone());
                for a in args { self.collect_used_in_expr(a, used); }
            }
            Expression::ArrayAccess(name, idx) => {
                used.insert(name.clone());
                self.collect_used_in_expr(idx, used);
            }
            Expression::Peek(addr) => self.collect_used_in_expr(addr, used),
            Expression::Inb(port) => self.collect_used_in_expr(port, used),
            Expression::FieldAccess(var, _) => { used.insert(var.clone()); }
            Expression::FieldAssign(var, _, val) => {
                used.insert(var.clone());
                self.collect_used_in_expr(val, used);
            }
            _ => {}
        }
    }

    fn eliminate_dead(&self, stmts: Vec<Statement>, live: &HashSet<String>) -> Vec<Statement> {
        stmts.into_iter().filter(|s| match s {
            Statement::Root(_, _, _) => true,
            Statement::Let(name, _, _) => live.contains(name),
            Statement::ArrayDefine(name, _, _) => live.contains(name),
            Statement::StringDefine(name, _) => live.contains(name),
            Statement::StructInstance(name, _) => live.contains(name),
            _ => true,
        }).collect()
    }

    // ---------------------------------------------------------------------
    // Constant folding / propagation.
    //
    // Soundness rules (the previous version violated all of them):
    //  * only function-local variables (parameters + `let`s inside the function) are tracked, never
    //    globals/roots (any call may change those);
    //  * a variable that stops being constant (non-constant assignment/let) is forgotten;
    //  * at an `if` the two branches start from the same state and the state afterwards is the
    //    intersection of both;
    //  * before a loop every variable assigned in the loop body is forgotten, because the body
    //    runs after the condition was evaluated for the first time;
    //  * a statically known `if`/`while` is replaced by the taken branch / nothing, spliced into the
    //    enclosing block (never by `loop {}` or `return`).
    // ---------------------------------------------------------------------
    fn fold_block(&mut self, stmts: Vec<Statement>) -> Vec<Statement> {
        let mut out = Vec::new();
        for s in stmts {
            self.fold_stmt_into(s, &mut out);
        }
        out
    }

    fn collect_assigned(stmts: &[Statement], set: &mut HashSet<String>) {
        for st in stmts {
            match st {
                Statement::Let(n, e, _) | Statement::Root(n, e, _) | Statement::Assignment(n, e) => {
                    set.insert(n.clone());
                    Self::collect_assigned_expr(e, set);
                }
                Statement::ArrayAssign(n, i, v) => {
                    set.insert(n.clone());
                    Self::collect_assigned_expr(i, set);
                    Self::collect_assigned_expr(v, set);
                }
                Statement::If(c, t, e) => {
                    Self::collect_assigned_expr(c, set);
                    Self::collect_assigned(t, set);
                    if let Some(e) = e { Self::collect_assigned(e, set); }
                }
                Statement::While(c, b) => {
                    Self::collect_assigned_expr(c, set);
                    Self::collect_assigned(b, set);
                }
                Statement::Loop(b) | Statement::IntHandler(_, b) => Self::collect_assigned(b, set),
                Statement::Return(Some(e)) | Statement::CallPtr(e) => Self::collect_assigned_expr(e, set),
                Statement::Call(_, args) => for a in args { Self::collect_assigned_expr(a, set); },
                Statement::Poke(a, v) | Statement::Outb(a, v) => {
                    Self::collect_assigned_expr(a, set);
                    Self::collect_assigned_expr(v, set);
                }
                Statement::SaveContext(n) | Statement::RestoreContext(n) | Statement::Bnw(n) => { set.insert(n.clone()); }
                _ => {}
            }
        }
    }

    fn collect_assigned_expr(e: &Expression, set: &mut HashSet<String>) {
        match e {
            Expression::FieldAssign(v, _, x) => { set.insert(v.clone()); Self::collect_assigned_expr(x, set); }
            Expression::BinaryOp(l, _, r) => { Self::collect_assigned_expr(l, set); Self::collect_assigned_expr(r, set); }
            Expression::Peek(x) | Expression::Inb(x) => Self::collect_assigned_expr(x, set),
            Expression::ArrayAccess(_, x) => Self::collect_assigned_expr(x, set),
            Expression::Call(_, args) | Expression::Syscall(args) => for a in args { Self::collect_assigned_expr(a, set); },
            _ => {}
        }
    }

    fn forget(&mut self, set: &HashSet<String>) {
        for n in set { self.constants.remove(n); }
    }

    fn set_const(&mut self, name: &str, folded: &Expression) {
        match folded {
            Expression::Number(v, _) if self.locals.contains(name) => { self.constants.insert(name.to_string(), *v); }
            _ => { self.constants.remove(name); }
        }
    }

    fn fold_stmt_into(&mut self, stmt: Statement, out: &mut Vec<Statement>) {
        match stmt {
            Statement::Let(name, expr, kind) => {
                let folded = self.fold_expr(expr);
                if self.in_fn { self.locals.insert(name.clone()); }
                self.set_const(&name, &folded);
                out.push(Statement::Let(name, folded, kind));
            }
            Statement::Root(name, expr, kind) => {
                let folded = self.fold_expr(expr);
                self.constants.remove(&name);
                out.push(Statement::Root(name, folded, kind));
            }
            Statement::Assignment(name, expr) => {
                let folded = self.fold_expr(expr);
                self.set_const(&name, &folded);
                out.push(Statement::Assignment(name, folded));
            }
            Statement::ArrayAssign(name, idx, val) => {
                self.constants.remove(&name);
                out.push(Statement::ArrayAssign(name, self.fold_expr(idx), self.fold_expr(val)));
            }
            Statement::Return(Some(expr)) => out.push(Statement::Return(Some(self.fold_expr(expr)))),
            Statement::If(cond, then_b, else_b) => {
                let cond_folded = self.fold_expr(cond);
                if let Expression::Number(v, _) = &cond_folded {
                    // statically known: splice the taken branch into the enclosing block
                    let taken = if *v != 0 { Some(then_b) } else { else_b };
                    if let Some(body) = taken {
                        for s in body { self.fold_stmt_into(s, out); }
                    }
                    return;
                }
                let before = self.constants.clone();
                let then_folded = self.fold_block(then_b);
                let after_then = std::mem::replace(&mut self.constants, before.clone());
                let else_folded = else_b.map(|eb| self.fold_block(eb));
                let after_else = std::mem::take(&mut self.constants);
                self.constants = after_then.into_iter()
                    .filter(|(k, v)| after_else.get(k) == Some(v))
                    .collect();
                out.push(Statement::If(cond_folded, then_folded, else_folded));
            }
            Statement::While(cond, body) => {
                let mut assigned = HashSet::new();
                Self::collect_assigned(&body, &mut assigned);
                Self::collect_assigned_expr(&cond, &mut assigned);
                self.forget(&assigned);
                let cond_folded = self.fold_expr(cond);
                if let Expression::Number(0, _) = &cond_folded {
                    return; // never executes
                }
                let body_folded = self.fold_block(body);
                self.forget(&assigned);
                out.push(Statement::While(cond_folded, body_folded));
            }
            Statement::Loop(body) => {
                let mut assigned = HashSet::new();
                Self::collect_assigned(&body, &mut assigned);
                self.forget(&assigned);
                let body_folded = self.fold_block(body);
                self.forget(&assigned);
                out.push(Statement::Loop(body_folded));
            }
            Statement::FunctionDefine(name, params, body, ret) => {
                let saved_c = std::mem::take(&mut self.constants);
                let saved_l = std::mem::take(&mut self.locals);
                let saved_f = self.in_fn;
                self.in_fn = true;
                for (p, _) in &params { self.locals.insert(p.clone()); }
                let body_folded = self.fold_block(body);
                self.constants = saved_c;
                self.locals = saved_l;
                self.in_fn = saved_f;
                out.push(Statement::FunctionDefine(name, params, body_folded, ret));
            }
            Statement::IntHandler(name, body) => {
                let saved_c = std::mem::take(&mut self.constants);
                let saved_l = std::mem::take(&mut self.locals);
                let saved_f = self.in_fn;
                self.in_fn = true;
                let b = self.fold_block(body);
                self.constants = saved_c;
                self.locals = saved_l;
                self.in_fn = saved_f;
                out.push(Statement::IntHandler(name, b));
            }
            Statement::Call(name, args) => {
                out.push(Statement::Call(name, args.into_iter().map(|a| self.fold_expr(a)).collect()));
            }
            Statement::Outb(port, val) => out.push(Statement::Outb(self.fold_expr(port), self.fold_expr(val))),
            Statement::Poke(addr, val) => out.push(Statement::Poke(self.fold_expr(addr), self.fold_expr(val))),
            Statement::CallPtr(expr) => out.push(Statement::CallPtr(self.fold_expr(expr))),
            Statement::SaveContext(n) | Statement::RestoreContext(n) | Statement::Bnw(n) if false => { let _ = n; }
            other => {
                // asm, break, context switches, anything unknown: be conservative
                if matches!(other, Statement::Asm(_) | Statement::SaveContext(_) | Statement::RestoreContext(_)) {
                    self.constants.clear();
                }
                out.push(other);
            }
        }
    }

    fn fold_expr(&self, expr: Expression) -> Expression {
        match expr {
            Expression::Variable(ref name) => {
                if let Some(&v) = self.constants.get(name) {
                    return Expression::Number(v, TypeKind::Unknown);
                }
                expr
            }
            Expression::BinaryOp(left, op, right) => {
                let l = self.fold_expr(*left);
                let r = self.fold_expr(*right);
                if let (Expression::Number(lv, lk), Expression::Number(rv, _)) = (&l, &r) {
                    let lv = *lv;
                    let rv = *rv;
                    let kind = lk.clone();
                    // BedRock values are 32-bit unless declared u64/i64; folding must wrap like the target does
                    let wide = matches!(kind, TypeKind::U64 | TypeKind::I64);
                    let signed = matches!(kind, TypeKind::I8 | TypeKind::I16 | TypeKind::I32 | TypeKind::I64);
                    if signed || wide {
                        return Expression::BinaryOp(Box::new(l), op, Box::new(r));
                    }
                    let m = |x: u64| x & 0xFFFF_FFFF;
                    if matches!(op.as_str(), "<<" | ">>") && rv >= 32 {
                        return Expression::BinaryOp(Box::new(l), op, Box::new(r));
                    }
                    match op.as_str() {
                        "+" => return Expression::Number(m(lv.wrapping_add(rv)), kind),
                        "-" => return Expression::Number(m(lv.wrapping_sub(rv)), kind),
                        "*" => return Expression::Number(m(lv.wrapping_mul(rv)), kind),
                        "/" if rv != 0 => return Expression::Number(m(lv) / m(rv), kind),
                        "%" if rv != 0 => return Expression::Number(m(lv) % m(rv), kind),
                        "<<" => return Expression::Number(m(lv << rv), kind),
                        ">>" => return Expression::Number(m(lv) >> rv, kind),
                        "&" => return Expression::Number(lv & rv, kind),
                        "|" => return Expression::Number(lv | rv, kind),
                        "^" => return Expression::Number(lv ^ rv, kind),
                        "==" => return Expression::Number(if lv == rv { 1 } else { 0 }, TypeKind::Bool),
                        "!=" => return Expression::Number(if lv != rv { 1 } else { 0 }, TypeKind::Bool),
                        ">" => return Expression::Number(if lv > rv { 1 } else { 0 }, TypeKind::Bool),
                        "<" => return Expression::Number(if lv < rv { 1 } else { 0 }, TypeKind::Bool),
                        ">=" => return Expression::Number(if lv >= rv { 1 } else { 0 }, TypeKind::Bool),
                        "<=" => return Expression::Number(if lv <= rv { 1 } else { 0 }, TypeKind::Bool),
                        _ => {}
                    }
                }
                Expression::BinaryOp(Box::new(l), op, Box::new(r))
            }
            Expression::Peek(addr) => Expression::Peek(Box::new(self.fold_expr(*addr))),
            Expression::Inb(port) => Expression::Inb(Box::new(self.fold_expr(*port))),
            Expression::ArrayAccess(name, idx) => {
                Expression::ArrayAccess(name, Box::new(self.fold_expr(*idx)))
            }
            Expression::FieldAssign(var, field, val) => {
                Expression::FieldAssign(var, field, Box::new(self.fold_expr(*val)))
            }
            other => other,
        }
    }
}
