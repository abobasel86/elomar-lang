//! Arabic names for the built-in functions, constructors and types:
//! `اطبع` is `print`, `نص` is `String`, `قيمة(x)` is `Some(x)`.
//!
//! They are not reserved words. Right after parsing, [`resolve`] rewrites
//! each one to its English name, unless the program defines that name where
//! it is used (a function, type, variable, parameter or pattern), which then
//! hides the built-in, as it would hide `print`. Everything after the parser
//! (the checker and both engines) sees only the English names.

use std::collections::HashSet;

use crate::{
    ArmBody, Block, Else, Expr, ExprKind, FnDecl, InterpPart, Item, Pattern, PatternKind, Program,
    Stmt, StmtKind, TypeExpr, TypeExprKind,
};

/// Built-in functions and value constructors: (Arabic, English).
pub const VALUE_NAMES: &[(&str, &str)] = &[
    ("اطبع", "print"),
    ("اقرأ", "input"),
    ("إلى_نص", "to_string"),
    ("إلى_صحيح", "to_int"),
    ("إلى_عشري", "to_float"),
    ("حلل_صحيح", "parse_int"),
    ("حلل_عشري", "parse_float"),
    ("تأكد", "assert"),
    ("تأكد_تساوي", "assert_eq"),
    ("مجموعة", "set"),
    ("أصغر", "min"),
    ("أكبر", "max"),
    ("مطلق", "abs"),
    ("الكل", "all"),
    ("مهلة", "timeout"),
    ("قيمة", "Some"),
    ("لاشيء", "None"),
    ("نجاح", "Ok"),
    ("فشل", "Err"),
];

/// Built-in types: (Arabic, English).
pub const TYPE_NAMES: &[(&str, &str)] = &[
    ("صحيح", "Int"),
    ("عشري", "Float"),
    ("منطقي", "Bool"),
    ("نص", "String"),
    ("قائمة", "List"),
    ("مجموعة", "Set"),
    ("قاموس", "Map"),
    ("اختياري", "Option"),
    ("نتيجة", "Result"),
    ("مدة", "Duration"),
];

