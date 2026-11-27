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
    Span, Stmt, StmtKind, TypeExpr, TypeExprKind,
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

/// Standard modules (`use std.ملفات` is `use std.fs`): (Arabic, English).
pub const MODULE_NAMES: &[(&str, &str)] = &[
    ("ذكاء", "ai"),
    ("وكيل", "agent"),
    ("ملفات", "fs"),
    ("جيسون", "json"),
    ("ويب", "http"),
    ("بيانات", "db"),
    ("بايثون", "py"),
    ("أدوات", "mcp"),
    ("رياضيات", "math"),
    ("عشوائي", "random"),
    ("وقت", "time"),
    ("نظام", "os"),
    ("أنماط", "regex"),
    ("ترميز", "encoding"),
    ("تشفير", "crypto"),
    ("خادم", "server"),
    ("عملية", "process"),
    ("جدول", "csv"),
];

/// Functions and constants of the standard modules: (module, Arabic,
/// English); `ملفات.اقرأ_نص` is `fs.read_text`.
pub const MODULE_FUNCTIONS: &[(&str, &str, &str)] = &[
    ("ai", "استنتج", "infer"),
    ("ai", "شغل", "run"),
    ("ai", "ضمن", "embed"),
    ("ai", "تشابه", "similarity"),
    ("ai", "ابحث", "search"),
    ("ai", "بث", "stream"),
    ("agent", "شغل", "run"),
    ("fs", "اقرأ_نص", "read_text"),
    ("fs", "اكتب_نص", "write_text"),
    ("fs", "ألحق_نص", "append_text"),
    ("fs", "موجود", "exists"),
    ("fs", "محتويات", "list_dir"),
    ("fs", "هل_مجلد", "is_dir"),
    ("fs", "حجم", "size"),
    ("fs", "أنشئ_مجلد", "create_dir"),
    ("fs", "احذف", "remove"),
    ("fs", "احذف_مجلد", "remove_dir"),
    ("fs", "أعد_تسمية", "rename"),
    ("fs", "انسخ", "copy"),
    ("json", "حلل", "parse"),
    ("json", "إلى_نص", "stringify"),
    ("json", "منسق", "pretty"),
    ("http", "اجلب", "get"),
    ("http", "أرسل", "post"),
    ("db", "افتح", "open"),
    ("db", "شغل", "execute"),
    ("db", "استعلم", "query"),
    ("db", "استعلم_واحدا", "query_one"),
    ("py", "استدع", "call"),
    ("mcp", "الأدوات", "tools"),
    ("mcp", "استدع", "call"),
    ("math", "جذر", "sqrt"),
    ("math", "جذر_تكعيبي", "cbrt"),
    ("math", "أس", "pow"),
    ("math", "أسي", "exp"),
    ("math", "لو_طبيعي", "ln"),
    ("math", "لو10", "log10"),
    ("math", "لو2", "log2"),
    ("math", "جا", "sin"),
    ("math", "جتا", "cos"),
    ("math", "ظا", "tan"),
    ("math", "جا_عكسي", "asin"),
    ("math", "جتا_عكسي", "acos"),
    ("math", "ظا_عكسي", "atan"),
    ("math", "ظا_عكسي2", "atan2"),
    ("math", "وتر", "hypot"),
    ("math", "ليس_عددا", "is_nan"),
    ("math", "باي", "pi"),
    ("math", "هـ", "e"),
    ("math", "تاو", "tau"),
    ("math", "لانهاية", "inf"),
    ("random", "صحيح", "int"),
    ("random", "عشري", "float"),
    ("random", "منطقي", "bool"),
    ("random", "اختر", "choice"),
    ("random", "اخلط", "shuffle"),
    ("random", "بذرة", "seed"),
    ("time", "الآن", "now"),
    ("time", "نم", "sleep"),
    ("time", "نسق", "format"),
    ("time", "حلل", "parse"),
    ("time", "اليوم", "today"),
    ("time", "تاريخ", "date"),
    ("time", "اصنع", "make"),
    ("time", "أضف_أياما", "add_days"),
    ("time", "أضف_أشهرا", "add_months"),
    ("time", "أيام_بين", "days_between"),
    ("time", "نسق_بصيغة", "format_as"),
    ("os", "المعطيات", "args"),
    ("os", "بيئة", "env"),
    ("os", "المنصة", "platform"),
    ("os", "اخرج", "exit"),
    ("regex", "يطابق", "is_match"),
    ("regex", "ابحث", "find"),
    ("regex", "ابحث_الكل", "find_all"),
    ("regex", "التقط", "captures"),
    ("regex", "استبدل", "replace"),
    ("regex", "قسم", "split"),
    ("encoding", "إلى_قاعدة64", "base64_encode"),
    ("encoding", "من_قاعدة64", "base64_decode"),
    ("encoding", "إلى_ست_عشري", "hex_encode"),
    ("encoding", "من_ست_عشري", "hex_decode"),
    ("encoding", "ترميز_رابط", "url_encode"),
    ("encoding", "فك_رابط", "url_decode"),
    ("crypto", "بصمة", "sha256"),
    ("crypto", "توقيع", "hmac_sha256"),
    ("crypto", "معرف_فريد", "uuid"),
    ("crypto", "رمز_سري", "token"),
    ("server", "شغل", "serve"),
    ("server", "نص", "text"),
    ("server", "صفحة", "html"),
    ("server", "جيسون", "json"),
    ("server", "حول", "redirect"),
    ("process", "شغل", "run"),
    ("csv", "حلل", "parse"),
    ("csv", "حلل_سجلات", "parse_records"),
    ("csv", "إلى_نص", "stringify"),
];

