use super::*;

fn diags(src: &str) -> Vec<Diagnostic> {
    let (mut program, parse_diags) = elomar_parser::parse(src);
    assert!(parse_diags.is_empty(), "parse errors: {parse_diags:#?}");
    check(
        &mut program,
        CheckOptions {
            require_main: false,
            ..Default::default()
        },
    )
}

fn codes(src: &str) -> Vec<&'static str> {
    diags(src).iter().map(|d| d.code).collect()
}

fn ok(src: &str) {
    let d = diags(src);
    assert!(d.is_empty(), "unexpected diagnostics: {d:#?}");
}

#[test]
fn accepts_basic_programs() {
    ok(
        "fn sum(a: Int, b: Int) -> Int {\n  return a + b\n}\nfn main() {\n  let total = sum(10, 20)\n  print(total)\n}",
    );
    ok(
        "fn main() {\n  var xs = []\n  xs.push(1)\n  for x in xs { print(x) }\n  for i in 0..3 { print(i) }\n}",
    );
    ok(
        "type P = { x: Int, y: Int }\nfn norm(p: P) -> Int {\n  return p.x * p.x + p.y * p.y\n}\nfn main() {\n  print(norm({ y: 2, x: 1 }))\n}",
    );
}

#[test]
fn division_always_produces_float() {
    ok("fn avg(a: Int, b: Int) -> Float {\n  return (a + b) / 2\n}");
    ok("fn half(a: Int) -> Int {\n  return a div 2\n}");
    assert_eq!(
        codes("fn f(a: Int) -> Int {\n  return a / 2\n}"),
        vec!["E2001"]
    );
    assert_eq!(
        codes("fn f(a: Float) -> Int {\n  return to_int(a div 2.0)\n}"),
        vec!["E2005"]
    );
}

#[test]
fn int_literals_become_floats_where_expected() {
    let (mut program, _) = elomar_parser::parse("fn main() {\n  var t: Float = 0\n  t += 1\n}");
    assert!(check(&mut program, CheckOptions::default()).is_empty());
    let Item::Fn(f) = &program.items[0] else {
        panic!()
    };
    let StmtKind::Let { value, .. } = &f.body.stmts[0].kind else {
        panic!()
    };
    assert_eq!(value.kind, ExprKind::Float(0.0));
}

#[test]
fn mixed_arithmetic_promotes_to_float() {
    ok("fn f(a: Int, b: Float) -> Float {\n  return a * b + 1\n}");
    assert_eq!(
        codes("fn f(a: Int, b: Float) -> Int {\n  return a + b\n}"),
        vec!["E2001"]
    );
}

#[test]
fn reports_type_mismatch_and_unknown_names() {
    assert_eq!(
        codes("fn main() {\n  let x: Int = \"hi\"\n}"),
        vec!["E2001"]
    );
    let d = diags("fn main() {\n  let count = 1\n  print(cuont)\n}");
    assert_eq!(d[0].code, "E1001");
    assert_eq!(d[0].hint.as_ref().unwrap().en, "did you mean `count`?");
    assert_eq!(codes("fn main() {\n  print(\"a\" + 1)\n}"), vec!["E2005"]);
    assert_eq!(codes("fn main() {\n  if 1 { }\n}"), vec!["E2003"]);
}

#[test]
fn immutability() {
    assert_eq!(codes("fn main() {\n  let x = 1\n  x = 2\n}"), vec!["E1003"]);
    assert_eq!(
        codes("fn main() {\n  let xs = [1]\n  xs.push(2)\n}"),
        vec!["E1003"]
    );
    assert_eq!(
        codes("fn f(xs: List<Int>) {\n  xs[0] = 2\n}"),
        vec!["E1003"]
    );
    ok("fn main() {\n  var r = { a: [1] }\n  r.a[0] = 5\n  r.a.push(3)\n}");
}

#[test]
fn scopes_and_duplicates() {
    assert_eq!(
        codes("fn main() {\n  let a = 1\n  let a = 2\n}"),
        vec!["E1002"]
    );
    assert_eq!(
        codes("fn main() {\n  let a = 1\n  if true {\n    let a = 2\n  }\n}"),
        vec!["W1001"]
    );
    assert_eq!(codes("fn f() {}\nfn f() {}"), vec!["E1002"]);
    assert_eq!(codes("fn print() {}"), vec!["E1002"]);
    assert_eq!(codes("fn main() {\n  break\n}"), vec!["E1004"]);
}

#[test]
fn missing_return() {
    assert_eq!(
        codes("fn f(a: Int) -> Int {\n  if a > 0 { return 1 }\n}"),
        vec!["E2007"]
    );
    ok("fn f(a: Int) -> Int {\n  if a > 0 { return 1 } else { return 2 }\n}");
    ok("fn f(a: Int) -> Int {\n  while true { return a }\n}");
}

#[test]
fn calls_and_named_arguments() {
    ok(
        "fn greet(name: String, times: Int) {}\nfn main() {\n  greet(\"a\", times: 2)\n  greet(times: 1, name: \"b\")\n}",
    );
    assert_eq!(
        codes("fn g(a: Int) {}\nfn main() {\n  g(1, 2)\n}"),
        vec!["E2002"]
    );
    assert_eq!(
        codes("fn g(a: Int) {}\nfn main() {\n  g(b: 1)\n}"),
        vec!["E2010", "E2002"]
    );
    assert_eq!(codes("fn main() {\n  let x = 1\n  x()\n}"), vec!["E2004"]);
}

#[test]
fn options_results_and_match() {
    ok(
        "fn safe(a: Int, b: Int) -> Result<Int, String> {\n  if b == 0 { return Err(\"zero\") }\n  return Ok(a div b)\n}\nfn main() {\n  match safe(4, 2) {\n    Ok(v) => print(v)\n    Err(e) => print(e)\n  }\n  let n = match parse_int(\"5\") { Some(v) => v, None => 0 }\n  print(n + 1)\n}",
    );
    assert_eq!(
        codes("fn main() {\n  match Some(1) {\n    Some(v) => print(v)\n  }\n}"),
        vec!["E2009"]
    );
    assert_eq!(
        codes("fn main() {\n  let x = match 3 { 1 => \"one\", 2 => 2, _ => \"many\" }\n}"),
        vec!["E2001"]
    );
}

#[test]
fn strings_are_not_indexable() {
    let d = diags("fn main() {\n  let s = \"abc\"\n  print(s[0])\n}");
    assert_eq!(d[0].code, "E2006");
    ok("fn main() {\n  let s = \"مرحبا\"\n  print(s.chars()[0], s.length, s.upper())\n}");
}

#[test]
fn methods() {
    assert_eq!(codes("fn main() {\n  print([1].lenght)\n}"), vec!["E1007"]);
    assert_eq!(
        codes("fn main() {\n  print([1].pussh(2))\n}"),
        vec!["E1008"]
    );
    assert_eq!(
        codes("fn main() {\n  print([\"a\"].sum())\n}"),
        vec!["E2005"]
    );
    ok("fn main() {\n  print([3, 1].sorted().reverse().length == 2, [\"a\"].join(\", \"))\n}");
}

