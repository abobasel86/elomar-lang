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
        "{TICKET}async fn main() {{\n  match await ai.infer<Ticket>(\"m\", \"i\", \"x\") {{\n    Ok(t) => print(t.priority)\n    Err(e) => print(e.kind, e.message)\n  }}\n}}"
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