/// Methods and the `length` property of the built-in types: (Arabic,
/// English). Unlike the names above, the checker resolves these, since only
/// the type of the value says whether `.أضف` is `push` (a list) or `add` (a
/// set); a record field or a method from an `impl` with the same name wins.
pub const METHOD_NAMES: &[(&str, &str)] = &[
    ("طول", "length"),
    ("أضف", "push"),
    ("أضف", "add"),
    ("أخرج", "pop"),
    ("أدرج", "insert"),
    ("احذف", "remove"),
    ("يحتوي", "contains"),
    ("موقع", "index_of"),
    ("فارغ", "is_empty"),
    ("الأول", "first"),
    ("الأخير", "last"),
    ("اعكس", "reverse"),
    ("مرتبة", "sorted"),
    ("اربط", "join"),
    ("مجموع", "sum"),
    ("اقتطع", "slice"),
    ("حول", "map"),
    ("رشح", "filter"),
    ("أي", "any"),
    ("الكل", "all"),
    ("ابحث", "find"),
    ("عد", "count"),
    ("اطو", "fold"),
    ("رتب_حسب", "sort_by"),
    ("خذ", "take"),
    ("تخط", "skip"),
    ("فريدة", "unique"),
    ("أصغر", "min"),
    ("أكبر", "max"),
    ("مرقمة", "enumerate"),
    ("قطع", "chunks"),
    ("حول_وادمج", "flat_map"),
    ("جمع_حسب", "group_by"),
    ("زاوج", "zip"),
    ("نوافذ", "windows"),
    ("مجموع_حسب", "sum_by"),
    ("أصغر_حسب", "min_by"),
    ("أكبر_حسب", "max_by"),
    ("حول_بالتوازي", "par_map"),
    ("رشح_بالتوازي", "par_filter"),
    ("اطو_بالتوازي", "par_fold"),
    ("رتب_حسب_بالتوازي", "par_sort_by"),
    ("أي_بالتوازي", "par_any"),
    ("الكل_بالتوازي", "par_all"),
    ("ابحث_بالتوازي", "par_find"),
    ("عد_بالتوازي", "par_count"),
    ("حول_وادمج_بالتوازي", "par_flat_map"),
    ("جمع_حسب_بالتوازي", "par_group_by"),
    ("مجموع_حسب_بالتوازي", "par_sum_by"),
    ("أصغر_حسب_بالتوازي", "par_min_by"),
    ("أكبر_حسب_بالتوازي", "par_max_by"),
    ("إلى_نص", "to_string"),
    ("أحرف_كبيرة", "upper"),
    ("أحرف_صغيرة", "lower"),
    ("شذب", "trim"),
    ("شذب_البداية", "trim_start"),
    ("شذب_النهاية", "trim_end"),
    ("يبدأ_ب", "starts_with"),
    ("ينتهي_ب", "ends_with"),
    ("قسم", "split"),
    ("حروف", "chars"),
    ("استبدل", "replace"),
    ("كرر", "repeat"),
    ("حلل_صحيح", "parse_int"),
    ("حلل_عشري", "parse_float"),
    ("أسطر", "lines"),
    ("كلمات", "words"),
    ("املأ_البداية", "pad_start"),
    ("املأ_النهاية", "pad_end"),
    ("إلى_عشري", "to_float"),
    ("إلى_صحيح", "to_int"),
    ("مطلق", "abs"),
    ("أس", "pow"),
    ("حصر", "clamp"),
    ("قرب", "round"),
    ("قرب_لأسفل", "floor"),
    ("قرب_لأعلى", "ceil"),
    ("جذر", "sqrt"),
    ("قرب_إلى", "round_to"),
    ("بخانات", "to_fixed"),
    ("فيه_قيمة", "is_some"),
    ("بلا_قيمة", "is_none"),
    ("القيمة_أو", "unwrap_or"),
    ("القيمة_أو_احسب", "unwrap_or_else"),
    ("ثم", "and_then"),
    ("أو", "or"),
    ("أو_فشل", "ok_or"),
    ("قيمة_تحقق", "is_some_and"),
    ("ناجحة", "is_ok"),
    ("فاشلة", "is_err"),
    ("حول_الفشل", "map_err"),
    ("النجاح", "ok"),
    ("الفشل", "err"),
    ("بالمللي", "millis"),
    ("بالثواني", "seconds"),
    ("اجلب", "get"),
    ("اجلب_أو", "get_or"),
    ("المدخلات", "entries"),
    ("فيه_مفتاح", "contains_key"),
    ("المفاتيح", "keys"),
    ("القيم", "values"),
    ("حول_القيم", "map_values"),
    ("النوع", "kind"),
    ("الرسالة", "message"),
    ("إلى_قائمة", "to_list"),
    ("اتحاد", "union"),
    ("تقاطع", "intersection"),
    ("فرق", "difference"),
    ("جزء_من", "is_subset"),
];

/// `s` without the differences an Arabic name may be written with: hamza on
/// alef, `ى` for `ي`, `ة` for `ه`, harakat, shadda and tatweel (as for the
/// Arabic keywords).
fn key(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}'))
        .map(|c| match c {
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            'ى' => 'ي',
            'ة' => 'ه',
            c => c,
        })
        .collect()
}

fn lookup(table: &[(&str, &'static str)], name: &str) -> Option<&'static str> {
    if name.is_ascii() {
        return None;
    }
    let k = key(name);
    table.iter().find(|(ar, _)| key(ar) == k).map(|(_, en)| *en)
}

/// The English name of a built-in function or constructor written in Arabic.
pub fn value_name(name: &str) -> Option<&'static str> {
    lookup(VALUE_NAMES, name)
}

/// The English name of a built-in type written in Arabic.
pub fn type_name(name: &str) -> Option<&'static str> {
    lookup(TYPE_NAMES, name)
}

/// The English names a method or property written in Arabic may stand for
/// (more than one when types differ, as `أضف` is `push` and `add`).
pub fn method_names(name: &str) -> impl Iterator<Item = &'static str> + '_ {
    let k = (!name.is_ascii()).then(|| key(name));
    METHOD_NAMES
        .iter()
        .filter(move |(ar, _)| k.as_deref() == Some(key(ar).as_str()))
        .map(|(_, en)| *en)
}

/// The Arabic name of the built-in method or property `english`.
pub fn method_arabic(english: &str) -> Option<&'static str> {
    METHOD_NAMES
        .iter()
        .find(|(_, en)| *en == english)
        .map(|(ar, _)| *ar)
}