#[test]
fn effects_are_inferred_for_private_functions() {
    ok("fn helper() {\n  print(1)\n}\nfn main() {\n  helper()\n}");
}

#[test]
fn declared_effects_must_cover_inferred_ones() {
    let d = diags("fn helper() {\n  print(1)\n}\nfn quiet() !{} {\n  helper()\n}");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "E3001");
    ok("fn loud() !{io} {\n  print(1)\n}");
}

#[test]
fn public_functions_must_declare_effects() {
    assert_eq!(codes("pub fn shout() {\n  print(1)\n}"), vec!["E3002"]);
    ok("pub fn add(a: Int, b: Int) -> Int {\n  return a + b\n}");
    ok("pub fn shout() !{io} {\n  print(1)\n}");
}

#[test]
fn recursive_effects_reach_a_fixed_point() {
    let d = diags(
        "fn a(n: Int) {\n  if n > 0 { b(n - 1) }\n}\nfn b(n: Int) {\n  if n == 0 { print(n) } else { a(n) }\n}\npub fn run() !{} {\n  a(3)\n}",
    );
    assert_eq!(d.iter().map(|d| d.code).collect::<Vec<_>>(), vec!["E3001"]);
}

#[test]
fn unknown_effect() {
    assert_eq!(codes("fn f() !{disk} {}"), vec!["E3003"]);
}

#[test]
fn constants_must_be_pure_and_ordered() {
    ok("let LIMIT = 10\nlet DOUBLE = LIMIT * 2\nfn main() {\n  print(DOUBLE)\n}");
    assert_eq!(codes("let A = B\nlet B = 1"), vec!["E1001"]);
    assert_eq!(
        codes("fn noisy() -> Int {\n  print(1)\n  return 1\n}\nlet X = noisy()"),
        vec!["E3004"]
    );
    assert_eq!(codes("let X = 1\nfn main() {\n  X = 2\n}"), vec!["E1003"]);
}

#[test]
fn records_report_missing_fields() {
    let d = diags("type P = { x: Int, y: Int }\nfn f(p: P) {}\nfn main() {\n  f({ x: 1 })\n}");
    assert_eq!(d[0].code, "E2001");
    assert!(
        d[0].hint
            .as_ref()
            .unwrap()
            .en
            .contains("missing fields: [`y`]")
    );
}

#[test]
fn main_is_required_when_running() {
    let (mut program, _) = elomar_parser::parse("fn helper() {}");
    let d = check(
        &mut program,
        CheckOptions {
            require_main: true,
            ..Default::default()
        },
    );
    assert_eq!(d[0].code, "E1005");
    assert_eq!(codes("fn main(x: Int) {}"), vec!["E2011"]);
}

#[test]
fn unused_values_warn() {
    let d = diags("fn main() {\n  var x = 1\n  x == 2\n}");
    assert_eq!(d[0].code, "W1002");
    assert!(d[0].hint.is_some());
}

#[test]
fn generic_type_arguments_are_not_supported_yet() {
    assert_eq!(
        codes("fn f() {}\nfn main() {\n  f<Int>()\n}"),
        vec!["E0103"]
    );
}

// ----- Phase 2: async/await and ai.infer --------------------------------

const TICKET: &str = "use std.ai\ntype Ticket = { category: String, priority: String }\n";

#[test]
fn async_functions_return_futures() {
    ok(
        "async fn two() -> Int {\n  return 2\n}\nasync fn main() {\n  let n = await two()\n  print(n + 1)\n}",
    );
    assert_eq!(
        codes("async fn two() -> Int {\n  return 2\n}\nfn main() {\n  let n: Int = two()\n}"),
        vec!["E2001"]
    );
}

#[test]
fn await_rules() {
    assert_eq!(
        codes("async fn f() {}\nfn main() {\n  await f()\n}"),
        vec!["E2013"]
    );
    assert_eq!(
        codes("async fn main() {\n  let x = await 3\n}"),
        vec!["E2014"]
    );
    assert_eq!(
        codes("async fn f() {}\nasync fn main() {\n  f()\n}"),
        vec!["W1003"]
    );
}

#[test]
fn ai_infer_is_typed() {
    let src = format!(
        "{TICKET}async fn classify(m: String) -> Result<Ticket, AIError> !{{ai}} {{\n  return await ai.infer<Ticket>(model: \"support\", instructions: \"Classify\", input: m, timeout: 10s)\n}}"
    );
    ok(&src);
    let src = format!(
        "{TICKET}async fn main() {{\n  match await ai.infer<Ticket>(\"m\", \"i\", \"x\") {{\n    Ok(t) => print(t.priority)\n    Err(e) => print(e.kind(), e.message())\n  }}\n}}"
    );
    ok(&src);
}

#[test]
fn ai_infer_rewrites_for_the_interpreter() {
    let src = format!(
        "{TICKET}async fn main() {{\n  let r = await ai.infer<Ticket>(input: \"x\", model: \"m\", instructions: \"i\")\n}}"
    );
    let (mut program, _) = elomar_parser::parse(&src);
    assert!(check(&mut program, CheckOptions::default()).is_empty());
    let Item::Fn(f) = &program.items[2] else {
        panic!()
    };
    let StmtKind::Let { value, .. } = &f.body.stmts[0].kind else {
        panic!()
    };
    let ExprKind::Await(call) = &value.kind else {
        panic!()
    };
    let ExprKind::Call {
        callee,
        type_args,
        args,
    } = &call.kind
    else {
        panic!()
    };
    let ExprKind::Field { target, .. } = &callee.kind else {
        panic!()
    };
    assert_eq!(target.kind, ExprKind::Name("std.ai".into()));
    assert!(
        matches!(type_args[0].kind, TypeExprKind::Record(_)),
        "alias expanded"
    );
    assert!(args.iter().all(|a| a.name.is_none()));
    assert_eq!(args[0].value.kind, ExprKind::Str("m".into()));
    assert_eq!(args[2].value.kind, ExprKind::Str("x".into()));
}

#[test]
fn ai_infer_errors() {
    // Not imported.
    assert_eq!(
        codes("async fn main() {\n  let r = await ai.infer<String>(\"m\", \"i\", \"x\")\n}"),
        vec!["E1001"]
    );
    // Missing type argument and missing input.
    assert_eq!(
        codes("use std.ai\nasync fn main() {\n  let r = await ai.infer(\"m\", \"i\")\n}"),
        vec!["E2008", "E2002"]
    );
    // Output and input types without a JSON form.
    assert_eq!(
        codes(
            "use std.ai\nasync fn main() {\n  let r = await ai.infer<Duration>(\"m\", \"i\", 1s)\n}"
        ),
        vec!["E2015", "E2015"]
    );
    // Unknown parameter and function.
    assert_eq!(
        codes(
            "use std.ai\nasync fn main() {\n  let r = await ai.infer<String>(\"m\", \"i\", \"x\", timout: 1s)\n}"
        ),
        vec!["E2010"]
    );
    assert_eq!(
        codes("use std.ai\nasync fn main() {\n  let r = await ai.chat(\"m\")\n}"),
        vec!["E1008"]
    );
    assert_eq!(codes("use std.web"), vec!["E1009"]);
}

