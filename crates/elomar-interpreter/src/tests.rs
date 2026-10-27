use super::*;

fn run_src(src: &str) -> Result<String, RuntimeError> {
    let (mut program, diags) = elomar_parser::parse(src);
    assert!(diags.is_empty(), "parse errors: {diags:#?}");
    let diags = elomar_types::check(
        &mut program,
        elomar_types::CheckOptions {
            require_main: true,
            ..Default::default()
        },
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "check errors: {errors:#?}");
    with_large_stack(|| {
        let mut host = BufferHost::default();
        run(&program, &mut host)?;
        Ok(host.output)
    })
}

fn out(src: &str) -> String {
    run_src(src).unwrap_or_else(|e| panic!("runtime error: {e:#?}"))
}

fn err_code(src: &str) -> &'static str {
    run_src(src).expect_err("expected a runtime error").code
}

#[test]
fn hello_and_arithmetic() {
    assert_eq!(out("fn main() {\n  print(\"Hello\")\n}"), "Hello\n");
    assert_eq!(
        out("fn main() {\n  print(7 / 2, 7 div 2, -7 % 2, 1 + 2.5, 2 * 3)\n}"),
        "3.5 3 -1 3.5 6\n"
    );
    assert_eq!(out("fn main() {\n  print(10 / 2)\n}"), "5.0\n");
}

#[test]
fn functions_and_recursion() {
    assert_eq!(
        out(
            "fn fact(n: Int) -> Int {\n  if n <= 1 { return 1 }\n  return n * fact(n - 1)\n}\nfn main() {\n  print(fact(20))\n}"
        ),
        "2432902008176640000\n"
    );
    assert_eq!(
        out(
            "fn greet(name: String, times: Int) {\n  for i in 1..=times { print(i, name) }\n}\nfn main() {\n  greet(times: 2, name: \"Lina\")\n}"
        ),
        "1 Lina\n2 Lina\n"
    );
}

#[test]
fn loops_break_continue() {
    assert_eq!(
        out(
            "fn main() {\n  var total = 0\n  for i in 0..10 {\n    if i == 7 { break }\n    if i % 2 == 0 { continue }\n    total += i\n  }\n  print(total)\n}"
        ),
        "9\n"
    );
    assert_eq!(
        out(
            "fn main() {\n  var n = 0\n  while true {\n    n += 1\n    if n == 3 { break }\n  }\n  print(n)\n}"
        ),
        "3\n"
    );
}

#[test]
fn value_semantics() {
    assert_eq!(
        out(
            "fn main() {\n  var a = [1, 2, 3]\n  var b = a\n  b.push(4)\n  b[0] = 9\n  print(a, b)\n}"
        ),
        "[1, 2, 3] [9, 2, 3, 4]\n"
    );
    assert_eq!(
        out(
            "fn bump(xs: List<Int>) -> List<Int> {\n  var ys = xs\n  ys.push(0)\n  return ys\n}\nfn main() {\n  let xs = [1]\n  print(bump(xs), xs)\n}"
        ),
        "[1, 0] [1]\n"
    );
    assert_eq!(
        out(
            "fn main() {\n  var r = { name: \"Lina\", tags: [\"a\"] }\n  let copy = r\n  r.tags.push(\"b\")\n  r.name = \"Sara\"\n  print(r)\n  print(copy)\n}"
        ),
        "{ name: \"Sara\", tags: [\"a\", \"b\"] }\n{ name: \"Lina\", tags: [\"a\"] }\n"
    );
}

#[test]
fn options_results_match() {
    let src = "fn safe(a: Int, b: Int) -> Result<Int, String> {\n  if b == 0 { return Err(\"zero\") }\n  return Ok(a div b)\n}\nfn main() {\n  for b in [2, 0] {\n    match safe(10, b) {\n      Ok(v) => print(\"ok\", v)\n      Err(e) => print(\"err\", e)\n    }\n  }\n  let n = match parse_int(\" 41 \") { Some(v) => v + 1, None => 0 }\n  print(n, parse_int(\"x\"), safe(1, 0))\n  print(Some(3).unwrap_or(0), None.unwrap_or(5))\n}";
    assert_eq!(out(src), "ok 5\nerr zero\n42 None Err(\"zero\")\n3 5\n");
}