/// Effects in `!{...}`: (Arabic, English).
pub const EFFECT_NAMES: &[(&str, &str)] = &[
    ("إدخال_إخراج", "io"),
    ("ملفات", "fs"),
    ("شبكة", "net"),
    ("ذكاء", "ai"),
    ("بيانات", "db"),
    ("بايثون", "py"),
    ("أدوات", "mcp"),
    ("عملية", "process"),
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

fn arabic_of(table: &[(&'static str, &str)], english: &str) -> Option<&'static str> {
    table
        .iter()
        .find(|(_, en)| *en == english)
        .map(|(ar, _)| *ar)
}

/// The Arabic name of the built-in function or constructor `english`.
pub fn value_arabic(english: &str) -> Option<&'static str> {
    VALUE_NAMES
        .iter()
        .find(|(_, en)| *en == english)
        .map(|(ar, _)| *ar)
}

/// The Arabic name of the built-in type `english`.
pub fn type_arabic(english: &str) -> Option<&'static str> {
    TYPE_NAMES
        .iter()
        .find(|(_, en)| *en == english)
        .map(|(ar, _)| *ar)
}

/// The English name of a standard module written in Arabic.
pub fn module_name(name: &str) -> Option<&'static str> {
    lookup(MODULE_NAMES, name)
}

/// The English name of function `name` of standard module `module`
/// (English), written in Arabic.
pub fn module_function(module: &str, name: &str) -> Option<&'static str> {
    if name.is_ascii() {
        return None;
    }
    let k = key(name);
    MODULE_FUNCTIONS
        .iter()
        .find(|(m, ar, _)| *m == module && key(ar) == k)
        .map(|(_, _, en)| *en)
}

/// The English name of an effect written in Arabic.
pub fn effect_name(name: &str) -> Option<&'static str> {
    lookup(EFFECT_NAMES, name)
}

fn module_english(name: &str) -> Option<&'static str> {
    MODULE_NAMES
        .iter()
        .find(|(_, en)| *en == name)
        .map(|(_, en)| *en)
        .or_else(|| module_name(name))
}