#[test]
fn ai_effect_is_inferred_and_enforced() {
    let src = format!(
        "{TICKET}pub async fn ask(q: String) -> Result<String, AIError> {{\n  return await ai.infer<String>(\"m\", \"i\", q)\n}}"
    );
    assert_eq!(codes(&src), vec!["E3002"]);
    let src = format!(
        "{TICKET}async fn ask(q: String) -> Result<String, AIError> !{{io}} {{\n  return await ai.infer<String>(\"m\", \"i\", q)\n}}"
    );
    assert_eq!(codes(&src), vec!["E3001"]);
}

fn codes_with_cloud(src: &str, cloud: &[&str]) -> Vec<&'static str> {
    let (mut program, _) = elomar_parser::parse(src);
    let opts = CheckOptions {
        cloud_models: cloud.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    check(&mut program, opts).iter().map(|d| d.code).collect()
}

#[test]
fn cloud_models_add_the_net_effect() {
    let ask = |effects: &str, model: &str| {
        format!(
            "use std.ai\npub async fn ask(q: String) -> Result<String, AIError> {effects} {{\n  return await ai.infer<String>({model}, \"Answer\", q)\n}}"
        )
    };
    // Mock models need only `ai`.
    assert!(codes_with_cloud(&ask("!{ai}", "\"local\""), &["gpt"]).is_empty());
    // A literal cloud model needs `net` too.
    assert_eq!(
        codes_with_cloud(&ask("!{ai}", "\"gpt\""), &["gpt"]),
        vec!["E3001"]
    );
    assert!(codes_with_cloud(&ask("!{ai, net}", "\"gpt\""), &["gpt"]).is_empty());
    // A model chosen at run time is assumed to be a cloud model.
    assert_eq!(
        codes_with_cloud(&ask("!{ai}", "q"), &["gpt"]),
        vec!["E3001"]
    );
    assert!(codes_with_cloud(&ask("!{ai}", "q"), &[]).is_empty());
}

#[test]
fn durations() {
    ok("fn main() {\n  let t = 1s + 500ms\n  print(t > 1s, t.millis(), t.seconds())\n}");
    assert_eq!(codes("fn main() {\n  print(1s + 1)\n}"), vec!["E2005"]);
}

// ----- Phase 3: enums, Map, `?` --------------------------------------------

const SHAPES: &str = "enum Shape {\n  Circle(Float)\n  Rect(Float, Float)\n  Empty\n}\n";

#[test]
fn enums_construct_and_match() {
    ok(&format!(
        "{SHAPES}fn area(s: Shape) -> Float {{\n  return match s {{\n    Shape.Circle(r) => 3.14 * r * r\n    Shape.Rect(w, h) => w * h\n    Shape.Empty => 0\n  }}\n}}\nfn main() {{\n  print(area(Shape.Circle(1.0)), area(Shape.Empty), Shape.Rect(1, 2) == Shape.Empty)\n}}"
    ));
}

#[test]
fn enum_errors() {
    assert_eq!(
        codes(&format!(
            "{SHAPES}fn main() {{\n  let s = Shape.Circel(1.0)\n}}"
        )),
        vec!["E1010"]
    );
    assert_eq!(
        codes(&format!("{SHAPES}fn main() {{\n  let s = Shape.Circle\n}}")),
        vec!["E2002"]
    );
    assert_eq!(
        codes(&format!(
            "{SHAPES}fn main() {{\n  let s = Shape.Empty()\n}}"
        )),
        vec!["E2002"]
    );
    assert_eq!(
        codes(&format!(
            "{SHAPES}fn main() {{\n  let s = Shape.Rect(1.0)\n}}"
        )),
        vec!["E2002"]
    );
    assert_eq!(
        codes(&format!("{SHAPES}fn main() {{\n  let s: Shape = 3\n}}")),
        vec!["E2001"]
    );
    assert_eq!(
        codes(&format!("{SHAPES}fn main() {{\n  let s = Shape\n}}")),
        vec!["E2004"]
    );
    assert_eq!(codes("enum Option { A }"), vec!["E1002"]);
    assert_eq!(codes("enum E { A, A }"), vec!["E1002"]);
}

#[test]
fn enum_matches_must_be_exhaustive() {
    let d = diags(&format!(
        "{SHAPES}fn f(s: Shape) -> Int {{\n  return match s {{\n    Shape.Circle(_) => 1\n  }}\n}}"
    ));
    assert_eq!(d[0].code, "E2009");
    assert!(
        d[0].message
            .en
            .contains("`Shape.Rect(_, _)`, `Shape.Empty`"),
        "{}",
        d[0].message.en
    );
}

#[test]
fn enums_are_nominal_and_recursive() {
    assert_eq!(
        codes("enum A { X }\nenum B { X }\nfn main() {\n  let a: A = B.X\n}"),
        vec!["E2001"]
    );
    ok(
        "enum Tree {\n  Leaf\n  Node(Tree, Int, Tree)\n}\nfn sum(t: Tree) -> Int {\n  return match t {\n    Tree.Leaf => 0\n    Tree.Node(l, v, r) => sum(l) + v + sum(r)\n  }\n}",
    );
}

#[test]
fn maps() {
    ok(
        "fn main() {\n  var stock = [\"apple\": 3, \"pear\": 0]\n  stock[\"kiwi\"] = 5\n  stock[\"apple\"] += 1\n  print(stock.length, stock.get(\"x\"), stock.keys(), stock.contains_key(\"pear\"))\n  for e in stock { print(e.key, e.value) }\n  var empty: Map<Int, String> = [:]\n  empty[1] = \"one\"\n  print(empty.remove(1))\n}",
    );
    assert_eq!(
        codes("fn main() {\n  let m = [\"a\": 1]\n  m[\"b\"] = 2\n}"),
        vec!["E1003"]
    );
    assert_eq!(
        codes("fn main() {\n  let m = [\"a\": 1, \"b\": \"x\"]\n}"),
        vec!["E2001"]
    );
    assert_eq!(codes("fn main() {\n  let m = [[1]: 1]\n}"), vec!["E2018"]);
    assert_eq!(codes("fn f(m: Map<Float, Int>) {}"), vec!["E2018"]);
}

#[test]
fn try_operator() {
    ok(
        "fn parse(s: String) -> Result<Int, String> {\n  return match parse_int(s) { Some(n) => Ok(n), None => Err(\"bad\") }\n}\nfn sum(a: String, b: String) -> Result<Int, String> {\n  return Ok(parse(a)? + parse(b)?)\n}",
    );
    ok(
        "fn first_even(xs: List<Int>) -> Option<Int> {\n  let f = xs.first()?\n  return Some(f * 2)\n}",
    );
    // Not in a Result/Option function.
    assert_eq!(
        codes("fn main() {\n  let n = parse_int(\"1\")?\n}"),
        vec!["E2016"]
    );
    // Wrong kind of value.
    assert_eq!(
        codes("fn f() -> Option<Int> {\n  let n = 3?\n  return None\n}"),
        vec!["E2017"]
    );
    // Error types differ.
    assert_eq!(
        codes(
            "fn a() -> Result<Int, Int> {\n  return Err(1)\n}\nfn b() -> Result<Int, String> {\n  return Ok(a()?)\n}"
        ),
        vec!["E2001"]
    );
}

