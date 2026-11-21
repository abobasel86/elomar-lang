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
            Err(e) => print(\"err\", e.kind())
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
    let src = "use std.ai\nasync fn main() {\n  let r = await ai.infer<String>(\"nope\", \"i\", \"x\")\n  match r {\n    Ok(s) => print(s)\n    Err(e) => print(e.kind())\n  }\n}";
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

// ----- Phase 3: enums, Map, `?` --------------------------------------------

#[test]
fn enums_at_runtime() {
    let src = "enum Shape {\n  Circle(Float)\n  Rect(Float, Float)\n  Empty\n}\nfn area(s: Shape) -> Float {\n  return match s {\n    Shape.Circle(r) => 3 * r * r\n    Shape.Rect(w, h) => w * h\n    Shape.Empty => 0\n  }\n}\nfn main() {\n  let shapes = [Shape.Circle(1.0), Shape.Rect(2.0, 3.0), Shape.Empty]\n  for s in shapes { print(s, area(s)) }\n  print(Shape.Rect(1.0, 2.0) == Shape.Rect(1.0, 2.0), Shape.Empty == Shape.Circle(0.0))\n}";
    assert_eq!(
        out(src),
        "Shape.Circle(1.0) 3.0\nShape.Rect(2.0, 3.0) 6.0\nShape.Empty 0.0\ntrue false\n"
    );
}

#[test]
fn recursive_enums() {
    let src = "enum Tree {\n  Leaf\n  Node(Tree, Int, Tree)\n}\nfn sum(t: Tree) -> Int {\n  return match t {\n    Tree.Leaf => 0\n    Tree.Node(l, v, r) => sum(l) + v + sum(r)\n  }\n}\nfn main() {\n  let t = Tree.Node(Tree.Node(Tree.Leaf, 1, Tree.Leaf), 2, Tree.Node(Tree.Leaf, 3, Tree.Leaf))\n  print(sum(t))\n}";
    assert_eq!(out(src), "6\n");
}

#[test]
fn maps_at_runtime() {
    let src = "fn main() {\n  var stock = [\"pear\": 0, \"apple\": 3]\n  stock[\"kiwi\"] = 5\n  stock[\"apple\"] += 1\n  print(stock, stock.length, stock[\"apple\"], stock.get(\"plum\"))\n  print(stock.keys(), stock.values(), stock.contains_key(\"pear\"), stock.remove(\"pear\"), stock.length)\n  for e in stock { print(e.key, \"=\", e.value) }\n  let copy = stock\n  stock[\"kiwi\"] = 0\n  print(copy[\"kiwi\"], stock[\"kiwi\"])\n  var nested: Map<String, List<Int>> = [\"a\": [1]]\n  nested[\"a\"].push(2)\n  print(nested, [:] == nested)\n}";
    assert_eq!(
        out(src),
        "[\"apple\": 4, \"kiwi\": 5, \"pear\": 0] 3 4 None\n[\"apple\", \"kiwi\", \"pear\"] [4, 5, 0] true Some(0) 2\napple = 4\nkiwi = 5\n5 0\n[\"a\": [1, 2]] false\n"
    );
}

#[test]
fn missing_map_key_is_a_runtime_error() {
    assert_eq!(
        err_code("fn main() {\n  let m = [\"a\": 1]\n  print(m[\"b\"])\n}"),
        "R0011"
    );
    assert_eq!(
        err_code("fn main() {\n  var m = [\"a\": 1]\n  m[\"b\"] += 1\n}"),
        "R0011"
    );
}

#[test]
fn try_operator_at_runtime() {
    let src = "fn parse(s: String) -> Result<Int, String> {\n  return match parse_int(s) { Some(n) => Ok(n), None => Err(\"not a number: \" + s) }\n}\nfn sum(a: String, b: String) -> Result<Int, String> {\n  let x = parse(a)?\n  print(\"parsed\", x)\n  return Ok(x + parse(b)?)\n}\nfn first_doubled(xs: List<Int>) -> Option<Int> {\n  return Some(xs.first()? * 2)\n}\nfn main() {\n  print(sum(\"1\", \"2\"))\n  print(sum(\"x\", \"2\"))\n  print(sum(\"1\", \"y\"))\n  print(first_doubled([4]), first_doubled([]))\n}";
    assert_eq!(
        out(src),
        "parsed 1\nOk(3)\nErr(\"not a number: x\")\nparsed 1\nErr(\"not a number: y\")\nSome(8) None\n"
    );
}

#[test]
fn ai_errors_are_enum_values() {
    let mut slow = rule(Some("slow"), Some("\"x\""), None);
    slow.delay_ms = Some(9_000);
    let src = "use std.ai\nasync fn ask(q: String) -> Result<String, AIError> {\n  return await ai.infer<String>(\"support\", \"i\", q, timeout: 1s)\n}\nasync fn main() {\n  for q in [\"slow\", \"limit\"] {\n    match await ask(q) {\n      Ok(s) => print(s)\n      Err(AIError.Timeout(msg)) => print(\"timeout:\", msg)\n      Err(e) => print(e, e.kind(), e.message())\n    }\n  }\n}";
    let out = run_ai(
        src,
        vec![slow, rule(Some("limit"), None, Some("RateLimited"))],
        AiPolicy::default(),
    );
    assert_eq!(
        out,
        "timeout: no answer within 1000 ms (mock latency 9000 ms)\nAIError.RateLimited(\"simulated RateLimited error\") RateLimited simulated RateLimited error\n"
    );
}

#[test]
fn ai_answers_into_enums() {
    let src = "use std.ai\nenum Priority { Low, Normal, High }\ntype Ticket = { priority: Priority, title: String }\nasync fn main() {\n  for q in [\"server down\", \"typo\", \"other\"] {\n    match await ai.infer<Ticket>(\"support\", \"Classify\", q) {\n      Ok(t) => print(t.title, t.priority, t.priority == Priority.High)\n      Err(e) => print(e.kind(), e.message())\n    }\n  }\n}";
    let out = run_ai(
        src,
        vec![
            rule(
                Some("down"),
                Some(r#"{"priority": "High", "title": "Outage"}"#),
                None,
            ),
            rule(
                Some("typo"),
                Some(r#"{"priority": "Urgent", "title": "Typo"}"#),
                None,
            ),
        ],
        AiPolicy::default(),
    );
    assert_eq!(
        out,
        "Outage Priority.High true\nInvalidStructuredOutput the model's answer does not match `{ priority: Priority, title: String }`: $.priority: expected one of Low, Normal, High, found \"Urgent\"\nmock Priority.Low false\n"
    );
}

// ----- std.fs and std.json -----------------------------------------------

fn run_fs(src: &str, policy: FsPolicy) -> String {
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
    let mut host = BufferHost::default();
    run_with(&program, &mut host, &mut AiRuntime::default(), policy).unwrap();
    host.output
}

fn temp_project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("elomar-interp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::create_dir_all(dir.join("out")).unwrap();
    std::fs::write(dir.join("data/hello.txt"), "\u{feff}مرحبا\nworld").unwrap();
    std::fs::write(dir.join("secret.txt"), "s").unwrap();
    dir
}

#[test]
fn files_with_permissions() {
    let dir = temp_project("fs");
    let policy = FsPolicy::new(&dir, &["data".into(), "out".into()], &["out".into()]);
    let src = "use std.fs\nfn show(r: Result<String, FsError>) {\n  match r {\n    Ok(t) => print(\"ok:\", t.split(\"\\n\"))\n    Err(e) => print(\"err:\", e.kind())\n  }\n}\nfn main() {\n  show(fs.read_text(\"data/hello.txt\"))\n  show(fs.read_text(\"data/missing.txt\"))\n  show(fs.read_text(\"secret.txt\"))\n  show(fs.read_text(\"data/../secret.txt\"))\n  print(fs.write_text(\"out/a.txt\", \"one\"), fs.append_text(\"out/a.txt\", \"+two\"))\n  show(fs.read_text(\"out/a.txt\"))\n  print(fs.write_text(\"data/x.txt\", \"no\").is_err(), fs.exists(\"out/a.txt\"), fs.list_dir(\"data\"))\n}";
    assert_eq!(
        run_fs(src, policy),
        "ok: [\"مرحبا\", \"world\"]\nerr: NotFound\nerr: PolicyDenied\nerr: PolicyDenied\nOk(()) Ok(())\nok: [\"one+two\"]\ntrue Ok(true) Ok([\"hello.txt\"])\n"
    );
    assert!(
        !dir.join("data/x.txt").exists(),
        "denied write must not create the file"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn files_are_denied_by_default() {
    let src = "use std.fs\nfn main() {\n  match fs.read_text(\"Cargo.toml\") {\n    Ok(_) => print(\"read\")\n    Err(e) => print(e)\n  }\n}";
    let out = run_fs(src, FsPolicy::deny_all("."));
    assert!(
        out.starts_with("FsError.PolicyDenied(\"reading `Cargo.toml` is not permitted"),
        "{out}"
    );
}

#[test]
fn json_round_trip() {
    let src = "use std.json\nenum Level { Low, High }\ntype Student = { name: String, grades: List<Int>, level: Level, note: Option<String>, scores: Map<String, Float> }\nfn main() {\n  let text = \"\\{\\\"name\\\": \\\"Lina\\\", \\\"grades\\\": [90, 85], \\\"level\\\": \\\"High\\\", \\\"scores\\\": \\{\\\"math\\\": 9.5}}\"\n  match json.parse<Student>(text) {\n    Ok(s) => {\n      print(s.name, s.grades, s.level, s.note, s.scores[\"math\"])\n      print(json.stringify(s))\n    }\n    Err(e) => print(e)\n  }\n  print(json.parse<List<Int>>(\"[1, 2\"))\n  print(json.parse<List<Int>>(\"[1, \\\"x\\\"]\"))\n  print(json.pretty([\"a\": [1, 2]]))\n}";
    assert_eq!(
        out(src),
        // Record types are structural, so parsed records list fields alphabetically.
        "Lina [90, 85] Level.High None 9.5\n{\"grades\":[90,85],\"level\":\"High\",\"name\":\"Lina\",\"note\":null,\"scores\":{\"math\":9.5}}\nErr(JsonError.InvalidJson(\"invalid JSON: EOF while parsing a list at line 1 column 5\"))\nErr(JsonError.TypeMismatch(\"the JSON does not match `List<Int>`: $[1]: expected Int, found \\\"x\\\"\"))\n{\n  \"a\": [\n    1,\n    2\n  ]\n}\n"
    );
}

#[test]
fn sets_at_runtime() {
    let src = "fn main() {\n  var seen = set([3, 1, 3, 2])\n  print(seen, seen.length)\n  seen.add(5)\n  print(seen.remove(1), seen.remove(42), seen)\n  let evens = set([2, 4])\n  print(seen.union(evens), seen.intersection(evens), seen.difference(evens), set([2]).is_subset(seen))\n  for x in set([\"b\", \"a\"]) { print(x) }\n  let copy = seen\n  seen.add(9)\n  print(copy.contains(9), seen.contains(9), set([1, 2]) == set([2, 1]))\n}";
    assert_eq!(
        out(src),
        "set([1, 2, 3]) 3\ntrue false set([2, 3, 5])\nset([2, 3, 4, 5]) set([2]) set([3, 5]) true\na\nb\nfalse true true\n"
    );
}

#[test]
fn generic_functions_at_runtime() {
    let src = "fn first<T>(xs: List<T>) -> Option<T> {\n  if xs.is_empty() { return None }\n  return Some(xs[0])\n}\nfn reversed<T>(xs: List<T>) -> List<T> {\n  var out: List<T> = []\n  for x in xs { out.insert(0, x) }\n  return out\n}\nfn main() {\n  print(first([3, 4]), first<String>([]), reversed([\"a\", \"b\", \"c\"]))\n}";
    assert_eq!(out(src), "Some(3) None [\"c\", \"b\", \"a\"]\n");
}

#[test]
fn closures_at_runtime() {
    let src = "fn double(x: Int) -> Int {\n  return x * 2\n}\nfn adder(n: Int) -> fn(Int) -> Int {\n  return |x| x + n\n}\nfn main() {\n  let xs = [3, 1, 2]\n  print(xs.map(|x| x * 10), xs.map(double), xs.filter(|x| x > 1))\n  print(xs.any(|x| x == 2), xs.all(|x| x > 1), xs.find(|x| x > 5), xs.count(|x| x != 1))\n  print(xs.fold(0, |acc, x| acc + x), [\"bb\", \"a\", \"ccc\"].sort_by(|s| s.length))\n  var total = 0\n  let f = || total + 1\n  total = 100\n  let add5 = adder(5)\n  print(f(), add5(1), adder(1)(1), f)\n}";
    assert_eq!(
        out(src),
        "[30, 10, 20] [6, 2, 4] [3, 2]\ntrue false None 2\n6 [\"a\", \"bb\", \"ccc\"]\n1 6 2 <function>\n"
    );
}

#[test]
fn closures_return_and_recurse() {
    let src = "fn fact(n: Int) -> Int {\n  if n <= 1 { return 1 }\n  return n * fact(n - 1)\n}\nfn main() {\n  let f = |n: Int| {\n    if n < 0 {\n      return 0\n    }\n    return fact(n)\n  }\n  print(f(5), f(-1), [1, 2, 3].map(fact))\n}";
    assert_eq!(out(src), "120 0 [1, 2, 6]\n");
}

// ----- std.http ----------------------------------------------------------------

/// Serves `count` requests on 127.0.0.1: `/redirect` answers 302, anything
/// else echoes the method, path and body with status 200 (or 404 for
/// `/missing`).
fn test_server(count: usize) -> u16 {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(count) {
            let mut stream = stream.expect("connection");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("request line");
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("header");
                if line.trim().is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().expect("length");
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).expect("body");
            let mut parts = request_line.split_whitespace();
            let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            let (status, extra) = match path {
                "/redirect" => ("302 Found", "Location: https://elsewhere.example/\r\n"),
                "/missing" => ("404 Not Found", ""),
                _ => ("200 OK", ""),
            };
            let text = format!("{method} {path} {}", String::from_utf8_lossy(&body));
            write!(
                stream,
                "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            )
            .expect("write");
        }
    });
    port
}

#[test]
fn http_requests() {
    let port = test_server(4);
    let base = format!("http://127.0.0.1:{port}");
    let src = format!(
        "use std.http\nfn show(r: Result<{{ status: Int, body: String }}, HttpError>) {{\n  match r {{\n    Ok(resp) => print(resp.status, resp.body)\n    Err(e) => print(e.kind())\n  }}\n}}\nfn main() {{\n  show(http.get(\"{base}/hello\"))\n  show(http.post(\"{base}/items\", \"\\{{\\\"a\\\": 1}}\"))\n  show(http.get(\"{base}/missing\"))\n  show(http.get(\"{base}/redirect\"))\n  show(http.get(\"http://127.0.0.1:1/other-port\"))\n  show(http.get(\"http://example.com/\"))\n  show(http.get(\"not a url\"))\n}}"
    );
    let (mut program, diags) = elomar_parser::parse(&src);
    assert!(diags.is_empty(), "{diags:#?}");
    let diags = elomar_types::check(&mut program, elomar_types::CheckOptions::default());
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:#?}");
    let permissions = elomar_ai_core::Permissions {
        network_hosts: vec![format!("127.0.0.1:{port}"), "example.com".into()],
        ..Default::default()
    };
    let output = with_large_stack(|| {
        let mut host = BufferHost::default();
        let mut ai = AiRuntime::default().with_permissions(permissions);
        run_with(&program, &mut host, &mut ai, FsPolicy::deny_all(".")).expect("run");
        host.output
    });
    assert_eq!(
        output,
        "200 GET /hello \n200 POST /items {\"a\": 1}\n404 GET /missing \n302 GET /redirect \nPolicyDenied\nInvalidUrl\nInvalidUrl\n"
    );
}

#[test]
fn generic_enums_at_runtime() {
    let src = "enum Tree<T> {\n  Leaf\n  Node(Tree<T>, T, Tree<T>)\n}\nfn insert(t: Tree<Int>, x: Int) -> Tree<Int> {\n  return match t {\n    Tree.Leaf => Tree.Node(Tree.Leaf, x, Tree.Leaf)\n    Tree.Node(l, v, r) => match x < v {\n      true => Tree.Node(insert(l, x), v, r)\n      false => Tree.Node(l, v, insert(r, x))\n    }\n  }\n}\nfn items<T>(t: Tree<T>) -> List<T> {\n  return match t {\n    Tree.Leaf => []\n    Tree.Node(l, v, r) => items(l) + [v] + items(r)\n  }\n}\nfn main() {\n  print(items([5, 2, 8, 1].fold(Tree.Leaf, |t, x| insert(t, x))))\n}";
    assert_eq!(out(src), "[1, 2, 5, 8]\n");
}

#[test]
fn traits_at_runtime() {
    let src = "trait Area {\n  fn area(self) -> Float\n  fn scaled(self, k: Float) -> Self\n}\nenum Shape {\n  Circle(Float)\n  Square(Float)\n}\ntype Rect = { w: Float, h: Float }\nimpl Area for Shape {\n  fn area(self) -> Float {\n    return match self {\n      Shape.Circle(r) => 3.0 * r * r\n      Shape.Square(s) => s * s\n    }\n  }\n  fn scaled(self, k: Float) -> Self {\n    return match self {\n      Shape.Circle(r) => Shape.Circle(r * k)\n      Shape.Square(s) => Shape.Square(s * k)\n    }\n  }\n}\nimpl Area for Rect {\n  fn area(self) -> Float {\n    return self.w * self.h\n  }\n  fn scaled(self, k: Float) -> Rect {\n    return { w: self.w * k, h: self.h * k }\n  }\n}\nimpl Rect {\n  fn is_square(self) -> Bool {\n    return self.w == self.h\n  }\n}\nfn total<T: Area>(xs: List<T>) -> Float {\n  return xs.fold(0.0, |acc, x| acc + x.area())\n}\nfn double_total<T: Area>(xs: List<T>) -> Float {\n  return total(xs.map(|x| x.scaled(2.0)))\n}\nfn main() {\n  let r: Rect = { w: 1.0, h: 2.0 }\n  print(r.area(), r.is_square(), r.scaled(3.0))\n  print(total([Shape.Circle(1.0), Shape.Square(2.0)]), double_total([r]))\n}";
    assert_eq!(out(src), "2.0 false { w: 3.0, h: 6.0 }\n7.0 8.0\n");
}

// ----- std.db ----------------------------------------------------------------------

#[test]
fn sqlite_in_memory() {
    let src = "use std.db\ntype Student = { id: Int, name: String, grade: Float, email: Option<String>, active: Bool }\nfn setup(conn: Db) -> Result<Int, DbError> {\n  db.execute(conn, \"CREATE TABLE s (id INTEGER PRIMARY KEY, name TEXT UNIQUE, grade REAL, email TEXT, active INTEGER); CREATE INDEX i ON s(name)\")?\n  for p in [{ n: \"Lina\", g: 91 }, { n: \"Omar\", g: 55 }] {\n    db.execute(conn, \"INSERT INTO s (name, grade, active) VALUES (:n, :g, 1)\", p)?\n  }\n  return db.execute(conn, \"UPDATE s SET email = :e WHERE name = :n\", { e: Some(\"l@x\"), n: \"Lina\" })\n}\nfn main() {\n  match db.open(\":memory:\") {\n    Ok(conn) => {\n      print(setup(conn))\n      print(db.query<Student>(conn, \"SELECT * FROM s WHERE grade > :min\", { min: 60 }))\n      print(db.query<String>(conn, \"SELECT name FROM s ORDER BY name\"), db.query_one<Int>(conn, \"SELECT id FROM s WHERE name = 'x'\"))\n      print(db.execute(conn, \"INSERT INTO s (name) VALUES ('Lina')\").is_err(), db.query<{ email: String }>(conn, \"SELECT email FROM s\").is_err())\n      match db.query<Int>(conn, \"SELECT missing FROM s\") {\n        Ok(_) => print(\"?\")\n        Err(e) => print(e.kind())\n      }\n    }\n    Err(e) => print(e)\n  }\n}";
    assert_eq!(
        out(src),
        "Ok(1)\nOk([{ active: true, email: Some(\"l@x\"), grade: 91.0, id: 1, name: \"Lina\" }])\nOk([\"Lina\", \"Omar\"]) Ok(None)\ntrue true\nSql\n"
    );
}

#[test]
fn sqlite_file_needs_write_permission() {
    let dir = temp_project("db");
    let policy = FsPolicy::new(&dir, &["data".into()], &["out".into()]);
    let src = "use std.db\nfn main() {\n  match db.open(\"out/app.db\") {\n    Ok(conn) => {\n      print(db.execute(conn, \"CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (7)\"))\n      print(db.query<Int>(conn, \"SELECT x FROM t\"))\n    }\n    Err(e) => print(e)\n  }\n  match db.open(\"data/app.db\") {\n    Ok(_) => print(\"opened\")\n    Err(e) => print(e.kind())\n  }\n}";
    assert_eq!(run_fs(src, policy), "Ok(1)\nOk([7])\nPolicyDenied\n");
    assert!(dir.join("out/app.db").exists());
}

// ----- ai.run with tools ----------------------------------------------------------

fn tool_rule(tool: &str, arguments: serde_json::Value) -> MockRule {
    MockRule {
        tool: Some(tool.into()),
        arguments: Some(arguments),
        ..Default::default()
    }
}

const AGENT: &str = "use std.ai
type Student = { name: String, grade: Int }
let STUDENTS = [{ name: \"Lina\", grade: 91 }, { name: \"Omar\", grade: 55 }]
fn find_student(name: String) -> Option<Student> {
  print(\"tool:\", name)
  return STUDENTS.find(|s| s.name == name)
}
fn class_average() -> Float {
  return STUDENTS.fold(0, |acc, s| acc + s.grade) / STUDENTS.length
}
async fn main() {
  print(await ai.run<{ answer: String }>(model: \"support\", instructions: \"i\", input: \"q\", tools: [find_student, class_average], max_steps: 4))
}";

#[test]
fn ai_run_calls_tools_until_an_answer() {
    let rules = vec![
        tool_rule("find_student", serde_json::json!({ "name": "Lina" })),
        tool_rule("class_average", serde_json::json!({})),
        rule(
            None,
            Some("{\"answer\": \"average {{last_tool_result}}\"}"),
            None,
        ),
    ];
    assert_eq!(
        run_ai(AGENT, rules, AiPolicy::default()),
        "tool: Lina\nOk({ answer: \"average 73.0\" })\n"
    );
}

#[test]
fn ai_run_reports_bad_tool_calls_to_the_model_and_limits_steps() {
    // Invalid arguments and unknown tools are sent back as errors; the mock
    // then keeps asking until `max_steps` runs out.
    let rules = vec![
        tool_rule("find_student", serde_json::json!({ "name": 3 })),
        tool_rule("find_student", serde_json::json!({ "name": "Omar" })),
        tool_rule("class_average", serde_json::json!({})),
        tool_rule("class_average", serde_json::json!({})),
    ];
    let out = run_ai(AGENT, rules, AiPolicy::default());
    assert_eq!(
        out,
        "tool: Omar\nErr(AIError.BudgetExceeded(\"the model did not finish within 4 step(s) (`max_steps`)\"))\n"
    );
    // Every step counts against `max_calls_per_run`.
    let rules = vec![tool_rule("class_average", serde_json::json!({}))];
    let policy = AiPolicy {
        max_calls_per_run: Some(1),
        ..Default::default()
    };
    assert!(
        run_ai(AGENT, rules, policy)
            .contains("AIError.BudgetExceeded(\"this run may make at most 1")
    );
}

// ----- agent.run and std.mcp ------------------------------------------------------------

const AGENT_RUN: &str = "use std.agent
use std.mcp
fn add(n: Int) -> Int {
  return n + 1
}
fn wipe() -> String {
  print(\"wiped!\")
  return \"done\"
}
async fn main() {
  let r = await agent.run<String>(
    model: \"support\", instructions: \"i\", input: \"q\",
    tools: [add, wipe],
    require_approval: [\"wipe\"],
    approve: |c| { print(\"approve?\", c.tool, c.arguments); return false },
    on_step: |s| print(\"step:\", s.tool, s.result, s.approved)
  )
  print(r)
  print(await agent.run<String>(model: \"support\", instructions: \"i\", input: \"q\", mcp: [\"nowhere\"]))
  print(mcp.call(\"nowhere\", \"t\", {}))
}";

#[test]
fn agent_run_asks_for_approval_and_reports_steps() {
    let rules = vec![
        tool_rule("add", serde_json::json!({ "n": 1 })),
        tool_rule("wipe", serde_json::json!({})),
        rule(None, Some("\"finished\""), None),
    ];
    let out = run_ai(AGENT_RUN, rules, AiPolicy::default());
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            "step: add 2 true",
            "approve? wipe {}",
            "step: wipe {\"error\":\"the user did not approve this call\"} false",
            "Ok(\"finished\")",
            "Err(AIError.PolicyDenied(\"MCP server `nowhere` is not configured; add `[mcp.servers.nowhere]` with its `command` to elomar.toml\"))",
            "Err(McpError.NotConfigured(\"MCP server `nowhere` is not configured; add `[mcp.servers.nowhere]` with its `command` to elomar.toml\"))",
        ]
    );
}

// ----- std.py --------------------------------------------------------------------------

fn run_py(src: &str, allow: bool) -> String {
    let dir = temp_project(if allow { "py-allowed" } else { "py-denied" });
    std::fs::create_dir_all(dir.join("python")).unwrap();
    std::fs::write(
        dir.join("python/helpers.py"),
        "def stats(xs, label):\n    print('working on', label)\n    return {'label': label, 'total': sum(xs), 'top': max(xs)}\n\ndef boom(x):\n    raise KeyError(x)\n",
    )
    .unwrap();
    let (mut program, diags) = elomar_parser::parse(src);
    assert!(diags.is_empty(), "{diags:#?}");
    let diags = elomar_types::check(&mut program, elomar_types::CheckOptions::default());
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:#?}");
    let permissions = elomar_ai_core::Permissions {
        python: allow,
        ..Default::default()
    };
    with_large_stack(|| {
        let mut host = BufferHost::default();
        let mut ai = AiRuntime::default().with_permissions(permissions);
        run_with(&program, &mut host, &mut ai, FsPolicy::deny_all(&dir)).expect("run");
        host.output
    })
}

const PY_SRC: &str = "use std.py\nfn main() {\n  print(py.call<{ label: String, total: Int, top: Int }>(\"helpers\", \"stats\", { xs: [3, 9, 4], label: \"مجموعة\" }))\n  print(py.call<Float>(\"math\", \"sqrt\", 2.25))\n  match py.call<Int>(\"helpers\", \"boom\", \"k\") {\n    Ok(_) => print(\"?\")\n    Err(e) => print(e.kind(), e.message())\n  }\n  print(py.call<Int>(\"math\", \"sqrt\", 2.25).is_err())\n}";

#[test]
fn python_calls() {
    assert_eq!(
        run_py(PY_SRC, true),
        "working on مجموعة\nOk({ label: \"مجموعة\", top: 9, total: 16 })\nOk(1.5)\nException KeyError: 'k'\ntrue\n"
    );
}

#[test]
fn python_needs_permission() {
    assert!(run_py(PY_SRC, false).starts_with("Err(PyError.PolicyDenied("));
}

#[test]
fn string_interpolation() {
    let src = r#"type P = { name: String, age: Int }
fn main() {
  let p: P = { name: "Lina", age: 30 }
  let xs = [1, 2]
  let m = ["k": 2.5]
  print("Hi {p.name}, next year {p.age + 1}.")
  print("{xs} {m["k"]} {xs.length > 1} {Some(3)} \{literal\}")
  print("مرحبا {p.name}! {"nested {p.age}"}")
}"#;
    assert_eq!(
        out(src),
        "Hi Lina, next year 31.\n[1, 2] 2.5 true Some(3) {literal}\nمرحبا Lina! nested 30\n"
    );
}

#[test]
fn more_methods() {
    let src = r#"fn main() {
  let s = "  مرحبا Elomar  "
  print("[{s.trim_start()}]", "[{s.trim_end()}]", "abc".reverse())
  print("hello world".index_of("world"), "abc".index_of("z"), "مرحبا".slice(1, 3))
  print("a b\tc".words(), "x\ny".lines(), "7".pad_start(3, "0"), "ab".pad_end(4, "."), "x".pad_start(3))
  print(2.pow(10), 15.clamp(0, 10), 2.0.pow(0.5).round_to(3), 3.14159.to_fixed(2), 1.5.clamp(0.0, 1.0))
  let xs = [3, 1, 3, 2, 1]
  print(xs.take(2), xs.skip(3), xs.take(99), xs.unique(), xs.min(), xs.max(), ["b", "a"].min())
  let empty: List<Int> = []
  print(empty.max(), xs.chunks(2))
  for e in ["a", "b"].enumerate() {
    print(e.index, e.value)
  }
  print([1, 2].flat_map(|x| [x, x * 10]))
  let groups = ["apple", "avocado", "banana"].group_by(|w| w.slice(0, 1))
  print(groups)
  let m = ["a": 1]
  print(m.get_or("a", 0), m.get_or("z", 0), m.entries())
}"#;
    assert_eq!(
        out(src),
        "[مرحبا Elomar  ] [  مرحبا Elomar] cba\n\
         Some(6) None رح\n\
         [\"a\", \"b\", \"c\"] [\"x\", \"y\"] 007 ab.. __x\n\
         1024 10 1.414 3.14 1.0\n\
         [3, 1] [2, 1] [3, 1, 3, 2, 1] [3, 1, 2] Some(1) Some(3) Some(\"a\")\n\
         None [[3, 1], [3, 2], [1]]\n\
         0 a\n1 b\n\
         [1, 10, 2, 20]\n\
         [\"a\": [\"apple\", \"avocado\"], \"b\": [\"banana\"]]\n\
         1 0 [{ key: \"a\", value: 1 }]\n"
            .replace("__x", "  x")
    );
    assert_eq!(
        err_code("fn main() {\n  print(\"ab\".slice(1, 5))\n}"),
        "R0003"
    );
    assert_eq!(err_code("fn main() {\n  print(2.pow(-1))\n}"), "R0006");
    assert_eq!(err_code("fn main() {\n  print([1].chunks(0))\n}"), "R0006");
}

#[test]
fn math_random_time_os() {
    let src = r#"use std.math
use std.random
use std.time
use std.os
fn main() {
  print(math.pi.round_to(4), math.sqrt(16), math.pow(2, 10), math.hypot(3.0, 4), math.ln(math.e), math.is_nan(math.sqrt(-1.0)))
  random.seed(42)
  let a = [random.int(1, 6), random.int(1, 6), random.int(1, 6)]
  random.seed(42)
  let b = [random.int(1, 6), random.int(1, 6), random.int(1, 6)]
  print(a == b, a.all(|x| x >= 1 && x <= 6), random.float() < 1.0, random.choice([7]), random.shuffle([1, 2, 3]).sorted())
  let empty: List<Int> = []
  print(random.choice(empty))
  let t = time.make(2026, 11, 20, 9, 30)
  print(time.format(t), time.parse("2026-11-20T12:30:00+03:00") == Some(t), time.parse("nope"))
  let d = time.date(t)
  print(d.year, d.month, d.day, d.hour, d.weekday)
  print(time.now() > t - 100000000000, time.today().length)
  print(os.args(), os.env("ELOMAR_SURELY_UNSET_VARIABLE"), os.platform().length > 0)
  os.exit(3)
  print("not reached")
}"#;
    let (mut program, diags) = elomar_parser::parse(src);
    assert!(diags.is_empty(), "{diags:#?}");
    let diags = elomar_types::check(
        &mut program,
        elomar_types::CheckOptions {
            require_main: true,
            ..Default::default()
        },
    );
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:#?}");
    let mut host = BufferHost {
        args: vec!["--name".into(), "Lina".into()],
        ..Default::default()
    };
    let err = with_large_stack(|| run(&program, &mut host)).unwrap_err();
    assert_eq!(err.exit, Some(3));
    assert_eq!(
        host.output,
        "3.1416 4.0 1024.0 5.0 1.0 true\n\
         true true true Some(7) [1, 2, 3]\n\
         None\n\
         2026-11-20T09:30:00Z true None\n\
         2026 11 20 9 5\n\
         true 10\n\
         [\"--name\", \"Lina\"] None true\n"
    );
}

#[test]
fn regex_encoding_crypto() {
    let src = r##"use std.regex
use std.encoding
use std.crypto
fn main() {
  let text = "Order 12 shipped, order 7 pending"
  print(regex.is_match("[0-9]+", text), regex.find("[0-9]+", text), regex.find_all("[0-9]+", text))
  print(regex.captures("(\\w+)@(\\w+)\\.com", "mail lina@example.com now"), regex.captures("x", "y"))
  print(regex.replace("(\\d+)", text, "#$1"), regex.split("\\s*,\\s*", "a , b,c"))
  let pattern = "["
  print(encoding.base64_encode("مرحبا"), encoding.base64_decode("2YXYsdit2KjYpw=="), encoding.base64_decode("!!"))
  print(encoding.hex_encode("Hi"), encoding.hex_decode("4869"), encoding.url_encode("a b&c=د"), encoding.url_decode("a+b%26c"))
  print(crypto.sha256("abc"))
  print(crypto.hmac_sha256("key", "The quick brown fox jumps over the lazy dog"))
  let id = crypto.uuid()
  print(id.length, id.slice(14, 15), crypto.token(16).length, crypto.uuid() != id)
  print(regex.is_match(pattern, "x"))
}"##;
    assert_eq!(err_code(src), "R0006");
    let src = src.replace("  print(regex.is_match(pattern, \"x\"))\n", "");
    assert_eq!(
        out(&src),
        "true Some(\"12\") [\"12\", \"7\"]\n\
         Some([\"lina@example.com\", \"lina\", \"example\"]) None\n\
         Order #12 shipped, order #7 pending [\"a\", \"b\", \"c\"]\n\
         2YXYsdit2KjYpw== Some(\"مرحبا\") None\n\
         4869 Some(\"Hi\") a%20b%26c%3D%D8%AF Some(\"a b&c\")\n\
         ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n\
         f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8\n\
         36 4 32 true\n"
    );
}

// ----- std.server ------------------------------------------------------------------

/// Sends raw HTTP requests once the server accepts connections; returns
/// each full response.
fn http_client(port: u16, requests: Vec<String>) -> std::thread::JoinHandle<Vec<String>> {
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        let mut out = Vec::new();
        for req in requests {
            let mut stream = loop {
                match std::net::TcpStream::connect(("127.0.0.1", port)) {
                    Ok(s) => break s,
                    Err(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
                }
            };
            stream.write_all(req.as_bytes()).expect("send");
            let mut resp = String::new();
            stream.read_to_string(&mut resp).expect("read");
            out.push(resp);
        }
        out
    })
}

#[test]
fn http_server() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let src = format!(
        r#"use std.server
type Item = {{ name: String, qty: Int }}
fn handle(req: HttpRequest) -> HttpResponse {{
  return match req.path {{
    "/hello" => server.text(200, "Hello, {{req.query.get_or("name", "guest")}}!")
    "/item" => server.json(201, {{ name: req.body, qty: 2 }})
    "/page" => server.html(200, "<h1>{{req.method}}</h1>")
    "/go" => server.redirect("/hello")
    "/boom" => server.text(200, "{{10 div ([0].length - 1)}}")
    _ => ({{ status: 404, headers: ["X-Note": "custom", "Bad\nName": "x"], body: "no {{req.path}}" }})
  }}
}}
fn main() {{
  print(server.serve("127.0.0.1:1", handle, 1))
  print(server.serve("127.0.0.1:{port}", handle, 7))
  print("stopped")
}}"#
    );
    let (mut program, diags) = elomar_parser::parse(&src);
    assert!(diags.is_empty(), "{diags:#?}");
    let diags = elomar_types::check(
        &mut program,
        elomar_types::CheckOptions {
            require_main: true,
            ..Default::default()
        },
    );
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:#?}");
    let get = |path: &str| format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n");
    let client = http_client(
        port,
        vec![
            get("/hello?name=%D9%84%D9%8A%D9%86%D8%A7"),
            "POST /item HTTP/1.1\r\nContent-Length: 4\r\n\r\npens".into(),
            "HEAD /page HTTP/1.1\r\n\r\n".into(),
            get("/go"),
            get("/nothing%20here"),
            get("/boom"),
            "POST /item HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n".into(),
        ],
    );
    let permissions = elomar_ai_core::Permissions {
        listen: vec![format!("127.0.0.1:{port}")],
        ..Default::default()
    };
    let output = with_large_stack(|| {
        let mut host = BufferHost::default();
        let mut ai = AiRuntime::default().with_permissions(permissions);
        run_with(&program, &mut host, &mut ai, FsPolicy::deny_all(".")).expect("run");
        host.output
    });
    let responses = client.join().unwrap();
    let body = |r: &str| {
        r.split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .unwrap()
    };
    assert!(
        responses[0].starts_with("HTTP/1.1 200 OK\r\n"),
        "{}",
        responses[0]
    );
    assert!(responses[0].contains("Content-Type: text/plain; charset=utf-8\r\n"));
    assert_eq!(body(&responses[0]), "Hello, لينا!");
    assert!(responses[1].starts_with("HTTP/1.1 201 Created\r\n"));
    assert!(responses[1].contains("Content-Type: application/json\r\n"));
    assert_eq!(body(&responses[1]), r#"{"name":"pens","qty":2}"#);
    assert!(
        responses[2].contains("Content-Length: 13\r\n"),
        "{}",
        responses[2]
    );
    assert_eq!(body(&responses[2]), "");
    assert!(responses[3].starts_with("HTTP/1.1 303 See Other\r\n"));
    assert!(responses[3].contains("Location: /hello\r\n"));
    assert!(responses[4].starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(responses[4].contains("X-Note: custom\r\n"));
    assert!(!responses[4].contains("Bad"), "{}", responses[4]);
    assert_eq!(body(&responses[4]), "no /nothing here");
    assert!(responses[5].starts_with("HTTP/1.1 500 Internal Server Error\r\n"));
    assert!(responses[6].starts_with("HTTP/1.1 411 Length Required\r\n"));
    assert_eq!(
        output,
        "Err(HttpError.PolicyDenied(\"listening on `127.0.0.1:1` is not permitted; add \\\"127.0.0.1:1\\\" to `listen` under `[permissions]` in elomar.toml\"))\n\
         error while handling GET /boom: division by zero\n\
         Ok(())\n\
         stopped\n"
    );
}

// ----- std.process -----------------------------------------------------------------

#[test]
fn process_run() {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let src = format!(
        r#"use std.process
fn show(r: Result<{{ status: Int, stdout: String, stderr: String }}, ProcessError>) {{
  match r {{
    Ok(o) => print(o.status, o.stdout.trim(), o.stderr.trim())
    Err(e) => print(e.kind())
  }}
}}
fn main() {{
  show(process.run("{python}", ["-c", "import sys; print(sys.argv[1:])", "a b", "; echo hacked", "$HOME"]))
  show(process.run("{python}", ["-c", "import sys; sys.stderr.write('bad'); sys.exit(3)"]))
  show(process.run("{python}", ["-c", "import time; time.sleep(5)"], 300ms))
  show(process.run("not-allowed", []))
  show(process.run("elomar-no-such-program-xyz", []))
}}"#
    );
    let (mut program, diags) = elomar_parser::parse(&src);
    assert!(diags.is_empty(), "{diags:#?}");
    let diags = elomar_types::check(
        &mut program,
        elomar_types::CheckOptions {
            require_main: true,
            ..Default::default()
        },
    );
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:#?}");
    let permissions = elomar_ai_core::Permissions {
        commands: vec![python.into(), "elomar-no-such-program-xyz".into()],
        ..Default::default()
    };
    let output = with_large_stack(|| {
        let mut host = BufferHost::default();
        let mut ai = AiRuntime::default().with_permissions(permissions);
        run_with(&program, &mut host, &mut ai, FsPolicy::deny_all(".")).expect("run");
        host.output
    });
    assert_eq!(
        output,
        "0 ['a b', '; echo hacked', '$HOME'] \n3  bad\nTimeout\nPolicyDenied\nNotFound\n"
    );
}

#[test]
fn process_needs_its_effect() {
    let src = "use std.process\npub fn git_status() -> Bool {\n  return process.run(\"git\", [\"status\"]).is_ok()\n}\nfn main() {\n  print(git_status())\n}";
    let (mut program, _) = elomar_parser::parse(src);
    let diags = elomar_types::check(&mut program, elomar_types::CheckOptions::default());
    assert!(diags.iter().any(|d| d.code == "E3002"), "{diags:#?}");
}

#[test]
fn option_and_result_methods() {
    let src = r#"fn half(n: Int) -> Result<Int, String> {
  return match n % 2 == 0 {
    true => Ok(n div 2)
    false => Err("{n} is odd")
  }
}
fn main() {
  let some = Some(4)
  let none: Option<Int> = None
  print(some.map(|x| x * 10), none.map(|x| x * 10))
  print(some.and_then(|x| match x > 3 {
    true => Some("big")
    false => None
  }), some.filter(|x| x > 9), some.is_some_and(|x| x == 4))
  print(none.unwrap_or_else(|| 7), none.or(Some(1)), none.ok_or("missing"), some.ok_or("missing"))
  print(half(8).map(|x| x + 1), half(3).map(|x| x + 1), half(3).map_err(|e| e.upper()))
  print(half(8).and_then(half), half(6).and_then(half), half(3).unwrap_or_else(|e| e.length))
  print(half(4).ok(), half(5).ok(), half(5).err())
  print("12".parse_int().map(|n| n * 2).unwrap_or(0))
}"#;
    assert_eq!(
        out(src),
        "Some(40) None\n\
         Some(\"big\") None true\n\
         7 Some(1) Err(\"missing\") Ok(4)\n\
         Ok(5) Err(\"3 is odd\") Err(\"3 IS ODD\")\n\
         Ok(2) Err(\"3 is odd\") 8\n\
         Some(2) None Some(\"5 is odd\")\n\
         24\n"
    );
}

#[test]
fn or_and_range_patterns() {
    let src = r#"enum Color {
  Red
  Green
  Blue
}
fn kind(day: String) -> String {
  return match day {
    "Fri" | "Sat" => "weekend"
    _ => "workday"
  }
}
fn grade(score: Int) -> String {
  return match score {
    90..=100 => "excellent"
    70..90 => "good"
    -100..0 => "invalid"
    _ => "keep going"
  }
}
fn warm(c: Color) -> Bool {
  return match c {
    Color.Red | Color.Green => true
    Color.Blue => false
  }
}
fn main() {
  print(kind("Sat"), kind("Mon"))
  print(grade(95), grade(90), grade(89), grade(70), grade(-5), grade(10))
  print(warm(Color.Green), warm(Color.Blue))
  print(match Some(3) {
    Some(1) | Some(2) => "small"
    Some(_) | None => "other"
  })
}"#;
    assert_eq!(
        out(src),
        "weekend workday\nexcellent excellent good good invalid keep going\ntrue false\nother\n"
    );
}

#[test]
fn tuples() {
    let src = r#"fn min_max(xs: List<Int>) -> (Int, Int) {
  return (xs.min().unwrap_or(0), xs.max().unwrap_or(0))
}
fn describe(p: (String, Int)) -> String {
  return match p {
    ("Lina", _) => "the teacher"
    (name, 0) => "{name} has nothing"
    (name, n) => "{name} has {n}"
  }
}
fn main() {
  let pair = (1, "one")
  print(pair, pair.0, pair.1)
  let (lo, hi) = min_max([4, 9, 2])
  print(lo, hi)
  let (a, _, c) = (1, 2, 3)
  print(a + c)
  var (x, y) = (1, 2)
  x += 10
  print(x, y)
  let nested = ((1, 2), "z")
  print(nested.0.1, nested.1)
  print(describe(("Lina", 3)), describe(("Omar", 0)), describe(("Sara", 5)))
  for (name, score) in ["Lina": 91, "Omar": 55] {
    print("{name}: {score}")
  }
  for (i, w) in [(1, "a"), (2, "b")] {
    print(i, w)
  }
  let people = [("Sara", 30), ("Ali", 25), ("Omar", 30)]
  print(people.sort_by(|p| (p.1, p.0)))
  print((1, "a") == (1, "a"), (1, 2) == (2, 1))
}"#;
    assert_eq!(
        out(src),
        "(1, \"one\") 1 one\n\
         2 9\n\
         4\n\
         11 2\n\
         2 z\n\
         the teacher Omar has nothing Sara has 5\n\
         Lina: 91\nOmar: 55\n\
         1 a\n2 b\n\
         [(\"Ali\", 25), (\"Omar\", 30), (\"Sara\", 30)]\n\
         true false\n"
    );
}

#[test]
fn default_trait_methods() {
    let src = r#"trait Greet {
  fn name(self) -> String
  fn greet(self) -> String {
    return "Hello, {self.name()}!"
  }
  fn shout(self, times: Int) -> String {
    return self.greet().upper().repeat(times)
  }
}
type Person = { first: String }
enum Robot {
  Unit(Int)
}
impl Greet for Person {
  fn name(self) -> String {
    return self.first
  }
}
impl Greet for Robot {
  fn name(self) -> String {
    return match self {
      Robot.Unit(n) => "unit {n}"
    }
  }
  fn greet(self) -> String {
    return "Beep, {self.name()}."
  }
}
fn welcome<T: Greet>(x: T) -> String {
  return x.greet()
}
fn main() {
  let p: Person = { first: "Lina" }
  print(p.greet(), p.shout(2))
  print(Robot.Unit(7).greet(), welcome(p), welcome(Robot.Unit(1)))
}"#;
    assert_eq!(
        out(src),
        "Hello, Lina! HELLO, LINA!HELLO, LINA!\nBeep, unit 7. Hello, Lina! Beep, unit 1.\n"
    );
}

#[test]
fn generic_impls() {
    let src = r#"type Stack<T> = { items: List<T> }
impl<T> Stack<T> {
  fn push(self, x: T) -> Stack<T> {
    return { items: self.items + [x] }
  }
  fn peek(self) -> Option<T> {
    return self.items.last()
  }
  fn size(self) -> Int {
    return self.items.length
  }
}
enum Box<T> {
  Full(T)
  Empty
}
trait Describe {
  fn describe(self) -> String
}
impl<T> Describe for Box<T> {
  fn describe(self) -> String {
    return match self {
      Box.Full(x) => "a full box: {x}"
      Box.Empty => "an empty box"
    }
  }
}
fn show<T: Describe>(x: T) -> String {
  return x.describe()
}
fn main() {
  let s: Stack<Int> = { items: [] }
  let s2 = s.push(1).push(2)
  print(s2.size(), s2.peek(), s.peek())
  let names: Stack<String> = { items: ["a"] }
  print(names.push("b").peek().unwrap_or(""))
  print(Box.Full(3).describe(), show(Box.Full("x")))
  let e: Box<Int> = Box.Empty
  print(show(e))
}"#;
    assert_eq!(
        out(src),
        "2 Some(2) None\nb\na full box: 3 a full box: x\nan empty box\n"
    );
}

#[test]
fn await_all() {
    let src = r#"use std.ai
async fn double(n: Int) -> Int {
  return n * 2
}
async fn main() {
  let answers = await all(["a", "b", "c"].map(|t| ai.infer<String>("support", "Echo", t)))
  print(answers.map(|r| r.is_ok()))
  print(await all([double(1), double(2), double(3)]))
  let empty: List<Future<Int>> = []
  print(await all(empty))
  let f = double(5)
  let first = await f
  print(first, await all([f, double(6)]))
}"#;
    // A budget of two calls: the third AI call in the batch is refused.
    let policy = AiPolicy {
        max_calls_per_run: Some(2),
        ..Default::default()
    };
    assert_eq!(
        run_ai(src, Vec::new(), policy),
        "[true, true, false]\n[2, 4, 6]\n[]\n10 [10, 12]\n"
    );
}
