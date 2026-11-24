//! Visiting every expression of a body mutably, closures' bodies included.

use elomar_ast::*;

/// Something that visits expressions with the function it is given.
pub(crate) type Walker<'a> = dyn FnMut(&mut dyn FnMut(&mut Expr)) + 'a;

pub(crate) fn exprs_in_block(b: &mut Block, f: &mut dyn FnMut(&mut Expr)) {
    for s in &mut b.stmts {
        exprs_in_stmt(s, f);
    }
}

fn exprs_in_stmt(s: &mut Stmt, f: &mut dyn FnMut(&mut Expr)) {
    match &mut s.kind {
        StmtKind::Let { value, .. } | StmtKind::LetTuple { value, .. } => exprs(value, f),
        StmtKind::Assign { target, value, .. } => {
            exprs(target, f);
            exprs(value, f);
        }
        StmtKind::If {
            cond,
            then,
            otherwise,
        } => {
            exprs(cond, f);
            exprs_in_block(then, f);
            match otherwise {
                Some(Else::Block(b)) => exprs_in_block(b, f),
                Some(Else::If(s)) => exprs_in_stmt(s, f),
                None => {}
            }
        }
        StmtKind::While { cond, body } => {
            exprs(cond, f);
            exprs_in_block(body, f);
        }
        StmtKind::For { iter, body, .. } => {
            exprs(iter, f);
            exprs_in_block(body, f);
        }
        StmtKind::Return(value) => {
            if let Some(e) = value {
                exprs(e, f);
            }
        }
        StmtKind::Block(b) => exprs_in_block(b, f),
        StmtKind::Expr(e) => exprs(e, f),
        StmtKind::Break | StmtKind::Continue => {}
    }
}

pub(crate) fn exprs(e: &mut Expr, f: &mut dyn FnMut(&mut Expr)) {
    f(e);
    match &mut e.kind {
        ExprKind::Interp(parts) => {
            for p in parts {
                if let InterpPart::Expr(x) = p {
                    exprs(x, f);
                }
            }
        }
        ExprKind::Tuple(items) | ExprKind::List(items) | ExprKind::Variant { args: items, .. } => {
            for x in items {
                exprs(x, f);
            }
        }
        ExprKind::Record(fields) => {
            for x in fields {
                exprs(&mut x.value, f);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Await(expr)
        | ExprKind::Try(expr)
        | ExprKind::Field { target: expr, .. } => exprs(expr, f),
        ExprKind::Binary { lhs, rhs, .. } => {
            exprs(lhs, f);
            exprs(rhs, f);
        }
        ExprKind::Range { start, end, .. } => {
            exprs(start, f);
            exprs(end, f);
        }
        ExprKind::Index { target, index } => {
            exprs(target, f);
            exprs(index, f);
        }
        ExprKind::Call { callee, args, .. } => {
            exprs(callee, f);
            for a in args {
                exprs(&mut a.value, f);
            }
        }
        ExprKind::Map(entries) => {
            for (k, v) in entries {
                exprs(k, f);
                exprs(v, f);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            exprs(scrutinee, f);
            for arm in arms {
                match &mut arm.body {
                    ArmBody::Expr(x) => exprs(x, f),
                    ArmBody::Block(b) => exprs_in_block(b, f),
                }
            }
        }
        ExprKind::Lambda { body, .. } => match &mut **body {
            ArmBody::Expr(x) => exprs(x, f),
            ArmBody::Block(b) => exprs_in_block(b, f),
        },
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::Duration(_)
        | ExprKind::Name(_) => {}
    }
}