#[test]
fn ai_error_is_an_enum() {
    ok(&format!(
        "{TICKET}async fn main() {{\n  match await ai.infer<Ticket>(\"m\", \"i\", \"x\") {{\n    Ok(t) => print(t.priority)\n    Err(AIError.Timeout(msg)) => print(\"slow:\", msg)\n    Err(e) => print(e.kind(), e.message())\n  }}\n}}"
    ));
    assert_eq!(
        codes(&format!(
            "{TICKET}async fn main() {{\n  match await ai.infer<Ticket>(\"m\", \"i\", \"x\") {{\n    Ok(t) => print(t.priority)\n    Err(e) => print(e.kind)\n  }}\n}}"
        )),
        vec!["E1007"]
    );
}

#[test]
fn unit_enums_can_cross_to_ai() {
    ok(
        "use std.ai\nenum Priority { Low, Normal, High }\ntype T = { p: Priority, tags: List<String> }\nasync fn main() {\n  let r = await ai.infer<T>(\"m\", \"i\", Priority.High)\n}",
    );
    assert_eq!(
        codes(&format!(
            "use std.ai\n{SHAPES}async fn main() {{\n  let r = await ai.infer<Shape>(\"m\", \"i\", \"x\")\n}}"
        )),
        vec!["E2015"]
    );
}

// ----- std.fs and std.json -----------------------------------------------

#[test]
fn fs_functions_are_typed_and_perform_fs() {
    ok(
        "use std.fs\nfn main() {\n  match fs.read_text(\"data/a.txt\") {\n    Ok(text) => print(text.length)\n    Err(FsError.NotFound(_)) => print(\"missing\")\n    Err(e) => print(e.kind(), e.message())\n  }\n  let w: Result<Unit, FsError> = fs.write_text(\"out/b.txt\", \"hi\")\n  let names: Result<List<String>, FsError> = fs.list_dir(\"data\")\n}",
    );
    assert_eq!(
        codes(
            "use std.fs\npub fn load() -> Result<String, FsError> {\n  return fs.read_text(\"a\")\n}"
        ),
        vec!["E3002"]
    );
    ok(
        "use std.fs\npub fn load() -> Result<String, FsError> !{fs} {\n  return fs.read_text(\"a\")\n}",
    );
    assert_eq!(
        codes("fn main() {\n  let t = fs.read_text(\"a\")\n}"),
        vec!["E1001"]
    );
    assert_eq!(
        codes("use std.fs\nfn main() {\n  let t = fs.read(\"a\")\n}"),
        vec!["E1008"]
    );
    assert_eq!(
        codes("use std.fs\nfn main() {\n  let t = fs.write_text(\"a\")\n}"),
        vec!["E2002"]
    );
}

#[test]
fn json_parse_and_stringify() {
    ok(
        "use std.json\ntype Student = { name: String, grades: List<Int>, extra: Map<String, Bool> }\nfn main() {\n  match json.parse<Student>(\"\\{}\") {\n    Ok(s) => print(json.stringify(s), json.pretty(s.grades))\n    Err(JsonError.InvalidJson(m)) => print(m)\n    Err(e) => print(e.kind())\n  }\n}",
    );
    // json is pure: no effect needed on a public function.
    ok("use std.json\npub fn encode(xs: List<Int>) -> String {\n  return json.stringify(xs)\n}");
    assert_eq!(
        codes("use std.json\nfn main() {\n  let r = json.parse(\"1\")\n}"),
        vec!["E2008"]
    );
    assert_eq!(
        codes("use std.json\nfn main() {\n  let r = json.parse<Duration>(\"1\")\n}"),
        vec!["E2015"]
    );
    assert_eq!(
        codes("use std.json\nfn main() {\n  print(json.stringify(1s))\n}"),
        vec!["E2015"]
    );
    assert_eq!(
        codes("use std.json\nfn main() {\n  print(json.stringify<Int>(1))\n}"),
        vec!["E0103"]
    );
    // Maps are JSON for std.json but not for models.
    assert_eq!(
        codes(
            "use std.ai\nasync fn main() {\n  let r = await ai.infer<Map<String, Int>>(\"m\", \"i\", \"x\")\n}"
        ),
        vec!["E2015"]
    );
}

#[test]
fn assert_eq_needs_matching_types() {
    ok(
        "fn main() {\n  assert_eq(1 + 1, 2)\n  assert_eq(0.5 * 2, 1)\n  assert_eq([\"a\"], [\"a\"])\n}",
    );
    assert_eq!(
        codes("fn main() {\n  assert_eq(1, \"1\")\n}"),
        vec!["E2001"]
    );
}

#[test]
fn sets() {
    ok(
        "fn main() {\n  var tags = set([\"a\", \"b\"])\n  tags.add(\"c\")\n  let removed: Bool = tags.remove(\"a\")\n  let other = set([\"b\"])\n  print(tags.length, tags.contains(\"b\"), tags.union(other), other.is_subset(tags), tags.to_list())\n  for t in tags { print(t.upper()) }\n  let empty: Set<Int> = set([])\n}",
    );
    assert_eq!(codes("fn main() {\n  let s = set([1.5])\n}"), vec!["E2018"]);
    assert_eq!(
        codes("fn main() {\n  let s = set([1])\n  s.add(2)\n}"),
        vec!["E1003"]
    );
    assert_eq!(
        codes("fn main() {\n  let s = set([1])\n  print(s.contains(\"x\"))\n}"),
        vec!["E2001"]
    );
    assert_eq!(codes("fn f(s: Set<Float>) {}"), vec!["E2018"]);
}

// ----- v0.3: generic functions --------------------------------------------

#[test]
fn generic_functions() {
    ok(
        "fn first<T>(xs: List<T>) -> Option<T> {\n  if xs.is_empty() { return None }\n  return Some(xs[0])\n}\nfn pair<A, B>(a: A, b: B) -> { left: A, right: B } {\n  return { left: a, right: b }\n}\nfn count<T>(xs: List<T>, x: T) -> Int {\n  var n = 0\n  for y in xs { if y == x { n += 1 } }\n  return n\n}\nfn main() {\n  let a: Option<Int> = first([1, 2])\n  let b: Option<String> = first([\"x\"])\n  let p = pair(1, \"one\")\n  let s: String = p.right\n  print(count([1, 2, 1], 1), first<String>([]))\n}",
    );
}

#[test]
fn generic_misuse() {
    // A type parameter is not a concrete type.
    assert_eq!(
        codes("fn bad<T>(x: T) -> Int {\n  return x\n}"),
        vec!["E2001"]
    );
    // No comparisons or arithmetic on an unknown type.
    assert_eq!(
        codes("fn less<T>(a: T, b: T) -> Bool {\n  return a < b\n}"),
        vec!["E2005"]
    );
    // Both arguments must agree on T.
    assert_eq!(
        codes("fn same<T>(a: T, b: T) {}\nfn main() {\n  same(1, \"x\")\n}"),
        vec!["E2001"]
    );
    // Wrong number of explicit type arguments; non-generic functions take none.
    assert_eq!(
        codes("fn id<T>(x: T) -> T {\n  return x\n}\nfn main() {\n  print(id<Int, Int>(1))\n}"),
        vec!["E2002"]
    );
    assert_eq!(
        codes("fn f() {}\nfn main() {\n  f<Int>()\n}"),
        vec!["E0103"]
    );
    assert_eq!(codes("fn f<T, T>(x: T) {}"), vec!["E1002"]);
}