#[test]
fn match_on_literals() {
    assert_eq!(
        out(
            "fn name(n: Int) -> String {\n  return match n { 1 => \"one\", 2 => \"two\", _ => \"many\" }\n}\nfn main() {\n  print(name(1), name(2), name(9))\n}"
        ),
        "one two many\n"
    );
}

#[test]
fn strings_and_unicode() {
    assert_eq!(
        out(
            "fn main() {\n  let s = \"مرحبا\"\n  print(s.length, s.chars()[0], \"a,b\".split(\",\"), \"ab\".upper(), \"x\".repeat(3))\n  for c in \"أب\" { print(c) }\n}"
        ),
        "5 م [\"a\", \"b\"] AB xxx\nأ\nب\n"
    );
}

#[test]
fn list_methods() {
    assert_eq!(
        out(
            "fn main() {\n  var xs = [3, 1, 2]\n  print(xs.sorted(), xs.sum(), xs.contains(2), xs.index_of(9), xs.first(), xs.slice(1, 3))\n  print(xs.pop(), xs, xs.length, [\"a\", \"b\"].join(\"-\"))\n  let fs: List<Float> = []\n  print(fs.sum())\n}"
        ),
        "[1, 2, 3] 6 true None Some(3) [1, 2]\nSome(2) [3, 1] 2 a-b\n0.0\n"
    );
}

#[test]
fn float_literals_where_floats_are_expected() {
    assert_eq!(
        out("fn main() {\n  var t: Float = 0\n  print(t)\n  print(max(3, 2.5), min(1, 2))\n}"),
        "0.0\n3.0 1\n"
    );
}

#[test]
fn constants() {
    assert_eq!(
        out("let LIMIT = 3\nlet NAMES = [\"a\", \"b\"]\nfn main() {\n  print(LIMIT * 2, NAMES)\n}"),
        "6 [\"a\", \"b\"]\n"
    );
}

#[test]
fn runtime_errors() {
    assert_eq!(
        err_code("fn main() {\n  let z = 0\n  print(1 / z)\n}"),
        "R0001"
    );
    assert_eq!(
        err_code("fn main() {\n  let z = 0\n  print(1 div z)\n}"),
        "R0001"
    );
    assert_eq!(
        err_code("fn main() {\n  let big = 9223372036854775807\n  print(big + 1)\n}"),
        "R0002"
    );
    assert_eq!(
        err_code("fn main() {\n  let xs = [1]\n  print(xs[3])\n}"),
        "R0003"
    );
    assert_eq!(
        err_code("fn main() {\n  assert(1 > 2, \"math\")\n}"),
        "R0004"
    );
    assert_eq!(
        err_code("fn f(n: Int) -> Int {\n  return f(n + 1)\n}\nfn main() {\n  print(f(0))\n}"),
        "R0005"
    );
    assert_eq!(err_code("fn main() {\n  print((-1.0).sqrt())\n}"), "R0006");
}

#[test]
fn runtime_errors_carry_a_call_trace() {
    let e = run_src(
        "fn inner(xs: List<Int>) -> Int {\n  return xs[5]\n}\nfn main() {\n  print(inner([1]))\n}",
    )
    .unwrap_err();
    assert_eq!(e.code, "R0003");
    assert_eq!(
        e.trace.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        vec!["inner"]
    );
}

#[test]
fn input_reads_lines() {
    let (mut program, _) = elomar_parser::parse(
        "fn main() {\n  let name = input(\"Name? \")\n  print(\"Hi\", name)\n}",
    );
    elomar_types::check(&mut program, Default::default());
    let mut host = BufferHost {
        inputs: vec!["Omar".into()],
        ..Default::default()
    };
    run(&program, &mut host).unwrap();
    assert_eq!(host.output, "Name? Hi Omar\n");
}