/// What a built-in name stands for, for [`BuiltinUse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinKind {
    /// A function or constructor ([`VALUE_NAMES`]).
    Value,
    /// A type ([`TYPE_NAMES`]).
    Type,
    /// A method or `length` ([`METHOD_NAMES`]).
    Member,
    /// A standard module ([`MODULE_NAMES`]).
    Module,
    /// A function of the standard module `module` ([`MODULE_FUNCTIONS`]).
    ModuleFunction { module: &'static str },
    /// An effect in `!{...}` ([`EFFECT_NAMES`]).
    Effect,
}

/// One place where a program names a built-in, in either language: what
/// `elomar fmt --keywords` needs to write it in the other one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinUse {
    pub span: Span,
    pub kind: BuiltinKind,
    /// The English name (`print`).
    pub english: &'static str,
    /// Whether the English name, written here, would still mean the
    /// built-in (no name of the program hides it).
    pub english_free: bool,
    /// The same for the Arabic name.
    pub arabic_free: bool,
}

impl BuiltinUse {
    /// The name in `arabic` or English, if writing it here keeps the meaning.
    pub fn spelling(&self, arabic: bool) -> Option<&'static str> {
        if arabic {
            let ar = match self.kind {
                BuiltinKind::Value => value_arabic(self.english),
                BuiltinKind::Type => type_arabic(self.english),
                BuiltinKind::Member => method_arabic(self.english),
                BuiltinKind::Module => arabic_of(MODULE_NAMES, self.english),
                BuiltinKind::ModuleFunction { module } => MODULE_FUNCTIONS
                    .iter()
                    .find(|(m, _, en)| *m == module && *en == self.english)
                    .map(|(_, ar, _)| *ar),
                BuiltinKind::Effect => arabic_of(EFFECT_NAMES, self.english),
            };
            ar.filter(|_| self.arabic_free)
        } else {
            self.english_free.then_some(self.english)
        }
    }
}

/// Rewrites the Arabic built-in names in `program` to English, except where
/// the program's own names hide them.
pub fn resolve(program: &mut Program) {
    resolve_recording(program);
}

