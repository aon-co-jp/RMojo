use rmojo::{compile_source, Value};
#[test]
fn call_scalar() {
    let p = compile_source("def add(a: int, b: int) -> int:\n    return a + b\ndef hi(n):\n    return 'hi ' + n\n").unwrap();
    assert_eq!(p.call("add", &[Value::Int(2), Value::Int(3)]).unwrap(), Value::Int(5));
    assert_eq!(p.call("hi", &[Value::Str("x".into())]).unwrap(), Value::Str("hi x".into()));
    assert!(p.call("nope", &[]).is_err());
    assert!(p.call("add", &[Value::Int(1)]).is_err());
}