// ----- v0.3: closures and function values -----------------------------------

#[test]
fn closures_and_function_values() {
    ok(
        "fn double(x: Int) -> Int {\n  return x * 2\n}\nfn apply(f: fn(Int) -> Int, x: Int) -> Int {\n  return f(x)\n}\nfn adder(n: Int) -> fn(Int) -> Int {\n  return |x| x + n\n}\nfn main() {\n  let xs = [1, 2, 3]\n  let a: List<Int> = xs.map(double)\n  let b: List<String> = xs.map(|x| to_string(x))\n  let c: Int = xs.fold(0, |acc, x| acc + x)\n  let d: Option<Int> = xs.find(|x| x > 1)\n  let e = xs.filter(|x| x % 2 == 1).sort_by(|x| -x)\n  let f = adder(2)\n  print(apply(f, 1), apply(|x: Int| { return x - 1 }, 3), xs.any(|x| x == 2), xs.count(|x| x > 1))\n}",
    );
}

#[test]
fn closure_errors() {
    // Captured variables are copies.
    assert_eq!(
        codes("fn main() {\n  var n = 0\n  let f = || { n += 1 }\n}"),
        vec!["E1013"]
    );
    // A closure's own variables can change.
    ok(
        "fn main() {\n  let f = |x: Int| {\n    var y = x\n    y += 1\n    return y\n  }\n  print(f(1))\n}",
    );
    // Built-ins are values, except the ones that work on futures.
    ok(
        "fn main() {\n  let p = print\n  p(1)\n  let xs: List<String> = [1, 2].map(to_string)\n  let f: fn(Float, Float) -> Float = max\n  print(xs, f(1.5, 2.0), [-1, 2].map(abs), [\"1\", \"x\"].map(parse_int))\n}",
    );
    assert_eq!(codes("async fn main() {\n  let a = all\n}"), vec!["E0103"]);
    assert_eq!(codes("pub fn run() {\n  let p = print\n}"), vec!["E3002"]);
    // Wrong arity and argument types.
    assert_eq!(
        codes("fn main() {\n  let f = |a: Int, b: Int| a + b\n  print(f(1))\n}"),
        vec!["E2002"]
    );
    assert_eq!(
        codes("fn main() {\n  [1].map(|x| x + \"a\")\n}"),
        vec!["E2005"]
    );
    assert_eq!(
        codes("fn main() {\n  [1].sort_by(|x| [x])\n}"),
        vec!["E2005"]
    );
    assert_eq!(codes("fn main() {\n  let x = 1\n  x(2)\n}"), vec!["E2004"]);
}

#[test]
fn closure_effects_belong_to_their_creator() {
    assert_eq!(
        codes("pub fn shout(xs: List<Int>) {\n  xs.map(|x| { print(x)\n    return x })\n}"),
        vec!["E3002"]
    );
    ok("pub fn shout(xs: List<Int>) !{io} {\n  xs.map(|x| { print(x)\n    return x })\n}");
}

// ----- std.http ----------------------------------------------------------------

#[test]
fn http_module() {
    ok(
        "use std.http\nfn main() {\n  match http.get(\"https://example.com\") {\n    Ok(r) => print(r.status, r.body.length)\n    Err(HttpError.PolicyDenied(m)) => print(m)\n    Err(e) => print(e.kind(), e.message())\n  }\n  let r: Result<{ status: Int, body: String }, HttpError> = http.post(\"https://example.com\", \"\\{}\", \"application/json\")\n}",
    );
    ok(
        "use std.http\npub fn fetch(url: String) -> Bool !{net} {\n  return http.get(url).is_ok()\n}",
    );
    // The `net` effect must be declared on public functions.
    assert_eq!(
        codes(
            "use std.http\npub fn fetch(url: String) -> Bool {\n  return http.get(url).is_ok()\n}"
        ),
        vec!["E3002"]
    );
    assert_eq!(
        codes("use std.http\nfn main() {\n  http.put(\"x\")\n}"),
        vec!["E1008"]
    );
    assert_eq!(
        codes("use std.http\nfn main() {\n  http.post(\"x\")\n}"),
        vec!["E2002"]
    );
    assert_eq!(codes("fn main() {\n  http.get(\"x\")\n}"), vec!["E1001"]);
}

// ----- v0.3: generic types and enums --------------------------------------------

#[test]
fn generic_types_and_enums() {
    ok(
        "type Pair<A, B> = { left: A, right: B }\ntype Named<T> = { name: String, value: T }\nenum Tree<T> {\n  Leaf\n  Node(Tree<T>, T, Tree<T>)\n}\nfn swap<A, B>(p: Pair<A, B>) -> Pair<B, A> {\n  return { left: p.right, right: p.left }\n}\nfn size<T>(t: Tree<T>) -> Int {\n  return match t {\n    Tree.Leaf => 0\n    Tree.Node(l, _, r) => size(l) + 1 + size(r)\n  }\n}\nfn main() {\n  let p: Pair<Int, String> = { left: 1, right: \"a\" }\n  let q: Pair<String, Int> = swap(p)\n  let n: Named<List<Int>> = { name: \"xs\", value: [1] }\n  let t = Tree.Node(Tree.Leaf, \"x\", Tree.Leaf)\n  let u: Tree<Int> = Tree.Leaf\n  print(size(t), size(u), n.value.length)\n}",
    );
}

#[test]
fn generic_type_errors() {
    assert_eq!(
        codes(
            "type Pair<A, B> = { left: A, right: B }\nfn main() {\n  let a: Pair<Int> = { left: 1 }\n}"
        ),
        vec!["E1006"]
    );
    assert_eq!(
        codes("enum Box<T> {\n  Full(T)\n  Empty\n}\nfn main() {\n  let b: Box = Box.Empty\n}"),
        vec!["E1006"]
    );
    assert_eq!(
        codes(
            "enum Box<T> {\n  Full(T)\n  Empty\n}\nfn main() {\n  let c: Box<Int> = Box.Full(\"x\")\n}"
        ),
        vec!["E2001"]
    );
    // The payload type follows the scrutinee's type argument.
    assert_eq!(
        codes(
            "enum Box<T> {\n  Full(T)\n  Empty\n}\nfn f(c: Box<Int>) {\n  match c {\n    Box.Full(s) => print(s.upper())\n    Box.Empty => print(\"\")\n  }\n}"
        ),
        vec!["E1008"]
    );
    assert_eq!(codes("enum E<T, T> {\n  A(T)\n}"), vec!["E1002"]);
}

// ----- v0.3: traits and impl -------------------------------------------------