/// Rewrites the Arabic built-in names in `program` to English, except where
/// the program's own names hide them.
pub fn resolve(program: &mut Program) {
    let mut r = Resolver {
        values: vec![HashSet::new()],
        types: vec![HashSet::new()],
    };
    for item in &program.items {
        match item {
            Item::Fn(f) => r.value(&f.name.name),
            Item::Const(c) => r.value(&c.name.name),
            Item::Type(t) => r.both(&t.name.name),
            Item::Enum(e) => r.both(&e.name.name),
            Item::Trait(t) => r.both(&t.name.name),
            Item::Use(u) => {
                for n in &u.names {
                    r.both(&n.name);
                }
                if let Some(last) = u.path.last() {
                    r.value(&last.name);
                }
            }
            Item::Impl(_) => {}
        }
    }
    for item in &mut program.items {
        r.item(item);
    }
}

struct Resolver {
    values: Vec<HashSet<String>>,
    types: Vec<HashSet<String>>,
}

impl Resolver {
    fn value(&mut self, name: &str) {
        self.values.last_mut().unwrap().insert(name.to_string());
    }

    fn both(&mut self, name: &str) {
        self.value(name);
        self.types.last_mut().unwrap().insert(name.to_string());
    }

    fn push(&mut self) {
        self.values.push(HashSet::new());
        self.types.push(HashSet::new());
    }

    fn pop(&mut self) {
        self.values.pop();
        self.types.pop();
    }

    fn hides_value(&self, name: &str) -> bool {
        self.values.iter().any(|s| s.contains(name))
    }

    fn hides_type(&self, name: &str) -> bool {
        self.types.iter().any(|s| s.contains(name))
    }