/// Like [`resolve`], also returning every place that names a built-in
/// function, constructor or type (in English or in Arabic), in source order.
pub fn resolve_recording(program: &mut Program) -> Vec<BuiltinUse> {
    let mut r = Resolver {
        values: vec![HashSet::new()],
        types: vec![HashSet::new()],
        uses: Vec::new(),
        modules: HashSet::new(),
    };
    // `use std.ملفات` is `use std.fs`; either spelling of an imported module
    // then names it (`ملفات.اقرأ_نص` and `fs.read_text`).
    for item in &mut program.items {
        if let Item::Use(u) = item
            && u.names.is_empty()
            && u.path.len() == 2
            && u.path[0].name == "std"
            && let Some(en) = module_english(&u.path[1].name)
        {
            let ident = &mut u.path[1];
            r.uses.push(BuiltinUse {
                span: ident.span,
                kind: BuiltinKind::Module,
                english: en,
                english_free: true,
                arabic_free: true,
            });
            ident.name = en.to_string();
            r.modules.insert(en);
        }
    }
    for item in &program.items {
        match item {
            Item::Use(u) if u.names.is_empty() && u.path.len() == 2 && u.path[0].name == "std" => {}
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
    r.uses.sort_by_key(|u| u.span.start);
    r.uses
}

struct Resolver {
    values: Vec<HashSet<String>>,
    types: Vec<HashSet<String>>,
    uses: Vec<BuiltinUse>,
    /// Standard modules the program imports (English names).
    modules: HashSet<&'static str>,
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

    /// The English name of `name` when it means a built-in function or
    /// constructor here, recording the use.
    fn value_use(&mut self, name: &str, span: Span) -> Option<&'static str> {
        if self.hides_value(name) {
            return None;
        }
        let (en, english_free, arabic_free) = if let Some(en) = value_name(name) {
            (en, !self.hides_value(en), true)
        } else {
            let (ar, en) = VALUE_NAMES.iter().find(|(_, en)| *en == name)?;
            (*en, true, !self.hides_value(ar))
        };
        self.uses.push(BuiltinUse {
            span,
            kind: BuiltinKind::Value,
            english: en,
            english_free,
            arabic_free,
        });
        Some(en)
    }

    /// The English name of `name` when it names an imported standard module
    /// here (in either language), recording the use.
    fn module_use(&mut self, name: &str, span: Span) -> Option<&'static str> {
        let en = module_english(name).filter(|en| self.modules.contains(en))?;
        if self.hides_value(name) {
            return None;
        }
        let ar = arabic_of(MODULE_NAMES, en).unwrap_or(en);
        self.uses.push(BuiltinUse {
            span,
            kind: BuiltinKind::Module,
            english: en,
            english_free: !self.hides_value(en),
            arabic_free: !self.hides_value(ar),
        });
        Some(en)
    }

    /// Effects in `!{...}` written in Arabic become English.
    fn effects(&mut self, effects: &mut Option<crate::EffectSet>) {
        for e in effects.iter_mut().flat_map(|s| s.effects.iter_mut()) {
            let en = effect_name(&e.name).or_else(|| {
                EFFECT_NAMES
                    .iter()
                    .find(|(_, en)| *en == e.name)
                    .map(|(_, en)| *en)
            });
            if let Some(en) = en {
                self.uses.push(BuiltinUse {
                    span: e.span,
                    kind: BuiltinKind::Effect,
                    english: en,
                    english_free: true,
                    arabic_free: true,
                });
                e.name = en.to_string();
            }
        }
    }

    /// The same for a type.
    fn type_use(&mut self, name: &str, span: Span) -> Option<&'static str> {
        if self.hides_type(name) {
            return None;
        }
        let (en, english_free, arabic_free) = if let Some(en) = type_name(name) {
            (en, !self.hides_type(en), true)
        } else {
            let (ar, en) = TYPE_NAMES.iter().find(|(_, en)| *en == name)?;
            (*en, true, !self.hides_type(ar))
        };
        self.uses.push(BuiltinUse {
            span,
            kind: BuiltinKind::Type,
            english: en,
            english_free,
            arabic_free,
        });
        Some(en)
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
                    self.effects(&mut m.effects);
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
        self.effects(&mut f.effects);
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
                if let Some(en) = self.type_use(&name.name, name.span) {
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
                if let Some(en) = self.value_use(n, e.span) {
                    *n = en.to_string();
                }
            }
            // `ملفات.اقرأ_نص`: a function of a standard module.
            ExprKind::Field { target, name }
                if matches!(&target.kind, ExprKind::Name(m)
                    if module_english(m).is_some_and(|en| self.modules.contains(en))
                        && !self.hides_value(m)) =>
            {
                let ExprKind::Name(m) = &mut target.kind else {
                    unreachable!()
                };
                let module = self.module_use(m, target.span).expect("checked above");
                *m = module.to_string();
                let en = module_function(module, &name.name).or_else(|| {
                    MODULE_FUNCTIONS
                        .iter()
                        .find(|(md, _, en)| *md == module && *en == name.name)
                        .map(|(_, _, en)| *en)
                });
                if let Some(en) = en {
                    self.uses.push(BuiltinUse {
                        span: name.span,
                        kind: BuiltinKind::ModuleFunction { module },
                        english: en,
                        english_free: true,
                        arabic_free: true,
                    });
                    name.name = en.to_string();
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
            PatternKind::Name(n) => {
                let is_none = value_name(n) == Some("None") || n == "None";
                match is_none
                    .then(|| self.value_use(&n.clone(), p.span))
                    .flatten()
                {
                    Some(en) => *n = en.to_string(),
                    None => self.value(&n.clone()),
                }
            }
            PatternKind::Ctor { name, args } => {
                if let Some(en) = self.value_use(&name.name, name.span) {
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