const DESCRIBE: &str = "trait Describe {\n  fn describe(self) -> String\n}\ntype Student = { name: String }\nimpl Describe for Student {\n  fn describe(self) -> String {\n    return self.name\n  }\n}\nimpl Describe for Int {\n  fn describe(self) -> String {\n    return to_string(self)\n  }\n}\nfn show<T: Describe>(x: T) -> String {\n  return x.describe()\n}\n";

#[test]
fn traits_and_methods() {
    ok(&format!(
        "{DESCRIBE}impl Student {{\n  fn shout(self, times: Int) -> String {{\n    return self.name.upper().repeat(times)\n  }}\n}}\nfn twice<U: Describe>(x: U) -> String {{\n  return show(x) + show(x)\n}}\nfn main() {{\n  let s: Student = {{ name: \"Lina\" }}\n  let a: String = s.describe()\n  print(a, s.shout(2), show(s), show(3), twice(4), [1, 2].map(|x| show(x)))\n}}"
    ));
}

#[test]
fn trait_errors() {
    // Missing, mismatched and extra methods.
    assert_eq!(
        codes(
            "trait T {\n  fn a(self) -> Int\n  fn b(self)\n}\nimpl T for Int {\n  fn a(self) -> String {\n    return \"\"\n  }\n  fn c(self) {}\n}"
        ),
        vec!["E2001", "E1014", "E1008"]
    );
    // A type that does not implement the trait.
    assert_eq!(
        codes(&format!("{DESCRIBE}fn main() {{\n  print(show(true))\n}}")),
        vec!["E2019"]
    );
    // A type parameter without the bound.
    assert_eq!(
        codes(&format!(
            "{DESCRIBE}fn wrap<U>(x: U) -> String {{\n  return show(x)\n}}"
        )),
        vec!["E2019"]
    );
    // Without a bound, a type parameter has no methods.
    assert_eq!(
        codes(&format!(
            "{DESCRIBE}fn wrap<U>(x: U) -> String {{\n  return x.describe()\n}}"
        )),
        vec!["E1008"]
    );
    assert_eq!(codes("fn f<T: Nope>(x: T) {}"), vec!["E1006"]);
    assert_eq!(
        codes(&format!(
            "{DESCRIBE}impl Describe for Int {{\n  fn describe(self) -> String {{\n    return \"\"\n  }}\n}}"
        )),
        vec!["E1002"]
    );
    assert_eq!(
        codes(&format!("{DESCRIBE}fn main() {{\n  let f = show\n}}")),
        vec!["E2020"]
    );
    // A bounded function is a value once its type is known.
    ok(&format!(
        "{DESCRIBE}fn main() {{\n  let f: fn(Int) -> String = show\n  print([1].map(show), f(2))\n}}"
    ));
}

#[test]
fn method_effects_reach_the_caller() {
    let src = "trait Say {\n  fn say(self)\n}\nimpl Say for Int {\n  fn say(self) {\n    print(self)\n  }\n}\nfn speak<T: Say>(x: T) {\n  x.say()\n}\n";
    assert_eq!(
        codes(&format!("{src}pub fn run() {{\n  speak(1)\n}}")),
        vec!["E3002"]
    );
    ok(&format!(
        "{src}pub fn run() !{{io}} {{\n  speak(1)\n  2.say()\n}}"
    ));
}

// ----- std.db ----------------------------------------------------------------------

#[test]
fn db_module() {
    ok(
        "use std.db\ntype Row = { id: Int, name: String, note: Option<String> }\nfn main() {\n  match db.open(\":memory:\") {\n    Ok(conn) => {\n      let n: Result<Int, DbError> = db.execute(conn, \"DELETE FROM t WHERE id = :id\", { id: 1 })\n      let rows: Result<List<Row>, DbError> = db.query<Row>(conn, \"SELECT * FROM t\")\n      let one: Result<Option<Int>, DbError> = db.query_one<Int>(conn, \"SELECT 1\")\n    }\n    Err(DbError.PolicyDenied(m)) => print(m)\n    Err(e) => print(e.kind(), e.message())\n  }\n}",
    );
    ok(
        "use std.db\npub fn count(conn: Db) -> Int !{db} {\n  return match db.query_one<Int>(conn, \"SELECT 1\") {\n    Ok(Some(n)) => n\n    _ => 0\n  }\n}",
    );
    assert_eq!(
        codes(
            "use std.db\npub fn count(conn: Db) -> Bool {\n  return db.execute(conn, \"x\").is_ok()\n}"
        ),
        vec!["E3002"]
    );
    assert_eq!(
        codes("use std.db\nfn f(conn: Db) {\n  print(db.query(conn, \"x\"))\n}"),
        vec!["E2008"]
    );
    assert_eq!(
        codes("use std.db\nfn f(conn: Db) {\n  print(db.query<List<Int>>(conn, \"x\"))\n}"),
        vec!["E2021"]
    );
    assert_eq!(
        codes("use std.db\nfn f(conn: Db) {\n  print(db.execute(conn, \"x\", [1]))\n}"),
        vec!["E2021"]
    );
    assert_eq!(
        codes("use std.db\nfn f() {\n  print(db.execute(\"x\", \"y\"))\n}"),
        vec!["E2001"]
    );
}

// ----- ai.run with tools ----------------------------------------------------------

const TOOLS: &str = "use std.ai
fn lookup(name: String) -> Option<Int> {
  return None
}
fn log_it(msg: String) {
  print(msg)
}
";

#[test]
fn ai_run_tools() {
    ok(&format!(
        "{TOOLS}async fn main() {{\n  let r: Result<String, AIError> = await ai.run<String>(model: \"m\", instructions: \"i\", input: \"q\", tools: [lookup, log_it], max_steps: 3, timeout: 5s)\n  let s = await ai.run<{{ n: Int }}>(\"m\", \"i\", 1, [])\n}}"
    ));
    // The tools' effects belong to the caller of `ai.run`.
    assert_eq!(
        codes(&format!(
            "{TOOLS}pub async fn ask() -> Result<String, AIError> !{{ai}} {{\n  return await ai.run<String>(\"m\", \"i\", \"q\", [log_it])\n}}"
        )),
        vec!["E3001"]
    );
    assert_eq!(
        codes(&format!(
            "{TOOLS}async fn main() {{\n  let f = |x: Int| x\n  let r = await ai.run<String>(\"m\", \"i\", \"q\", [f])\n}}"
        )),
        vec!["E2022"]
    );
    assert_eq!(
        codes(&format!(
            "{TOOLS}async fn main() {{\n  let r = await ai.run<String>(\"m\", \"i\", \"q\", lookup)\n}}"
        )),
        vec!["E2022"]
    );
    assert_eq!(
        codes(&format!(
            "{TOOLS}async fn main() {{\n  let r = await ai.run<String>(\"m\", \"i\", \"q\")\n}}"
        )),
        vec!["E2002"]
    );
    assert_eq!(
        codes(
            "use std.ai\nfn f(m: Map<String, Int>) -> Int {\n  return 1\n}\nasync fn main() {\n  let r = await ai.run<String>(\"m\", \"i\", \"q\", [f])\n}"
        ),
        vec!["E2015"]
    );
}

// ----- std.py --------------------------------------------------------------------------