    fn value_alias(&self, name: &str) -> Option<&'static str> {
        value_name(name).filter(|_| !self.hides_value(name))
    }

    fn item(&mut self, item: &mut Item) {
        match item {
            Item::Fn(f) => self.function(f),
            Item::Const(c) => {
                if let Some(t) = &mut c.ty {
                    self.ty(t);
                }
                self.expr(&mut c.value);
            }
            Item::Type(t) => {
                self.push();
                for p in &t.type_params {
                    self.types.last_mut().unwrap().insert(p.name.clone());
                }
                self.ty(&mut t.ty);
                self.pop();
            }
            Item::Enum(e) => {
                self.push();
                for p in &e.type_params {
                    self.types.last_mut().unwrap().insert(p.name.clone());
                }
                for v in &mut e.variants {
                    for f in &mut v.fields {
                        self.ty(f);
                    }
                }
                self.pop();
            }
            Item::Trait(t) => {
                for m in &mut t.methods {
                    self.push();
                    for p in &mut m.params {
                        self.ty(&mut p.ty);
                        self.value(&p.name.name);
                    }
                    if let Some(ret) = &mut m.ret {
                        self.ty(ret);
                    }
                    if let Some(body) = &mut m.body {
                        self.block(body);
                    }
                    self.pop();
                }
            }
            Item::Impl(i) => {
                self.push();
                for p in &i.type_params {
                    self.types.last_mut().unwrap().insert(p.name.clone());
                }
                self.ty(&mut i.target);
                for m in &mut i.methods {
                    self.function(m);
                }
                self.pop();
            }
            Item::Use(_) => {}
        }
    }

    fn function(&mut self, f: &mut FnDecl) {
        self.push();
        for p in &f.type_params {
            self.types.last_mut().unwrap().insert(p.name.clone());
        }
        for p in &mut f.params {
            self.ty(&mut p.ty);
            self.value(&p.name.name);
        }
        if let Some(ret) = &mut f.ret {
            self.ty(ret);
        }
        self.block(&mut f.body);
        self.pop();
    }

    fn ty(&mut self, t: &mut TypeExpr) {
        match &mut t.kind {
            TypeExprKind::Named { name, args } => {
                if let Some(en) = type_name(&name.name).filter(|_| !self.hides_type(&name.name)) {
                    name.name = en.to_string();
                }
                for a in args {
                    self.ty(a);
                }
            }
            TypeExprKind::Record(fields) => {
                for (_, f) in fields {
                    self.ty(f);
                }
            }
            TypeExprKind::Tuple(items) => {
                for i in items {
                    self.ty(i);
                }
            }
            TypeExprKind::Fn { params, ret } => {
                for p in params {
                    self.ty(p);
                }
                self.ty(ret);
            }
        }
    }

    fn block(&mut self, b: &mut Block) {
        self.push();
        for s in &mut b.stmts {
            self.stmt(s);
        }
        self.pop();
    }

    fn stmt(&mut self, s: &mut Stmt) {
        match &mut s.kind {
            StmtKind::Let {
                name, ty, value, ..
            } => {
                if let Some(t) = ty {
                    self.ty(t);
                }
                self.expr(value);
                self.value(&name.name);
            }
            StmtKind::LetTuple { names, value, .. } => {
                self.expr(value);
                for n in names.iter() {
                    self.value(&n.name);
                }
            }
            StmtKind::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond);
                self.block(then);
                match otherwise {
                    Some(Else::Block(b)) => self.block(b),
                    Some(Else::If(stmt)) => self.stmt(stmt),
                    None => {}
                }
            }
            StmtKind::While { cond, body } => {
                self.expr(cond);
                self.block(body);
            }
            StmtKind::For { vars, iter, body } => {
                self.expr(iter);
                self.push();
                for v in vars.iter() {
                    self.value(&v.name);
                }
                self.block(body);
                self.pop();
            }
            StmtKind::Return(Some(e)) | StmtKind::Expr(e) => self.expr(e),
            StmtKind::Block(b) => self.block(b),
            StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn expr(&mut self, e: &mut Expr) {
        match &mut e.kind {
            ExprKind::Name(n) => {
                if let Some(en) = self.value_alias(n) {
                    *n = en.to_string();
                }
            }
            ExprKind::Interp(parts) => {
                for p in parts {
                    if let InterpPart::Expr(e) = p {
                        self.expr(e);
                    }
                }
            }
            ExprKind::Tuple(items) | ExprKind::List(items) => {
                for i in items {
                    self.expr(i);
                }
            }
            ExprKind::Record(fields) => {
                for f in fields {
                    self.expr(&mut f.value);
                }
            }
            ExprKind::Unary { expr, .. } | ExprKind::Await(expr) | ExprKind::Try(expr) => {
                self.expr(expr)
            }
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Range { start, end, .. } => {
                self.expr(start);
                self.expr(end);
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                self.expr(callee);
                for t in type_args {
                    self.ty(t);
                }
                for a in args {
                    self.expr(&mut a.value);
                }
            }
            ExprKind::Field { target, .. } => self.expr(target),
            ExprKind::Index { target, index } => {
                self.expr(target);
                self.expr(index);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                for arm in arms {
                    self.push();
                    self.pattern(&mut arm.pattern);
                    self.arm_body(&mut arm.body);
                    self.pop();
                }
            }
            ExprKind::Map(entries) => {
                for (k, v) in entries {
                    self.expr(k);
                    self.expr(v);
                }
            }
            ExprKind::Lambda { params, body } => {
                self.push();
                for p in params.iter_mut() {
                    if let Some(t) = &mut p.ty {
                        self.ty(t);
                    }
                    self.value(&p.name.name);
                }
                self.arm_body(body);
                self.pop();
            }
            ExprKind::Variant { args, .. } => {
                for a in args {
                    self.expr(a);
                }
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Duration(_) => {}
        }
    }

    fn arm_body(&mut self, body: &mut ArmBody) {
        match body {
            ArmBody::Expr(e) => self.expr(e),
            ArmBody::Block(b) => self.block(b),
        }
    }

    /// Resolves constructors in a pattern and adds the names it binds.
    fn pattern(&mut self, p: &mut Pattern) {
        match &mut p.kind {
            PatternKind::Name(n) => match self.value_alias(n) {
                Some("None") => *n = "None".to_string(),
                _ => self.value(&n.clone()),
            },
            PatternKind::Ctor { name, args } => {
                if let Some(en) = self.value_alias(&name.name) {
                    name.name = en.to_string();
                }
                for a in args {
                    self.pattern(a);
                }
            }
            PatternKind::Variant { args, .. } | PatternKind::Tuple(args) => {
                for a in args {
                    self.pattern(a);
                }
            }
            PatternKind::Or(alts) => {
                for a in alts {
                    self.pattern(a);
                }
            }
            PatternKind::Wildcard
            | PatternKind::Int(_)
            | PatternKind::Float(_)
            | PatternKind::Str(_)
            | PatternKind::Bool(_)
            | PatternKind::Range { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_with_or_without_hamza() {
        assert_eq!(value_name("اطبع"), Some("print"));
        assert_eq!(value_name("اكبر"), Some("max"));
        assert_eq!(value_name("الى_نص"), Some("to_string"));
        assert_eq!(value_name("تاكّد"), Some("assert"));
        assert_eq!(type_name("مده"), Some("Duration"));
        assert_eq!(type_name("نص"), Some("String"));
        assert_eq!(value_name("نص"), None);
        assert_eq!(type_name("print"), None);
    }
}
