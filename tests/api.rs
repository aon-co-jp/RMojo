use rmojo::{compile_source, Value};
#[test]
fn call_scalar() {
    let p = compile_source("def add(a: int, b: int) -> int:\n    return a + b\ndef hi(n):\n    return 'hi ' + n\n").unwrap();
    assert_eq!(p.call("add", &[Value::Int(2), Value::Int(3)]).unwrap(), Value::Int(5));
    assert_eq!(p.call("hi", &[Value::Str("x".into())]).unwrap(), Value::Str("hi x".into()));
    assert!(p.call("nope", &[]).is_err());
    assert!(p.call("add", &[Value::Int(1)]).is_err());
}

#[test]
fn step_limit_stops_runaway() {
    use rmojo::Limits;
    let p = compile_source("def f():\n    xs = []\n    while True:\n        xs.append(1)\n    return 0\ndef g(n: int) -> int:\n    t = 0\n    while True:\n        t = t + n\n    return t\ndef ok(n):\n    t = 0\n    for i in range(n):\n        t = t + i\n    return t\n").unwrap();
    let lim = Limits { max_steps: 10_000, max_ms: None };
    assert!(p.call_limited("f", &[], &lim).unwrap_err().contains("LimitError"));
    assert!(p.call_limited("g", &[Value::Int(1)], &lim).unwrap_err().contains("LimitError"));
    assert_eq!(p.call_limited("ok", &[Value::Int(100)], &lim).unwrap(), Value::Int(4950));
}

#[test]
fn time_limit_stops_runaway() {
    use rmojo::Limits;
    let p = compile_source("def f(n: int) -> int:\n    t = 0\n    while True:\n        t = t + n\n    return t\ndef h():\n    xs = []\n    while True:\n        xs.append(1)\n    return 0\n").unwrap();
    let lim = Limits { max_steps: u64::MAX, max_ms: Some(100) };
    let t = std::time::Instant::now();
    assert!(p.call_limited("f", &[Value::Int(1)], &lim).unwrap_err().contains("time limit"));
    assert!(p.call_limited("h", &[], &lim).unwrap_err().contains("time limit"));
    assert!(t.elapsed().as_secs() < 5);
}