#[test]
fn py_module() {
    ok(
        "use std.py\ntype S = { mean: Float, tags: Map<String, Int> }\nfn main() {\n  let a: Result<S, PyError> = py.call<S>(\"m\", \"f\", { xs: [1, 2] })\n  let b = py.call<Float>(\"math\", \"sqrt\", 2.0, 5s)\n  match b {\n    Ok(x) => print(x)\n    Err(PyError.Exception(m)) => print(m)\n    Err(e) => print(e.kind())\n  }\n}",
    );
    assert_eq!(
        codes(
            "use std.py\npub fn f() -> Bool {\n  return py.call<Int>(\"m\", \"f\", 1).is_ok()\n}"
        ),
        vec!["E3002"]
    );
    ok("use std.py\npub fn f() -> Bool !{py} {\n  return py.call<Int>(\"m\", \"f\", 1).is_ok()\n}");
    assert_eq!(
        codes("use std.py\nfn f() {\n  print(py.call(\"m\", \"f\", 1))\n}"),
        vec!["E2008"]
    );
    assert_eq!(
        codes("use std.py\nfn f() {\n  print(py.call<Duration>(\"m\", \"f\", 1))\n}"),
        vec!["E2015"]
    );
    assert_eq!(
        codes("use std.py\nfn f() {\n  print(py.run(\"x\"))\n}"),
        vec!["E1008"]
    );
}

// ----- std.mcp and agent.run -----------------------------------------------------------

#[test]
fn mcp_module() {
    ok(
        "use std.mcp\nfn main() {\n  let tools: Result<List<{ name: String, description: String }>, McpError> = mcp.tools(\"notes\")\n  match mcp.call(\"notes\", \"add\", { text: \"hi\" }) {\n    Ok(t) => print(t.upper())\n    Err(McpError.ToolFailed(m)) => print(m)\n    Err(e) => print(e.kind(), e.message())\n  }\n  let none = mcp.call(\"notes\", \"count\", {})\n}",
    );
    assert_eq!(
        codes("use std.mcp\npub fn f() -> Bool {\n  return mcp.tools(\"x\").is_ok()\n}"),
        vec!["E3002"]
    );
    ok("use std.mcp\npub fn f() -> Bool !{mcp} {\n  return mcp.tools(\"x\").is_ok()\n}");
    assert_eq!(
        codes("use std.mcp\nfn f() {\n  print(mcp.call(\"s\", \"t\", 1))\n}"),
        vec!["E2001"]
    );
    assert_eq!(
        codes("use std.mcp\nfn f() {\n  print(mcp.connect(\"s\"))\n}"),
        vec!["E1008"]
    );
    assert_eq!(
        codes("fn f() {\n  print(mcp.tools(\"s\"))\n}"),
        vec!["E1001"]
    );
}

const AGENT: &str = "use std.agent
fn lookup(name: String) -> Option<Int> {
  return None
}
";

#[test]
fn agent_run() {
    ok(&format!(
        "{AGENT}async fn main() {{\n  let r: Result<String, AIError> = await agent.run<String>(model: \"m\", instructions: \"i\", input: \"q\", tools: [lookup], mcp: [\"notes\"], require_approval: [\"lookup\", \"notes.add\"], approve: |c| c.tool != \"x\", on_step: |s| print(s.tool, s.arguments, s.result, s.approved), max_steps: 4, timeout: 5s)\n  let s = await agent.run<String>(\"m\", \"i\", \"q\")\n}}"
    ));
    // MCP servers are the `mcp` effect; an empty list is not.
    assert_eq!(
        codes(&format!(
            "{AGENT}pub async fn ask() -> Result<String, AIError> !{{ai}} {{\n  return await agent.run<String>(model: \"m\", instructions: \"i\", input: \"q\", mcp: [\"notes\"])\n}}"
        )),
        vec!["E3001"]
    );
    ok(&format!(
        "{AGENT}pub async fn ask() -> Result<String, AIError> !{{ai}} {{\n  return await agent.run<String>(model: \"m\", instructions: \"i\", input: \"q\", mcp: [])\n}}"
    ));
    // Approval names must be tools of the call.
    assert_eq!(
        codes(&format!(
            "{AGENT}async fn main() {{\n  let r = await agent.run<String>(model: \"m\", instructions: \"i\", input: \"q\", tools: [lookup], mcp: [\"notes\"], require_approval: [\"lookup2\", \"other.add\", \"notes.add\"])\n}}"
        )),
        vec!["E2022", "E2022"]
    );
    // `approve` must return Bool.
    assert_eq!(
        codes(&format!(
            "{AGENT}async fn main() {{\n  let r = await agent.run<String>(model: \"m\", instructions: \"i\", input: \"q\", approve: |c| c.tool)\n}}"
        )),
        vec!["E2001"]
    );
    assert_eq!(
        codes(&format!(
            "{AGENT}async fn main() {{\n  let r = await agent.run<String>(model: \"m\", instructions: \"i\", input: \"q\", servers: [])\n}}"
        )),
        vec!["E2010"]
    );
    assert_eq!(
        codes(&format!(
            "{AGENT}async fn main() {{\n  let r = await agent.start(\"m\")\n}}"
        )),
        vec!["E1008"]
    );
}

// ----- editor information (elomar lsp) ------------------------------------------------

#[test]
fn analyze_records_hovers_and_definitions() {
    let src = "trait Describe {\n  fn describe(self) -> String\n}\ntype P = { x: Int }\nimpl Describe for P {\n  fn describe(self) -> String {\n    return to_string(self.x)\n  }\n}\nfn twice(n: Int) -> Int {\n  return n * 2\n}\nfn main() {\n  var count = twice(3)\n  let p: P = { x: count }\n  print(p.describe(), p.x)\n}";
    let (mut program, diags) = elomar_parser::parse(src);
    assert!(diags.is_empty());
    let (diags, info) = crate::analyze(&mut program, CheckOptions::default());
    assert!(diags.is_empty(), "{diags:#?}");
    let hover_at = |needle: &str, nth: usize| -> String {
        let at = src.match_indices(needle).nth(nth).unwrap().0 as u32;
        info.hovers
            .iter()
            .filter(|h| h.span.start <= at && at < h.span.end)
            .min_by_key(|h| h.span.end - h.span.start)
            .map(|h| h.text.clone())
            .unwrap_or_default()
    };
    assert_eq!(hover_at("twice(3)", 0), "fn twice(n: Int) -> Int");
    assert_eq!(hover_at("count", 1), "var count: Int");
    assert_eq!(hover_at("describe()", 0), "fn P::describe(self) -> String");
    assert_eq!(hover_at("P = ", 0), "type P = { x: Int }");
    assert_eq!(hover_at("main", 0), "fn main() !{io}");
    // `p.` offers the field and the method.
    let p_use = src.rfind("p.x").unwrap() as u32;
    let members = &info
        .hovers
        .iter()
        .find(|h| h.span.start == p_use)
        .unwrap()
        .members;
    assert!(members.contains(&"x".to_string()) && members.contains(&"describe".to_string()));
    // `twice(3)` leads to `fn twice`.
    let use_at = src.find("twice(3)").unwrap() as u32;
    let def_at = src.find("twice(n").unwrap() as u32;
    assert!(
        info.definitions
            .iter()
            .any(|(u, d)| u.start == use_at && d.start == def_at)
    );
}