// ----- Phase 2: futures and ai.infer ------------------------------------

use elomar_ai_core::{AiPolicy, AiRuntime, MockRule};
use elomar_ai_provider_mock::MockProvider;

fn run_ai(src: &str, rules: Vec<MockRule>, policy: AiPolicy) -> String {
    let (mut program, diags) = elomar_parser::parse(src);
    assert!(diags.is_empty(), "parse errors: {diags:#?}");
    let diags = elomar_types::check(
        &mut program,
        elomar_types::CheckOptions {
            require_main: true,
            ..Default::default()
        },
    );
    assert!(
        diags.iter().all(|d| !d.is_error()),
        "check errors: {diags:#?}"
    );
    let mut ai = AiRuntime::new(policy);
    ai.register("support", Box::new(MockProvider::new(rules).unwrap()));
    let mut host = BufferHost::default();
    run_with_ai(&program, &mut host, &mut ai).unwrap();
    host.output
}

fn rule(contains: Option<&str>, response: Option<&str>, error: Option<&str>) -> MockRule {
    MockRule {
        input_contains: contains.map(Into::into),
        response: response.map(Into::into),
        error: error.map(Into::into),
        ..Default::default()
    }
}

const CLASSIFY: &str = "use std.ai
type Ticket = { category: String, priority: String, tags: List<String>, score: Option<Float> }

async fn classify(message: String) -> Result<Ticket, AIError> !{ai} {
    return await ai.infer<Ticket>(
        model: \"support\",
        instructions: \"Classify the support request.\",
        input: message,
        timeout: 2s
    )
}

async fn main() {
    for m in [\"forgot my password\", \"app is slow\", \"refund please\", \"weird\"] {
        match await classify(m) {
            Ok(t) => print(\"ok\", t.category, t.priority, t.tags, t.score)
            Err(e) => print(\"err\", e.kind)
        }
    }
}
";

#[test]
fn ai_infer_with_mock_rules() {
    let mut slow = rule(Some("slow"), Some("{}"), None);
    slow.delay_ms = Some(5_000);
    let out = run_ai(
        CLASSIFY,
        vec![
            rule(
                Some("password"),
                Some(
                    r#"{"category": "account", "priority": "high", "tags": ["login"], "score": 0.9}"#,
                ),
                None,
            ),
            slow,
            rule(Some("refund"), Some(r#"{"category": "billing"}"#), None),
        ],
        AiPolicy::default(),
    );
    assert_eq!(
        out,
        "ok account high [\"login\"] Some(0.9)\nerr Timeout\nerr InvalidStructuredOutput\nok mock mock [] None\n"
    );
}

#[test]
fn ai_policy_limits_calls() {
    let out = run_ai(
        CLASSIFY,
        Vec::new(),
        AiPolicy {
            max_calls_per_run: Some(2),
            default_timeout_ms: None,
        },
    );
    assert_eq!(out.matches("BudgetExceeded").count(), 2, "{out}");
}

#[test]
fn unknown_model_is_an_error_value() {
    let src = "use std.ai\nasync fn main() {\n  let r = await ai.infer<String>(\"nope\", \"i\", \"x\")\n  match r {\n    Ok(s) => print(s)\n    Err(e) => print(e.kind)\n  }\n}";
    assert_eq!(
        run_ai(src, vec![], AiPolicy::default()),
        "ProviderUnavailable\n"
    );
}

#[test]
fn futures_are_lazy_and_memoized() {
    let src = "async fn noisy() -> Int {\n  print(\"running\")\n  return 7\n}\nasync fn main() {\n  let f = noisy()\n  print(\"before\")\n  let a = await f\n  let b = await f\n  print(a + b)\n}";
    assert_eq!(out(src), "before\nrunning\n14\n");
}

#[test]
fn durations_at_runtime() {
    assert_eq!(
        out(
            "fn main() {\n  let t = 1m + 30s\n  print(t, t.millis(), t.seconds(), 1500ms, t > 1m)\n}"
        ),
        "90s 90000 90.0 1500ms true\n"
    );
}
