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
    let src = "use std.json\nenum Level { Low, High }\ntype Student = { name: String, grades: List<Int>, level: Level, note: Option<String>, scores: Map<String, Float> }\nfn main() {\n  let text = \"{\\\"name\\\": \\\"Lina\\\", \\\"grades\\\": [90, 85], \\\"level\\\": \\\"High\\\", \\\"scores\\\": {\\\"math\\\": 9.5}}\"\n  match json.parse<Student>(text) {\n    Ok(s) => {\n      print(s.name, s.grades, s.level, s.note, s.scores[\"math\"])\n      print(json.stringify(s))\n    }\n    Err(e) => print(e)\n  }\n  print(json.parse<List<Int>>(\"[1, 2\"))\n  print(json.parse<List<Int>>(\"[1, \\\"x\\\"]\"))\n  print(json.pretty([\"a\": [1, 2]]))\n}";
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
        "use std.http\nfn show(r: Result<{{ status: Int, body: String }}, HttpError>) {{\n  match r {{\n    Ok(resp) => print(resp.status, resp.body)\n    Err(e) => print(e.kind())\n  }}\n}}\nfn main() {{\n  show(http.get(\"{base}/hello\"))\n  show(http.post(\"{base}/items\", \"{{\\\"a\\\": 1}}\"))\n  show(http.get(\"{base}/missing\"))\n  show(http.get(\"{base}/redirect\"))\n  show(http.get(\"http://127.0.0.1:1/other-port\"))\n  show(http.get(\"http://example.com/\"))\n  show(http.get(\"not a url\"))\n}}"
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