#[test]
fn interpolation_is_checked() {
    ok("fn main() {\n  let n = 3\n  let s: String = \"n = {n * 2}\"\n  print(s)\n}");
    assert_eq!(
        codes("fn main() {\n  print(\"{missing}\")\n}"),
        vec!["E1001"]
    );
    let d = diags("fn main() {\n  let n = 1\n  print(\"x {n + true}\")\n}");
    let src = "fn main() {\n  let n = 1\n  print(\"x {n + true}\")\n}";
    let span = d[0].label.span;
    assert_eq!(&src[span.start as usize..span.end as usize], "n + true");
}

#[test]
fn literal_regex_is_checked() {
    assert_eq!(
        codes("use std.regex\nfn main() {\n  print(regex.is_match(\"(a\", \"a\"))\n}"),
        vec!["E2023"]
    );
    ok("use std.regex\nfn main() {\n  print(regex.is_match(\"(a)\", \"a\"))\n}");
}

#[test]
fn nested_patterns_are_exhaustive() {
    let r = "fn f(r: Result<Option<Int>, String>) -> Int {\n  return match r {\n    Ok(Some(n)) => n\n    Ok(None) => 0\n    Err(_) => -1\n  }\n}";
    ok(r);
    let d = diags(&r.replace("    Ok(None) => 0\n", ""));
    assert_eq!(d[0].code, "E2009");
    assert!(
        d[0].message.en.contains("`Ok(None)`"),
        "{}",
        d[0].message.en
    );
    let e = "enum Shape {\n  Circle(Float)\n  Rect(Float, Float)\n}\nfn f(s: Option<Shape>) -> Float {\n  return match s {\n    Some(Shape.Circle(r)) => r\n    Some(Shape.Rect(w, _)) => w\n    None => 0.0\n  }\n}";
    ok(e);
    let d = diags(&e.replace("    Some(Shape.Rect(w, _)) => w\n", ""));
    assert!(
        d[0].message.en.contains("`Some(Shape.Rect(_, _))`"),
        "{}",
        d[0].message.en
    );
    ok(
        "fn f(x: Option<Bool>) -> Int {\n  return match x {\n    Some(true) => 1\n    Some(false) => 2\n    None => 3\n  }\n}",
    );
}

#[test]
fn or_and_range_pattern_rules() {
    ok("fn f(b: Bool) -> Int {\n  return match b {\n    true | false => 1\n  }\n}");
    assert_eq!(
        codes("fn f(x: Option<Int>) -> Int {\n  return match x {\n    Some(n) | None => 1\n  }\n}"),
        vec!["E2024"]
    );
    assert_eq!(
        codes("fn f(x: Int) -> Int {\n  return match x {\n    5..=1 => 1\n    _ => 2\n  }\n}"),
        vec!["E2025"]
    );
    assert_eq!(
        codes("fn f(x: String) -> Int {\n  return match x {\n    1..3 => 1\n    _ => 2\n  }\n}"),
        vec!["E2001"]
    );
}

#[test]
fn tuple_rules() {
    ok(
        "fn f() -> (Int, String) {\n  return (1, \"a\")\n}\nfn main() {\n  let (n, s) = f()\n  print(n + 1, s.upper())\n}",
    );
    assert_eq!(
        codes("fn main() {\n  let t = (1, 2)\n  print(t.2)\n}"),
        vec!["E2027"]
    );
    assert_eq!(
        codes("fn main() {\n  let (a, b) = 5\n  print(a)\n}"),
        vec!["E2026"]
    );
    assert_eq!(
        codes("fn main() {\n  let (a, b) = (1, 2, 3)\n  print(a)\n}"),
        vec!["E2026"]
    );
    assert_eq!(
        codes("fn f(t: (Int, Bool)) -> Int {\n  return match t {\n    (_, true) => 1\n  }\n}"),
        vec!["E2009"]
    );
    ok(
        "fn f(t: (Int, Bool)) -> Int {\n  return match t {\n    (_, true) => 1\n    (n, false) => n\n  }\n}",
    );
    let d = diags("fn main() {\n  let t: (Int, String) = (1, 2)\n}");
    assert_eq!(d[0].code, "E2001");
}

#[test]
fn default_methods_still_require_the_rest() {
    let src = "trait Greet {\n  fn name(self) -> String\n  fn greet(self) -> String {\n    return self.name()\n  }\n}\ntype P = { n: String }\nimpl Greet for P {\n}\n";
    // The missing method is reported first (the default body that calls it
    // may add a follow-up error).
    assert_eq!(codes(src)[0], "E1014");
}

#[test]
fn generic_impl_types_flow() {
    let src = "type Stack<T> = { items: List<T> }\nimpl<T> Stack<T> {\n  fn push(self, x: T) -> Stack<T> {\n    return { items: self.items + [x] }\n  }\n}\nfn main() {\n  let s: Stack<Int> = { items: [] }\n  print(s.push(\"no\"))\n}";
    assert_eq!(codes(src), vec!["E2001"]);
}

#[test]
fn unknown_type_in_ai_infer_is_an_error_not_a_crash() {
    // Found by fuzz testing.
    let codes = codes(
        "use std.ai\nasync fn main() {\n    let r = await ai.infer<selfResult<Int, String>>(\"mock\", \"Count\", \"x\")\n}",
    );
    assert!(!codes.is_empty(), "{codes:?}");
}

#[test]
fn every_builtin_method_has_one_arabic_name() {
    use elomar_ast::arabic::{METHOD_NAMES, method_arabic, method_names};
    for (ty, methods) in crate::builtins::TYPE_METHODS {
        for m in methods {
            let ar = method_arabic(m).unwrap_or_else(|| panic!("{ty}.{m} has no Arabic name"));
            // No other method of the same type has that Arabic name.
            let same: Vec<&str> = method_names(ar).filter(|en| methods.contains(en)).collect();
            assert_eq!(same, vec![*m], "{ty}: `{ar}` is ambiguous");
        }
    }
    for (_, en) in METHOD_NAMES {
        assert_eq!(
            METHOD_NAMES.iter().filter(|(_, e)| e == en).count(),
            1,
            "{en}"
        );
    }
}

#[test]
fn arabic_method_names() {
    ok(
        "fn main() {\n  var xs = [3, 1]\n  xs.أضف(2)\n  print(xs.طول, xs.مرتبة(), \"أ\".يحتوي(\"أ\"))\n}",
    );
    // On a set, `أضف` is `add`, not `push`.
    ok("fn main() {\n  var s = set([1])\n  s.اضف(2)\n  print(s.طول)\n}");
    // A record's own field of that name wins.
    ok(
        "type R = { طول: String }\nfn main() {\n  let r: R = { طول: \"x\" }\n  print(r.طول.upper())\n}",
    );
    // A list has no `مفاتيح`; mutating needs `var`.
    assert_eq!(
        codes("fn main() {\n  print([1].المفاتيح())\n}"),
        vec!["E1008"]
    );
    assert_eq!(
        codes("fn main() {\n  let xs = [1]\n  xs.أضف(2)\n}"),
        vec!["E1003"]
    );
}
