//! RMojo — a Mojo-style language: Python syntax on the surface, a memory-safe Rust implementation inside (no C++, no `unsafe`).
//! Pipeline: Python source -> rustpython-parser AST -> resolved IR (locals/globals/functions as indices) -> fast interpreter.
//! Supported subset (v0.1): def (typed or untyped), int/float/bool/str, arithmetic, comparisons, and/or/not, if/elif/else, while,
//! for-in-range, break/continue/return, recursion, module-level globals, print/int/float/str/bool/abs/min/max/len.
//! Safety: no `unsafe`; integer overflow is a runtime error; recursion depth and the call stack are bounded.
#![forbid(unsafe_code)]

use rustpython_parser::ast::{self, Constant, Expr, Stmt};
use rustpython_parser::Parse;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering as AOrd};

/// Mojo mode (.mojo files): assignment from an lvalue copies lists/dicts/structs (value semantics).
pub(crate) static COPY_MODE: AtomicBool = AtomicBool::new(false);

fn mc(e: E) -> E {
    if COPY_MODE.load(AOrd::Relaxed) && matches!(e, E::Local(_) | E::Global(_) | E::Attr(..) | E::Index(..)) {
        E::Builtin("__copy", vec![e])
    } else {
        e
    }
}

fn dedup(items: Vec<V>) -> Vec<V> {
    let mut out: Vec<V> = Vec::new();
    for it in items {
        if !out.iter().any(|e| veq(e, &it)) {
            out.push(it);
        }
    }
    out
}

fn deep_copy(v: &V) -> V {
    match v {
        V::L(l) => V::L(Rc::new(std::cell::RefCell::new(l.borrow().iter().map(deep_copy).collect()))),
        V::D(d) => V::D(Rc::new(std::cell::RefCell::new(d.borrow().iter().map(|(k, x)| (k.clone(), deep_copy(x))).collect()))),
        V::Z(l) => V::Z(Rc::new(std::cell::RefCell::new(l.borrow().iter().map(deep_copy).collect()))),
        V::O(o) => V::O(Rc::new(ObjBox { cls: o.cls, f: std::cell::RefCell::new(o.f.borrow().iter().map(deep_copy).collect()) })),
        V::T(t) => V::T(Rc::new(t.iter().map(deep_copy).collect())),
        other => other.clone(),
    }
}

const MAX_DEPTH: usize = 20_000;

#[derive(Clone, Debug)]
enum V {
    I(i64),
    F(f64),
    B(bool),
    S(Rc<String>),
    O(Rc<ObjBox>),
    L(Rc<std::cell::RefCell<Vec<V>>>),
    D(Rc<std::cell::RefCell<Vec<(V, V)>>>),
    Z(Rc<std::cell::RefCell<Vec<V>>>),
    X(Rc<Vec<V>>),
    T(Rc<Vec<V>>),
    N,
}

/// A struct instance (reference semantics, like a Python object).
#[derive(Debug)]
struct ObjBox {
    cls: usize,
    f: std::cell::RefCell<Vec<V>>,
}

struct Class {
    fields: Vec<String>,
    methods: HashMap<String, usize>,
    init: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Bop {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
}

#[derive(Clone, Copy, Debug)]
enum Cop {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug)]
enum E {
    Const(V),
    Local(usize),
    Global(usize),
    Bin(Bop, Box<E>, Box<E>),
    Neg(Box<E>),
    Not(Box<E>),
    Cmp(Box<E>, Vec<(Cop, E)>),
    And(Vec<E>),
    Or(Vec<E>),
    If(Box<E>, Box<E>, Box<E>),
    Call(usize, Vec<E>),
    Builtin(&'static str, Vec<E>),
    New(usize, Vec<E>),
    Attr(Box<E>, String),
    Method(Box<E>, String, Vec<E>),
    List(Vec<E>),
    Dict(Vec<(E, E)>),
    Index(Box<E>, Box<E>),
    In(Box<E>, Box<E>, bool),
    Tuple(Vec<E>),
    Comp(Vec<(Vec<Target>, E, Vec<E>)>, Box<E>),
    Fmt(Box<E>, String),
    Concat(Vec<E>),
    ToDict(Box<E>),
    ToSet(Box<E>),
    Slice(Box<E>, Option<Box<E>>, Option<Box<E>>, Option<Box<E>>),
}

#[derive(Clone, Debug)]
enum St {
    SetLocal(usize, E),
    SetGlobal(usize, E),
    SetAttr(E, String, E),
    SetIndex(E, E, E),
    ForIn(Vec<Target>, E, Vec<St>),
    Unpack(Vec<Target>, E),
    Try(Vec<St>, Vec<(Vec<String>, Option<Target>, Vec<St>)>, Vec<St>, Vec<St>),
    Raise(E),
    Multi(Vec<Target>, Vec<E>),
    Expr(E),
    If(E, Vec<St>, Vec<St>),
    While(E, Vec<St>),
    LoopElse(Box<St>, Vec<St>),
    ForRange(Target, E, E, E, Vec<St>),
    Return(Option<E>),
    Break,
    Continue,
}

#[derive(Clone, Copy, Debug)]
enum Target {
    Local(usize),
    Global(usize),
}

struct Func {
    nparams: usize,
    nlocals: usize,
    body: Vec<St>,
}

struct Program {
    funcs: Vec<Func>,
    main_body: Vec<St>,
    nglobals: usize,
    entry: Option<usize>,
    ifuncs: Vec<Option<IFunc>>,
    classes: Vec<Class>,
    exc_parent: HashMap<String, String>,
    names: HashMap<String, usize>,
}

/// Boxed error (keeps `Result<V, Er>` small so the hot path stays in registers).
struct Er(Box<String>);
impl From<&str> for Er {
    fn from(s: &str) -> Er {
        Er(Box::new(s.to_string()))
    }
}
impl From<String> for Er {
    fn from(s: String) -> Er {
        Er(Box::new(s))
    }
}
impl std::fmt::Display for Er {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
type R<T> = Result<T, Er>;

// ---------------------------------------------------------------- compile (AST -> IR)

struct Scope<'a> {
    funcs: &'a HashMap<String, usize>,
    classes: &'a HashMap<String, usize>,
    globals: &'a mut HashMap<String, usize>,
    locals: Option<HashMap<String, usize>>, // None at module level
}

impl<'a> Scope<'a> {
    fn lookup_set(&mut self, name: &str) -> Target {
        match &mut self.locals {
            Some(l) => {
                let n = l.len();
                Target::Local(*l.entry(name.to_string()).or_insert(n))
            }
            None => {
                let n = self.globals.len();
                Target::Global(*self.globals.entry(name.to_string()).or_insert(n))
            }
        }
    }
}

fn cexpr(e: &Expr, sc: &mut Scope) -> R<E> {
    Ok(match e {
        Expr::Constant(c) => E::Const(match &c.value {
            Constant::Int(i) => V::I(i.to_string().parse::<i64>().map_err(|_| "integer literal out of range (64-bit)".to_string())?),
            Constant::Float(f) => V::F(*f),
            Constant::Bool(b) => V::B(*b),
            Constant::Str(s) => V::S(Rc::new(s.to_string())),
            Constant::None => V::N,
            _ => return Err("unsupported literal".into()),
        }),
        Expr::Name(n) => {
            let id = n.id.as_str();
            if let Some(l) = &sc.locals {
                if let Some(&i) = l.get(id) {
                    return Ok(E::Local(i));
                }
            }
            if let Some(&g) = sc.globals.get(id) {
                return Ok(E::Global(g));
            }
            return Err(format!("name '{id}' is not defined").into());
        }
        Expr::BinOp(b) => {
            let op = match b.op {
                ast::Operator::Add => Bop::Add,
                ast::Operator::Sub => Bop::Sub,
                ast::Operator::Mult => Bop::Mul,
                ast::Operator::Div => Bop::Div,
                ast::Operator::FloorDiv => Bop::FloorDiv,
                ast::Operator::Mod => Bop::Mod,
                ast::Operator::Pow => Bop::Pow,
                ast::Operator::BitOr | ast::Operator::BitAnd | ast::Operator::BitXor | ast::Operator::LShift | ast::Operator::RShift => {
                    let c = match b.op {
                        ast::Operator::BitOr => "|",
                        ast::Operator::BitAnd => "&",
                        ast::Operator::BitXor => "^",
                        ast::Operator::LShift => "<<",
                        _ => ">>",
                    };
                    return Ok(E::Builtin("__setop", vec![E::Const(V::S(Rc::new(c.to_string()))), cexpr(&b.left, sc)?, cexpr(&b.right, sc)?]));
                }
                _ => return Err("unsupported operator".into()),
            };
            E::Bin(op, Box::new(cexpr(&b.left, sc)?), Box::new(cexpr(&b.right, sc)?))
        }
        Expr::UnaryOp(u) => match u.op {
            ast::UnaryOp::USub => E::Neg(Box::new(cexpr(&u.operand, sc)?)),
            ast::UnaryOp::UAdd => cexpr(&u.operand, sc)?,
            ast::UnaryOp::Not => E::Not(Box::new(cexpr(&u.operand, sc)?)),
            _ => return Err("unsupported unary operator".into()),
        },
        Expr::Compare(c) if c.ops.len() == 1 && matches!(c.ops[0], ast::CmpOp::In | ast::CmpOp::NotIn) => {
            E::In(Box::new(cexpr(&c.left, sc)?), Box::new(cexpr(&c.comparators[0], sc)?), matches!(c.ops[0], ast::CmpOp::NotIn))
        }
        Expr::Compare(c) => {
            let mut rest = Vec::new();
            for (op, r) in c.ops.iter().zip(c.comparators.iter()) {
                let o = match op {
                    ast::CmpOp::Eq => Cop::Eq,
                    ast::CmpOp::NotEq => Cop::Ne,
                    ast::CmpOp::Lt => Cop::Lt,
                    ast::CmpOp::LtE => Cop::Le,
                    ast::CmpOp::Gt => Cop::Gt,
                    ast::CmpOp::GtE => Cop::Ge,
                    _ => return Err("unsupported comparison".into()),
                };
                rest.push((o, cexpr(r, sc)?));
            }
            E::Cmp(Box::new(cexpr(&c.left, sc)?), rest)
        }
        Expr::BoolOp(b) => {
            let vs = b.values.iter().map(|v| cexpr(v, sc)).collect::<R<Vec<_>>>()?;
            match b.op {
                ast::BoolOp::And => E::And(vs),
                ast::BoolOp::Or => E::Or(vs),
            }
        }
        Expr::IfExp(i) => E::If(Box::new(cexpr(&i.test, sc)?), Box::new(cexpr(&i.body, sc)?), Box::new(cexpr(&i.orelse, sc)?)),
        Expr::List(l) => E::List(l.elts.iter().map(|x| cexpr(x, sc)).collect::<R<Vec<_>>>()?),
        Expr::Dict(d) => {
            let mut segs: Vec<E> = Vec::new();
            let mut kv = Vec::new();
            for (k, v) in d.keys.iter().zip(d.values.iter()) {
                match k {
                    Some(k) => kv.push((cexpr(k, sc)?, cexpr(v, sc)?)),
                    None => {
                        if !kv.is_empty() {
                            segs.push(E::Dict(std::mem::take(&mut kv)));
                        }
                        segs.push(cexpr(v, sc)?);
                    }
                }
            }
            if segs.is_empty() {
                E::Dict(kv)
            } else {
                if !kv.is_empty() {
                    segs.push(E::Dict(kv));
                }
                E::Builtin("__dmerge", segs)
            }
        }
        Expr::Set(st) => E::ToSet(Box::new(E::List(st.elts.iter().map(|x| cexpr(x, sc)).collect::<R<Vec<_>>>()?))),
        Expr::SetComp(sc_) => {
            let mut gens = Vec::new();
            for g in &sc_.generators {
                if g.is_async {
                    return Err("async comprehension is not supported".into());
                }
                let mut tg = Vec::new();
                match &g.target {
                    Expr::Name(n) => tg.push(sc.lookup_set(n.id.as_str())),
                    Expr::Tuple(tt) => {
                        for e in &tt.elts {
                            let Expr::Name(n) = e else { return Err("comprehension target must be names".into()) };
                            tg.push(sc.lookup_set(n.id.as_str()));
                        }
                    }
                    _ => return Err("comprehension target must be a name".into()),
                }
                let it = cexpr(&g.iter, sc)?;
                let conds = g.ifs.iter().map(|c| cexpr(c, sc)).collect::<R<Vec<_>>>()?;
                gens.push((tg, it, conds));
            }
            E::ToSet(Box::new(E::Comp(gens, Box::new(cexpr(&sc_.elt, sc)?))))
        }
        Expr::ListComp(lc) => {
            let mut gens = Vec::new();
            for g in &lc.generators {
                if g.is_async {
                    return Err("async comprehension is not supported".into());
                }
                let mut tg = Vec::new();
                match &g.target {
                    Expr::Name(n) => tg.push(sc.lookup_set(n.id.as_str())),
                    Expr::Tuple(tt) => {
                        for e in &tt.elts {
                            let Expr::Name(n) = e else { return Err("comprehension target must be names".into()) };
                            tg.push(sc.lookup_set(n.id.as_str()));
                        }
                    }
                    _ => return Err("comprehension target must be a name".into()),
                }
                let it = cexpr(&g.iter, sc)?;
                let conds = g.ifs.iter().map(|c| cexpr(c, sc)).collect::<R<Vec<_>>>()?;
                gens.push((tg, it, conds));
            }
            E::Comp(gens, Box::new(cexpr(&lc.elt, sc)?))
        }
        Expr::JoinedStr(j) => {
            let mut parts = Vec::new();
            for v in &j.values {
                match v {
                    Expr::FormattedValue(fv) => {
                        let mut dynspec: Option<E> = None;
                        let spec = match &fv.format_spec {
                            Some(b) => match &**b {
                                Expr::JoinedStr(js) => {
                                    let mut t = String::new();
                                    let mut pieces: Vec<E> = Vec::new();
                                    let mut nested = false;
                                    for x in &js.values {
                                        if let Expr::Constant(c) = x {
                                            if let ast::Constant::Str(st) = &c.value {
                                                t.push_str(st);
                                                pieces.push(E::Const(V::S(Rc::new(st.to_string()))));
                                            }
                                        } else if let Expr::FormattedValue(inner) = x {
                                            nested = true;
                                            pieces.push(E::Fmt(Box::new(cexpr(&inner.value, sc)?), String::new()));
                                        } else {
                                            return Err("nested format spec is not supported".into());
                                        }
                                    }
                                    if nested {
                                        dynspec = Some(E::Concat(pieces));
                                    }
                                    t
                                }
                                _ => String::new(),
                            },
                            None => String::new(),
                        };
                        match dynspec {
                            Some(d) => parts.push(E::Builtin("__fmtdyn", vec![cexpr(&fv.value, sc)?, d])),
                            None => parts.push(E::Fmt(Box::new(cexpr(&fv.value, sc)?), spec)),
                        }
                    }
                    other => parts.push(cexpr(other, sc)?),
                }
            }
            E::Concat(parts)
        }
        Expr::DictComp(dc) => {
            let mut gens = Vec::new();
            for g in &dc.generators {
                if g.is_async {
                    return Err("async comprehension is not supported".into());
                }
                let mut tg = Vec::new();
                match &g.target {
                    Expr::Name(n) => tg.push(sc.lookup_set(n.id.as_str())),
                    Expr::Tuple(tt) => {
                        for e in &tt.elts {
                            let Expr::Name(n) = e else { return Err("comprehension target must be names".into()) };
                            tg.push(sc.lookup_set(n.id.as_str()));
                        }
                    }
                    _ => return Err("comprehension target must be a name".into()),
                }
                let it = cexpr(&g.iter, sc)?;
                let conds = g.ifs.iter().map(|c| cexpr(c, sc)).collect::<R<Vec<_>>>()?;
                gens.push((tg, it, conds));
            }
            let pair = E::Tuple(vec![cexpr(&dc.key, sc)?, cexpr(&dc.value, sc)?]);
            E::ToDict(Box::new(E::Comp(gens, Box::new(pair))))
        }
        Expr::Tuple(t) => E::Tuple(t.elts.iter().map(|x| cexpr(x, sc)).collect::<R<Vec<_>>>()?),
        Expr::Subscript(sb) if matches!(&*sb.slice, Expr::Slice(_)) => {
            let Expr::Slice(sl) = &*sb.slice else { unreachable!() };
            let mut o = |x: &Option<Box<Expr>>| -> R<Option<Box<E>>> {
                Ok(match x {
                    Some(e) => Some(Box::new(cexpr(e, sc)?)),
                    None => None,
                })
            };
            let (lo, hi, st) = (o(&sl.lower)?, o(&sl.upper)?, o(&sl.step)?);
            E::Slice(Box::new(cexpr(&sb.value, sc)?), lo, hi, st)
        }
        Expr::Subscript(sb) => {
            E::Index(Box::new(cexpr(&sb.value, sc)?), Box::new(cexpr(&sb.slice, sc)?))
        }
        Expr::Attribute(a) => E::Attr(Box::new(cexpr(&a.value, sc)?), a.attr.to_string()),
        Expr::Call(c) if matches!(&*c.func, Expr::Attribute(_)) => {
            let Expr::Attribute(a) = &*c.func else { unreachable!() };
            let args = c.args.iter().map(|x| cexpr(x, sc)).collect::<R<Vec<_>>>()?;
            E::Method(Box::new(cexpr(&a.value, sc)?), a.attr.to_string(), args)
        }
        Expr::Call(c) => {
            let Expr::Name(f) = &*c.func else { return Err("only direct calls of named functions are supported".into()) };
            let args = c.args.iter().map(|a| cexpr(a, sc)).collect::<R<Vec<_>>>()?;
            let name = f.id.as_str();
            if let Some(&ci) = sc.classes.get(name) {
                return Ok(E::New(ci, args));
            }
            if let Some(&fi) = sc.funcs.get(name) {
                return Ok(E::Call(fi, args));
            }
            let b: &'static str = match name {
                "print" => "print",
                "int" => "int",
                "float" => "float",
                "str" => "str",
                "bool" => "bool",
                "abs" => "abs",
                "simd" => "simd",
                "range" => "range",
                "Error" => "Error",
                "sqrt" => "sqrt",
                "sin" => "sin",
                "cos" => "cos",
                "tan" => "tan",
                "exp" => "exp",
                "log" => "log",
                "floor" => "floor",
                "ceil" => "ceil",
                "sum" => "sum",
                "sorted" => "sorted",
                "list" => "list",
                "set" => "set",
                "min" => "min",
                "max" => "max",
                "len" => "len",
                _ => return Err(format!("name '{name}' is not defined").into()),
            };
            E::Builtin(b, args)
        }
        _ => return Err("unsupported expression".into()),
    })
}

fn cblock(body: &[Stmt], sc: &mut Scope) -> R<Vec<St>> {
    let mut out = Vec::new();
    for s in body {
        match s {
            Stmt::Expr(e) => out.push(St::Expr(cexpr(&e.value, sc)?)),
            Stmt::Pass(_) => {}
            Stmt::Break(_) => out.push(St::Break),
            Stmt::Continue(_) => out.push(St::Continue),
            Stmt::Return(r) => out.push(St::Return(match &r.value {
                Some(v) => Some(cexpr(v, sc)?),
                None => None,
            })),
            Stmt::Assign(a) => {
                if a.targets.len() != 1 {
                    // a = b = v : evaluate v once into the first name, then copy it to the others
                    let mut tg = Vec::new();
                    for t in &a.targets {
                        let Expr::Name(n) = t else { return Err("chained assignment supports names only".into()) };
                        tg.push(sc.lookup_set(n.id.as_str()));
                    }
                    let v = cexpr(&a.value, sc)?;
                    let first = match tg[0] {
                        Target::Local(i) => (St::SetLocal(i, v), E::Local(i)),
                        Target::Global(i) => (St::SetGlobal(i, v), E::Global(i)),
                    };
                    out.push(first.0);
                    for t in &tg[1..] {
                        out.push(match *t {
                            Target::Local(i) => St::SetLocal(i, first.1.clone()),
                            Target::Global(i) => St::SetGlobal(i, first.1.clone()),
                        });
                    }
                    continue;
                }
                if let (Expr::Tuple(tt), false) = (&a.targets[0], matches!(&*a.value, Expr::Tuple(_))) {
                    let mut tg = Vec::new();
                    for t in &tt.elts {
                        let Expr::Name(n) = t else { return Err("only simple names can be assigned".into()) };
                        tg.push(sc.lookup_set(n.id.as_str()));
                    }
                    out.push(St::Unpack(tg, cexpr(&a.value, sc)?));
                    continue;
                }
                if let (Expr::Tuple(tt), Expr::Tuple(vt)) = (&a.targets[0], &*a.value) {
                    if tt.elts.len() != vt.elts.len() {
                        return Err("tuple assignment: length mismatch".into());
                    }
                    let vals = vt.elts.iter().map(|x| cexpr(x, sc)).collect::<R<Vec<_>>>()?;
                    let mut tg = Vec::new();
                    for t in &tt.elts {
                        let Expr::Name(n) = t else { return Err("only simple names can be assigned".into()) };
                        tg.push(sc.lookup_set(n.id.as_str()));
                    }
                    out.push(St::Multi(tg, vals));
                    continue;
                }
                if let Expr::Subscript(sb) = &a.targets[0] {
                    let v = mc(cexpr(&a.value, sc)?);
                    out.push(St::SetIndex(cexpr(&sb.value, sc)?, cexpr(&sb.slice, sc)?, v));
                    continue;
                }
                if let Expr::Attribute(at) = &a.targets[0] {
                    let o = cexpr(&at.value, sc)?;
                    let v = mc(cexpr(&a.value, sc)?);
                    out.push(St::SetAttr(o, at.attr.to_string(), v));
                    continue;
                }
                let Expr::Name(n) = &a.targets[0] else { return Err("only simple names can be assigned".into()) };
                let v = mc(cexpr(&a.value, sc)?);
                out.push(match sc.lookup_set(n.id.as_str()) {
                    Target::Local(i) => St::SetLocal(i, v),
                    Target::Global(i) => St::SetGlobal(i, v),
                });
            }
            Stmt::AnnAssign(a) => {
                let Expr::Name(n) = &*a.target else { return Err("only simple names can be assigned".into()) };
                if let Some(v) = &a.value {
                    let v = mc(cexpr(v, sc)?);
                    out.push(match sc.lookup_set(n.id.as_str()) {
                        Target::Local(i) => St::SetLocal(i, v),
                        Target::Global(i) => St::SetGlobal(i, v),
                    });
                } else {
                    sc.lookup_set(n.id.as_str());
                }
            }
            Stmt::AugAssign(a) => {
                if let Expr::Subscript(sb) = &*a.target {
                    let op = match a.op {
                        ast::Operator::Add => Bop::Add,
                        ast::Operator::Sub => Bop::Sub,
                        ast::Operator::Mult => Bop::Mul,
                        ast::Operator::Div => Bop::Div,
                        ast::Operator::FloorDiv => Bop::FloorDiv,
                        ast::Operator::Mod => Bop::Mod,
                        _ => return Err("unsupported operator".into()),
                    };
                    let cur = E::Index(Box::new(cexpr(&sb.value, sc)?), Box::new(cexpr(&sb.slice, sc)?));
                    let v = E::Bin(op, Box::new(cur), Box::new(cexpr(&a.value, sc)?));
                    out.push(St::SetIndex(cexpr(&sb.value, sc)?, cexpr(&sb.slice, sc)?, v));
                    continue;
                }
                if let Expr::Attribute(at) = &*a.target {
                    let op = match a.op {
                        ast::Operator::Add => Bop::Add,
                        ast::Operator::Sub => Bop::Sub,
                        ast::Operator::Mult => Bop::Mul,
                        ast::Operator::Div => Bop::Div,
                        ast::Operator::FloorDiv => Bop::FloorDiv,
                        ast::Operator::Mod => Bop::Mod,
                        ast::Operator::Pow => Bop::Pow,
                        _ => return Err("unsupported operator".into()),
                    };
                    let cur = E::Attr(Box::new(cexpr(&at.value, sc)?), at.attr.to_string());
                    let v = E::Bin(op, Box::new(cur), Box::new(cexpr(&a.value, sc)?));
                    out.push(St::SetAttr(cexpr(&at.value, sc)?, at.attr.to_string(), v));
                    continue;
                }
                let Expr::Name(n) = &*a.target else { return Err("only simple names can be assigned".into()) };
                let op = match a.op {
                    ast::Operator::Add => Bop::Add,
                    ast::Operator::Sub => Bop::Sub,
                    ast::Operator::Mult => Bop::Mul,
                    ast::Operator::Div => Bop::Div,
                    ast::Operator::FloorDiv => Bop::FloorDiv,
                    ast::Operator::Mod => Bop::Mod,
                    ast::Operator::Pow => Bop::Pow,
                    _ => return Err("unsupported operator".into()),
                };
                let cur = cexpr(&Expr::Name(ast::ExprName { range: a.range, id: n.id.clone(), ctx: ast::ExprContext::Load }), sc)?;
                let v = E::Bin(op, Box::new(cur), Box::new(cexpr(&a.value, sc)?));
                out.push(match sc.lookup_set(n.id.as_str()) {
                    Target::Local(i) => St::SetLocal(i, v),
                    Target::Global(i) => St::SetGlobal(i, v),
                });
            }
            Stmt::Raise(r) => match &r.exc {
                Some(e) => {
                    // `raise Name(msg)` -> message "Name: msg"; `raise Name` -> "Name"
                    let (name, arg) = match &**e {
                        Expr::Call(c) => (
                            match &*c.func {
                                Expr::Name(n) => n.id.to_string(),
                                _ => "Exception".to_string(),
                            },
                            match c.args.first() {
                                Some(a) => Some(cexpr(a, sc)?),
                                None => None,
                            },
                        ),
                        Expr::Name(n) => (n.id.to_string(), None),
                        _ => return Err("unsupported raise".into()),
                    };
                    out.push(St::Raise(E::Builtin("__raise", vec![E::Const(V::S(Rc::new(name))), arg.unwrap_or(E::Const(V::N))])));
                }
                None => out.push(St::Raise(E::Builtin("__reraise", vec![]))),
            },
            Stmt::Try(t) => {
                let body = cblock(&t.body, sc)?;
                let mut hs = Vec::new();
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    let tg = match &h.name {
                        Some(n) => Some(sc.lookup_set(n.as_str())),
                        None => None,
                    };
                    let mut tys: Vec<String> = Vec::new();
                    match h.type_.as_deref() {
                        Some(Expr::Name(n)) => tys.push(n.id.to_string()),
                        Some(Expr::Tuple(tt)) => {
                            for e in &tt.elts {
                                if let Expr::Name(n) = e {
                                    tys.push(n.id.to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                    hs.push((tys, tg, cblock(&h.body, sc)?));
                }
                let els = cblock(&t.orelse, sc)?;
                out.push(St::Try(body, hs, els, cblock(&t.finalbody, sc)?));
            }
            Stmt::If(i) => out.push(St::If(cexpr(&i.test, sc)?, cblock(&i.body, sc)?, cblock(&i.orelse, sc)?)),
            Stmt::While(w) => {
                let lp = St::While(cexpr(&w.test, sc)?, cblock(&w.body, sc)?);
                if w.orelse.is_empty() {
                    out.push(lp);
                } else {
                    out.push(St::LoopElse(Box::new(lp), cblock(&w.orelse, sc)?));
                }
            }
            Stmt::For(f) => {
                let start_len = out.len();
                let orelse = if f.orelse.is_empty() { Vec::new() } else { cblock(&f.orelse, sc)? };
                let _ = start_len;
                let is_range = matches!(&*f.iter, Expr::Call(c) if matches!(&*c.func, Expr::Name(n) if n.id.as_str() == "range"));
                if !is_range {
                    let mut tg = Vec::new();
                    match &*f.target {
                        Expr::Name(n) => tg.push(sc.lookup_set(n.id.as_str())),
                        Expr::Tuple(tt) => {
                            for e in &tt.elts {
                                let Expr::Name(n) = e else { return Err("for target must be names".into()) };
                                tg.push(sc.lookup_set(n.id.as_str()));
                            }
                        }
                        _ => return Err("for target must be a name".into()),
                    }
                    let it = cexpr(&f.iter, sc)?;
                    let lp = St::ForIn(tg, it, cblock(&f.body, sc)?);
                    out.push(if orelse.is_empty() { lp } else { St::LoopElse(Box::new(lp), orelse) });
                    continue;
                }
                let Expr::Name(t) = &*f.target else { return Err("for target must be a name".into()) };
                let Expr::Call(c) = &*f.iter else { return Err("unreachable".into()) };
                if c.args.is_empty() || c.args.len() > 3 {
                    return Err("range() takes 1 to 3 arguments".into());
                }
                let a: Vec<E> = c.args.iter().map(|x| cexpr(x, sc)).collect::<R<Vec<_>>>()?;
                let (start, stop, step) = match a.len() {
                    1 => (E::Const(V::I(0)), a[0].clone(), E::Const(V::I(1))),
                    2 => (a[0].clone(), a[1].clone(), E::Const(V::I(1))),
                    _ => (a[0].clone(), a[1].clone(), a[2].clone()),
                };
                let tgt = sc.lookup_set(t.id.as_str());
                let lp = St::ForRange(tgt, start, stop, step, cblock(&f.body, sc)?);
                out.push(if orelse.is_empty() { lp } else { St::LoopElse(Box::new(lp), orelse) });
            }
            _ => return Err("unsupported statement".into()),
        }
    }
    Ok(out)
}

fn compile(src: &str) -> R<Program> {
    let suite = ast::Suite::parse(src, "<rmojo>").map_err(|e| Er::from(format!("SyntaxError: {e}")))?;
    // pass 1: function names
    let mut fnames: HashMap<String, usize> = HashMap::new();
    for s in &suite {
        if let Stmt::FunctionDef(f) = s {
            let n = fnames.len();
            fnames.insert(f.name.to_string(), n);
        }
    }
    let mut cnames: HashMap<String, usize> = HashMap::new();
    let mut cdefs: Vec<&ast::StmtClassDef> = Vec::new();
    for s in &suite {
        if let Stmt::ClassDef(c) = s {
            cnames.insert(c.name.to_string(), cdefs.len());
            cdefs.push(c);
        }
    }
    // exception hierarchy of user classes: class Name(Base) -> parent map (builtin bases such as Exception end the chain)
    let mut exc_parent: HashMap<String, String> = HashMap::new();
    for c in &cdefs {
        if let Some(Expr::Name(b)) = c.bases.first() {
            exc_parent.insert(c.name.to_string(), b.id.to_string());
        }
    }
    let mut globals: HashMap<String, usize> = HashMap::new();
    // pass 2: module-level assignments define globals (so functions can read them); collect in order
    let mut main_stmts: Vec<&Stmt> = Vec::new();
    let mut fdefs: Vec<&ast::StmtFunctionDef> = Vec::new();
    for s in &suite {
        match s {
            Stmt::FunctionDef(f) => fdefs.push(f),
            Stmt::ClassDef(_) => {}
            other => main_stmts.push(other),
        }
    }
    // declare module-level names first (targets of assignments/for)
    fn declare(stmts: &[Stmt], g: &mut HashMap<String, usize>) {
        for s in stmts {
            match s {
                Stmt::Assign(a) => {
                    for t in &a.targets {
                        match t {
                            Expr::Name(n) => {
                                let k = g.len();
                                g.entry(n.id.to_string()).or_insert(k);
                            }
                            Expr::Tuple(tt) => {
                                for e in &tt.elts {
                                    if let Expr::Name(n) = e {
                                        let k = g.len();
                                        g.entry(n.id.to_string()).or_insert(k);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Stmt::AnnAssign(a) => {
                    if let Expr::Name(n) = &*a.target {
                        let k = g.len();
                        g.entry(n.id.to_string()).or_insert(k);
                    }
                }
                Stmt::AugAssign(a) => {
                    if let Expr::Name(n) = &*a.target {
                        let k = g.len();
                        g.entry(n.id.to_string()).or_insert(k);
                    }
                }
                Stmt::For(f) => {
                    if let Expr::Name(n) = &*f.target {
                        let k = g.len();
                        g.entry(n.id.to_string()).or_insert(k);
                    }
                    if let Expr::Tuple(tt) = &*f.target {
                        for e in &tt.elts {
                            if let Expr::Name(n) = e {
                                let k = g.len();
                                g.entry(n.id.to_string()).or_insert(k);
                            }
                        }
                    }
                    declare(&f.body, g);
                }
                Stmt::If(i) => {
                    declare(&i.body, g);
                    declare(&i.orelse, g);
                }
                Stmt::While(w) => declare(&w.body, g),
                _ => {}
            }
        }
    }
    let owned: Vec<Stmt> = main_stmts.iter().map(|s| (*s).clone()).collect();
    declare(&owned, &mut globals);

    let mut funcs: Vec<Func> = Vec::new();
    for f in &fdefs {
        let mut locals: HashMap<String, usize> = HashMap::new();
        for a in f.args.args.iter() {
            let n = locals.len();
            locals.insert(a.def.arg.to_string(), n);
        }
        let nparams = locals.len();
        let mut sc = Scope { funcs: &fnames, classes: &cnames, globals: &mut globals, locals: Some(locals) };
        let body = cblock(&f.body, &mut sc)?;
        let nlocals = sc.locals.as_ref().map(|l| l.len()).unwrap_or(0);
        funcs.push(Func { nparams, nlocals, body });
    }
    // struct/class methods: appended after the top-level functions
    fn self_attrs(stmts: &[Stmt], out: &mut Vec<String>) {
        for st in stmts {
            match st {
                Stmt::Assign(a) => {
                    for t in &a.targets {
                        if let Expr::Attribute(at) = t {
                            if !out.contains(&at.attr.to_string()) {
                                out.push(at.attr.to_string());
                            }
                        }
                    }
                }
                Stmt::If(i) => {
                    self_attrs(&i.body, out);
                    self_attrs(&i.orelse, out);
                }
                Stmt::While(w) => self_attrs(&w.body, out),
                Stmt::For(f) => self_attrs(&f.body, out),
                _ => {}
            }
        }
    }
    let mut classes: Vec<Class> = Vec::new();
    let mut mdefs: Vec<(usize, &ast::StmtFunctionDef)> = Vec::new();
    for (ci, c) in cdefs.iter().enumerate() {
        let mut fields: Vec<String> = Vec::new();
        for st in &c.body {
            if let Stmt::AnnAssign(a) = st {
                if let Expr::Name(n) = &*a.target {
                    fields.push(n.id.to_string());
                }
            }
        }
        let mut methods = HashMap::new();
        let mut init = None;
        for st in &c.body {
            if let Stmt::FunctionDef(f) = st {
                self_attrs(&f.body, &mut fields);
                let fi = fdefs.len() + mdefs.len();
                if f.name.as_str() == "__init__" {
                    init = Some(fi);
                }
                methods.insert(f.name.to_string(), fi);
                mdefs.push((ci, f));
            }
        }
        classes.push(Class { fields, methods, init });
    }
    for (_, f) in &mdefs {
        let mut locals: HashMap<String, usize> = HashMap::new();
        for a in f.args.args.iter() {
            let n = locals.len();
            locals.insert(a.def.arg.to_string(), n);
        }
        let nparams = locals.len();
        let mut sc = Scope { funcs: &fnames, classes: &cnames, globals: &mut globals, locals: Some(locals) };
        let body = cblock(&f.body, &mut sc)?;
        let nlocals = sc.locals.as_ref().map(|l| l.len()).unwrap_or(0);
        funcs.push(Func { nparams, nlocals, body });
    }
    let mut sc = Scope { funcs: &fnames, classes: &cnames, globals: &mut globals, locals: None };
    let main_body = cblock(&owned, &mut sc)?;
    let entry = fnames.get("main").copied();
    // typed int fast path: fixpoint over functions whose signatures are `int -> int` and whose bodies stay inside the int subset
    let mut cand: HashMap<String, usize> = HashMap::new();
    for f in &fdefs {
        if typed_signature(f) {
            cand.insert(f.name.to_string(), fnames[f.name.as_str()]);
        }
    }
    let mut ifuncs: Vec<Option<IFunc>>;
    loop {
        ifuncs = (0..fdefs.len()).map(|_| None).collect();
        let mut removed = false;
        for f in &fdefs {
            if cand.contains_key(f.name.as_str()) {
                match compile_typed(f, &cand) {
                    Some(t) => ifuncs[fnames[f.name.as_str()]] = Some(t),
                    None => {
                        cand.remove(f.name.as_str());
                        removed = true;
                    }
                }
            }
        }
        if !removed {
            break;
        }
    }
    ifuncs.resize_with(funcs.len(), || None);
    Ok(Program { funcs, main_body, nglobals: globals.len(), entry, ifuncs, classes, exc_parent, names: fnames })
}

// ---------------------------------------------------------------- typed int fast path
// Functions annotated `def f(a: int, ...) -> int` whose bodies only use ints are compiled to an unboxed i64 IR (no tags, no Rc).
// This is the Mojo-style "typed = fast" path. Anything outside the subset silently falls back to the generic interpreter.

#[derive(Debug)]
enum IE {
    C(i64),
    L(usize),
    Bin(Bop, Box<IE>, Box<IE>),
    BinLC(Bop, usize, i64),
    BinLL(Bop, usize, usize),
    Neg(Box<IE>),
    Call(usize, Vec<IE>),
    If(Box<IC>, Box<IE>, Box<IE>),
}
#[derive(Debug)]
enum IC {
    Cmp(Cop, Box<IE>, Box<IE>),
    CmpC(Cop, Box<IE>, i64),
    CmpLC(Cop, usize, i64),
    CmpLL(Cop, usize, usize),
    And(Vec<IC>),
    Or(Vec<IC>),
    Not(Box<IC>),
    Truth(Box<IE>),
}
#[derive(Debug)]
enum IS {
    Set(usize, IE),
    If(IC, Vec<IS>, Vec<IS>),
    While(IC, Vec<IS>),
    For(usize, IE, IE, IE, Vec<IS>),
    Ret(IE),
    Break,
    Continue,
}
#[derive(Debug)]
struct IFunc {
    nparams: usize,
    nlocals: usize,
    body: Vec<IS>,
}

fn mk_bin(op: Bop, a: IE, b: IE) -> IE {
    match (a, b) {
        (IE::L(x), IE::C(c)) => IE::BinLC(op, x, c),
        (IE::L(x), IE::L(y)) => IE::BinLL(op, x, y),
        (a, b) => IE::Bin(op, Box::new(a), Box::new(b)),
    }
}

fn mk_cmp(op: Cop, a: IE, b: IE) -> IC {
    match (a, b) {
        (IE::L(x), IE::C(c)) => IC::CmpLC(op, x, c),
        (IE::L(x), IE::L(y)) => IC::CmpLL(op, x, y),
        (a, IE::C(c)) => IC::CmpC(op, Box::new(a), c),
        (a, b) => IC::Cmp(op, Box::new(a), Box::new(b)),
    }
}

fn is_name(e: &Option<Box<Expr>>, n: &str) -> bool {
    matches!(e.as_deref(), Some(Expr::Name(x)) if x.id.as_str() == n)
}

fn typed_signature(f: &ast::StmtFunctionDef) -> bool {
    f.args.posonlyargs.is_empty()
        && f.args.vararg.is_none()
        && f.args.kwonlyargs.is_empty()
        && f.args.kwarg.is_none()
        && f.args.args.iter().all(|a| a.default.is_none() && is_name(&a.def.annotation, "int"))
        && is_name(&f.returns, "int")
}

struct TCx<'a> {
    typed: &'a HashMap<String, usize>,
    locals: HashMap<String, usize>,
}

fn t_expr(e: &Expr, cx: &TCx) -> Option<IE> {
    Some(match e {
        Expr::Constant(c) => match &c.value {
            Constant::Int(i) => IE::C(i.to_string().parse::<i64>().ok()?),
            _ => return None,
        },
        Expr::Name(n) => IE::L(*cx.locals.get(n.id.as_str())?),
        Expr::BinOp(b) => {
            let op = match b.op {
                ast::Operator::Add => Bop::Add,
                ast::Operator::Sub => Bop::Sub,
                ast::Operator::Mult => Bop::Mul,
                ast::Operator::FloorDiv => Bop::FloorDiv,
                ast::Operator::Mod => Bop::Mod,
                ast::Operator::Pow => Bop::Pow,
                _ => return None,
            };
            mk_bin(op, t_expr(&b.left, cx)?, t_expr(&b.right, cx)?)
        }
        Expr::UnaryOp(u) => match u.op {
            ast::UnaryOp::USub => IE::Neg(Box::new(t_expr(&u.operand, cx)?)),
            ast::UnaryOp::UAdd => t_expr(&u.operand, cx)?,
            _ => return None,
        },
        Expr::IfExp(i) => IE::If(Box::new(t_cond(&i.test, cx)?), Box::new(t_expr(&i.body, cx)?), Box::new(t_expr(&i.orelse, cx)?)),
        Expr::Call(c) => {
            let Expr::Name(f) = &*c.func else { return None };
            let fi = *cx.typed.get(f.id.as_str())?;
            IE::Call(fi, c.args.iter().map(|a| t_expr(a, cx)).collect::<Option<Vec<_>>>()?)
        }
        _ => return None,
    })
}

fn t_cond(e: &Expr, cx: &TCx) -> Option<IC> {
    Some(match e {
        Expr::Compare(c) => {
            let mut parts = Vec::new();
            let mut left = t_expr(&c.left, cx)?;
            for (op, r) in c.ops.iter().zip(c.comparators.iter()) {
                let o = match op {
                    ast::CmpOp::Eq => Cop::Eq,
                    ast::CmpOp::NotEq => Cop::Ne,
                    ast::CmpOp::Lt => Cop::Lt,
                    ast::CmpOp::LtE => Cop::Le,
                    ast::CmpOp::Gt => Cop::Gt,
                    ast::CmpOp::GtE => Cop::Ge,
                    _ => return None,
                };
                let right = t_expr(r, cx)?;
                parts.push(mk_cmp(o, left, right));
                // chained comparison re-evaluates the shared operand: only allowed for names/constants (no side effects)
                if c.ops.len() > 1 && !matches!(r, Expr::Name(_) | Expr::Constant(_)) {
                    return None;
                }
                left = t_expr(r, cx)?;
            }
            if parts.len() == 1 {
                parts.pop()?
            } else {
                IC::And(parts)
            }
        }
        Expr::BoolOp(b) => {
            let vs = b.values.iter().map(|v| t_cond(v, cx)).collect::<Option<Vec<_>>>()?;
            match b.op {
                ast::BoolOp::And => IC::And(vs),
                ast::BoolOp::Or => IC::Or(vs),
            }
        }
        Expr::UnaryOp(u) if matches!(u.op, ast::UnaryOp::Not) => IC::Not(Box::new(t_cond(&u.operand, cx)?)),
        other => IC::Truth(Box::new(t_expr(other, cx)?)),
    })
}

fn t_block(body: &[Stmt], cx: &mut TCx) -> Option<Vec<IS>> {
    let mut out = Vec::new();
    for s in body {
        match s {
            Stmt::Pass(_) => {}
            Stmt::Break(_) => out.push(IS::Break),
            Stmt::Continue(_) => out.push(IS::Continue),
            Stmt::Return(r) => out.push(IS::Ret(t_expr(r.value.as_deref()?, cx)?)),
            Stmt::Assign(a) if a.targets.len() == 1 => {
                let Expr::Name(n) = &a.targets[0] else { return None };
                let v = t_expr(&a.value, cx)?;
                let k = cx.locals.len();
                let i = *cx.locals.entry(n.id.to_string()).or_insert(k);
                out.push(IS::Set(i, v));
            }
            Stmt::AnnAssign(a) => {
                let Expr::Name(n) = &*a.target else { return None };
                if !matches!(&*a.annotation, Expr::Name(x) if x.id.as_str() == "int") {
                    return None;
                }
                let v = t_expr(a.value.as_deref()?, cx)?;
                let k = cx.locals.len();
                let i = *cx.locals.entry(n.id.to_string()).or_insert(k);
                out.push(IS::Set(i, v));
            }
            Stmt::AugAssign(a) => {
                let Expr::Name(n) = &*a.target else { return None };
                let i = *cx.locals.get(n.id.as_str())?;
                let op = match a.op {
                    ast::Operator::Add => Bop::Add,
                    ast::Operator::Sub => Bop::Sub,
                    ast::Operator::Mult => Bop::Mul,
                    ast::Operator::FloorDiv => Bop::FloorDiv,
                    ast::Operator::Mod => Bop::Mod,
                    ast::Operator::Pow => Bop::Pow,
                    _ => return None,
                };
                out.push(IS::Set(i, mk_bin(op, IE::L(i), t_expr(&a.value, cx)?)));
            }
            Stmt::If(i) => {
                let c = t_cond(&i.test, cx)?;
                let a = t_block(&i.body, cx)?;
                let b = t_block(&i.orelse, cx)?;
                out.push(IS::If(c, a, b));
            }
            Stmt::While(w) if w.orelse.is_empty() => {
                let c = t_cond(&w.test, cx)?;
                out.push(IS::While(c, t_block(&w.body, cx)?));
            }
            Stmt::For(f) if f.orelse.is_empty() => {
                let Expr::Name(t) = &*f.target else { return None };
                let Expr::Call(c) = &*f.iter else { return None };
                if !matches!(&*c.func, Expr::Name(n) if n.id.as_str() == "range") || c.args.is_empty() || c.args.len() > 3 {
                    return None;
                }
                let a: Vec<IE> = c.args.iter().map(|x| t_expr(x, cx)).collect::<Option<Vec<_>>>()?;
                let mut it = a.into_iter();
                let (start, stop, step) = match it.len() {
                    1 => (IE::C(0), it.next()?, IE::C(1)),
                    2 => (it.next()?, it.next()?, IE::C(1)),
                    _ => (it.next()?, it.next()?, it.next()?),
                };
                let k = cx.locals.len();
                let ti = *cx.locals.entry(t.id.to_string()).or_insert(k);
                out.push(IS::For(ti, start, stop, step, t_block(&f.body, cx)?));
            }
            _ => return None,
        }
    }
    Some(out)
}

fn compile_typed(f: &ast::StmtFunctionDef, typed: &HashMap<String, usize>) -> Option<IFunc> {
    let mut locals = HashMap::new();
    for a in f.args.args.iter() {
        let n = locals.len();
        locals.insert(a.def.arg.to_string(), n);
    }
    let nparams = locals.len();
    let mut cx = TCx { typed, locals };
    let body = t_block(&f.body, &mut cx)?;
    Some(IFunc { nparams, nlocals: cx.locals.len(), body })
}

enum IFlow {
    Next,
    Break,
    Continue,
    Ret(i64),
}

fn iovf() -> Er {
    Er::from("OverflowError: integer overflow (RMojo integers are 64-bit)")
}

fn ibin(op: Bop, x: i64, y: i64) -> R<i64> {
    match op {
        Bop::Add => x.checked_add(y).ok_or_else(iovf),
        Bop::Sub => x.checked_sub(y).ok_or_else(iovf),
        Bop::Mul => x.checked_mul(y).ok_or_else(iovf),
        Bop::FloorDiv => {
            if y == 0 {
                return Err("ZeroDivisionError: division by zero".into());
            }
            let q = x.checked_div(y).ok_or_else(iovf)?;
            Ok(if (x % y != 0) && ((x < 0) != (y < 0)) { q - 1 } else { q })
        }
        Bop::Mod => {
            if y == 0 {
                return Err("ZeroDivisionError: division by zero".into());
            }
            let r = x.checked_rem(y).unwrap_or(0);
            Ok(if r != 0 && ((r < 0) != (y < 0)) { r + y } else { r })
        }
        Bop::Pow => {
            if y < 0 {
                return Err("TypeError: negative exponent needs float (not supported in typed int functions)".into());
            }
            x.checked_pow(u32::try_from(y).map_err(|_| iovf())?).ok_or_else(iovf)
        }
        Bop::Div => Err("internal: / in typed function".into()),
    }
}

#[inline(always)]
fn cmp_i(op: Cop, x: i64, y: i64) -> bool {
    match op {
        Cop::Eq => x == y,
        Cop::Ne => x != y,
        Cop::Lt => x < y,
        Cop::Le => x <= y,
        Cop::Gt => x > y,
        Cop::Ge => x >= y,
    }
}

struct IVm<'p> {
    funcs: &'p [Option<IFunc>],
    stack: Vec<i64>,
    depth: usize,
    fuel: u64,
}

impl<'p> IVm<'p> {
    #[inline]
    fn ev(&mut self, e: &IE, base: usize) -> R<i64> {
        match e {
            IE::C(v) => Ok(*v),
            IE::L(i) => Ok(self.stack[base + *i]),
            IE::Bin(op, a, b) => {
                let x = self.ev(a, base)?;
                let y = self.ev(b, base)?;
                match op {
                    Bop::Add => x.checked_add(y).ok_or_else(iovf),
                    Bop::Sub => x.checked_sub(y).ok_or_else(iovf),
                    Bop::Mul => x.checked_mul(y).ok_or_else(iovf),
                    _ => ibin(*op, x, y),
                }
            }
            IE::BinLC(op, i, c) => {
                let x = self.stack[base + *i];
                match op {
                    Bop::Add => x.checked_add(*c).ok_or_else(iovf),
                    Bop::Sub => x.checked_sub(*c).ok_or_else(iovf),
                    Bop::Mul => x.checked_mul(*c).ok_or_else(iovf),
                    Bop::Mod if *c > 0 => Ok(x.rem_euclid(*c)),
                    Bop::FloorDiv if *c > 0 => Ok(x.div_euclid(*c)),
                    _ => ibin(*op, x, *c),
                }
            }
            IE::BinLL(op, i, j) => {
                let (x, y) = (self.stack[base + *i], self.stack[base + *j]);
                match op {
                    Bop::Add => x.checked_add(y).ok_or_else(iovf),
                    Bop::Sub => x.checked_sub(y).ok_or_else(iovf),
                    Bop::Mul => x.checked_mul(y).ok_or_else(iovf),
                    _ => ibin(*op, x, y),
                }
            }
            IE::Neg(a) => self.ev(a, base)?.checked_neg().ok_or_else(iovf),
            IE::If(c, a, b) => {
                if self.cd(c, base)? {
                    self.ev(a, base)
                } else {
                    self.ev(b, base)
                }
            }
            IE::Call(fi, args) => {
                let nbase = self.stack.len();
                for a in args {
                    let v = self.ev(a, base)?;
                    self.stack.push(v);
                }
                self.run(*fi, nbase)
            }
        }
    }

    fn run(&mut self, fi: usize, nbase: usize) -> R<i64> {
        let Some(f) = self.funcs[fi].as_ref() else { return Err("internal: not a typed function".into()) };
        if self.depth >= MAX_DEPTH {
            return Err("RecursionError: maximum recursion depth exceeded".into());
        }
        self.stack.resize(nbase + f.nlocals, 0);
        self.depth += 1;
        let r = self.blk(&f.body, nbase);
        self.depth -= 1;
        self.stack.truncate(nbase);
        match r? {
            IFlow::Ret(v) => Ok(v),
            _ => Err("TypeError: function annotated -> int returned no value".into()),
        }
    }

    fn cd(&mut self, c: &IC, base: usize) -> R<bool> {
        Ok(match c {
            IC::Cmp(op, a, b) => {
                let x = self.ev(a, base)?;
                let y = self.ev(b, base)?;
                match op {
                    Cop::Eq => x == y,
                    Cop::Ne => x != y,
                    Cop::Lt => x < y,
                    Cop::Le => x <= y,
                    Cop::Gt => x > y,
                    Cop::Ge => x >= y,
                }
            }
            IC::CmpC(op, a, c) => cmp_i(*op, self.ev(a, base)?, *c),
            IC::CmpLC(op, i, c) => cmp_i(*op, self.stack[base + *i], *c),
            IC::CmpLL(op, i, j) => cmp_i(*op, self.stack[base + *i], self.stack[base + *j]),
            IC::And(v) => {
                for c in v {
                    if !self.cd(c, base)? {
                        return Ok(false);
                    }
                }
                true
            }
            IC::Or(v) => {
                for c in v {
                    if self.cd(c, base)? {
                        return Ok(true);
                    }
                }
                false
            }
            IC::Not(c) => !self.cd(c, base)?,
            IC::Truth(e) => self.ev(e, base)? != 0,
        })
    }

    fn blk(&mut self, body: &[IS], base: usize) -> R<IFlow> {
        for s in body {
            match s {
                IS::Set(i, e) => {
                    let v = self.ev(e, base)?;
                    self.stack[base + *i] = v;
                }
                IS::If(c, a, b) => {
                    let f = if self.cd(c, base)? { self.blk(a, base)? } else { self.blk(b, base)? };
                    if !matches!(f, IFlow::Next) {
                        return Ok(f);
                    }
                }
                IS::While(c, b) => {
                    while self.cd(c, base)? {
                        self.fuel = self.fuel.checked_sub(1).ok_or_else(|| Er::from("LimitError: step limit exceeded"))?;
                        match self.blk(b, base)? {
                            IFlow::Break => break,
                            IFlow::Ret(v) => return Ok(IFlow::Ret(v)),
                            _ => {}
                        }
                    }
                }
                IS::For(t, start, stop, step, b) => {
                    let mut i = self.ev(start, base)?;
                    let end = self.ev(stop, base)?;
                    let st = self.ev(step, base)?;
                    if st == 0 {
                        return Err("ValueError: range() arg 3 must not be zero".into());
                    }
                    while (st > 0 && i < end) || (st < 0 && i > end) {
                        self.fuel = self.fuel.checked_sub(1).ok_or_else(|| Er::from("LimitError: step limit exceeded"))?;
                        self.stack[base + *t] = i;
                        match self.blk(b, base)? {
                            IFlow::Break => break,
                            IFlow::Ret(v) => return Ok(IFlow::Ret(v)),
                            _ => {}
                        }
                        i = match i.checked_add(st) {
                            Some(n) => n,
                            None => break,
                        };
                    }
                }
                IS::Ret(e) => return Ok(IFlow::Ret(self.ev(e, base)?)),
                IS::Break => return Ok(IFlow::Break),
                IS::Continue => return Ok(IFlow::Continue),
            }
        }
        Ok(IFlow::Next)
    }
}

// ---------------------------------------------------------------- run

enum Flow {
    Next,
    Break,
    Continue,
    Return(V),
}

struct Vm<'p> {
    p: &'p Program,
    globals: Vec<V>,
    depth: usize,
    stack: Vec<V>,
    brk: bool,
    cur: String,
    out: std::io::BufWriter<std::io::Stdout>,
    fuel: u64,
}

fn pyrepr_f(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let a = f.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        let s = format!("{:e}", f); // e.g. 1e21, 1.5e-7
        let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
        let ev: i32 = e.parse().unwrap_or(0);
        return format!("{}e{}{:02}", m, if ev < 0 { '-' } else { '+' }, ev.abs());
    }
    let s = format!("{:?}", f);
    s
}

fn repr(v: &V) -> String {
    match v {
        V::S(s) => {
            let q = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
            let mut o = String::new();
            o.push(q);
            for c in s.chars() {
                match c {
                    '\\' => o.push_str("\\\\"),
                    '\n' => o.push_str("\\n"),
                    '\t' => o.push_str("\\t"),
                    c if c == q => {
                        o.push('\\');
                        o.push(c)
                    }
                    c => o.push(c),
                }
            }
            o.push(q);
            o
        }
        V::T(x) => {
            if x.len() == 1 {
                format!("({},)", repr(&x[0]))
            } else {
                format!("({})", x.iter().map(repr).collect::<Vec<_>>().join(", "))
            }
        }
        V::X(x) => format!("[{}]", x.iter().map(repr).collect::<Vec<_>>().join(", ")),
        V::L(l) => format!("[{}]", l.borrow().iter().map(repr).collect::<Vec<_>>().join(", ")),
        V::Z(l) => if l.borrow().is_empty() { "set()".to_string() } else { format!("{{{}}}", l.borrow().iter().map(repr).collect::<Vec<_>>().join(", ")) },
        V::D(d) => format!("{{{}}}", d.borrow().iter().map(|(k, v)| format!("{}: {}", repr(k), repr(v))).collect::<Vec<_>>().join(", ")),
        other => show(other),
    }
}

fn veq(a: &V, b: &V) -> bool {
    match (a, b) {
        (V::L(x), V::L(y)) => {
            let (x, y) = (x.borrow(), y.borrow());
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| veq(p, q))
        }
        (V::T(x), V::T(y)) => x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| veq(p, q)),
        (V::N, V::N) => true,
        _ => compare(Cop::Eq, a, b).unwrap_or(false),
    }
}

fn slice_indices(len: usize, lo: Option<i64>, hi: Option<i64>, step: Option<i64>) -> R<Vec<usize>> {
    let st = step.unwrap_or(1);
    if st == 0 {
        return Err("ValueError: slice step cannot be zero".into());
    }
    let n = len as i64;
    let clamp = |v: i64, lo_b: i64, hi_b: i64| v.max(lo_b).min(hi_b);
    let mut out = Vec::new();
    if st > 0 {
        let a = match lo { Some(v) => clamp(if v < 0 { v + n } else { v }, 0, n), None => 0 };
        let b = match hi { Some(v) => clamp(if v < 0 { v + n } else { v }, 0, n), None => n };
        let mut i = a;
        while i < b {
            out.push(i as usize);
            i += st;
        }
    } else {
        let a = match lo { Some(v) => clamp(if v < 0 { v + n } else { v }, -1, n - 1), None => n - 1 };
        let b = match hi { Some(v) => clamp(if v < 0 { v + n } else { v }, -1, n - 1), None => -1 };
        let mut i = a;
        while i > b {
            out.push(i as usize);
            i += st;
        }
    }
    Ok(out)
}

fn norm_index(i: i64, len: usize) -> Option<usize> {
    let n = if i < 0 { i + len as i64 } else { i };
    if n >= 0 && (n as usize) < len {
        Some(n as usize)
    } else {
        None
    }
}

fn show(v: &V) -> String {
    match v {
        V::I(i) => i.to_string(),
        V::F(f) => pyrepr_f(*f),
        V::B(b) => if *b { "True".into() } else { "False".into() },
        V::S(s) => s.as_str().to_string(),
        V::O(_) => "<object>".into(),
        V::L(_) | V::D(_) | V::Z(_) | V::X(_) | V::T(_) => repr(v),
        V::N => "None".into(),
    }
}

fn format_spec(v: &V, spec: &str) -> R<String> {
    let ch: Vec<char> = spec.chars().collect();
    let mut i = 0;
    let (mut fill, mut align) = (' ', None);
    if ch.len() >= 2 && matches!(ch[1], '<' | '>' | '^') {
        fill = ch[0];
        align = Some(ch[1]);
        i = 2;
    } else if !ch.is_empty() && matches!(ch[0], '<' | '>' | '^') {
        align = Some(ch[0]);
        i = 1;
    }
    let mut zero = false;
    if i < ch.len() && ch[i] == '0' && align.is_none() {
        zero = true;
        i += 1;
    }
    let mut width = 0usize;
    while i < ch.len() && ch[i].is_ascii_digit() {
        width = width * 10 + ch[i].to_digit(10).unwrap() as usize;
        i += 1;
    }
    let mut prec: Option<usize> = None;
    if i < ch.len() && ch[i] == '.' {
        i += 1;
        let mut p = 0usize;
        while i < ch.len() && ch[i].is_ascii_digit() {
            p = p * 10 + ch[i].to_digit(10).unwrap() as usize;
            i += 1;
        }
        prec = Some(p);
    }
    let ty = if i < ch.len() { Some(ch[i]) } else { None };
    if i + 1 < ch.len() {
        return Err("ValueError: unsupported format spec".into());
    }
    let mut numeric = false;
    let body = match (v, ty) {
        (V::I(n), None | Some('d')) => {
            numeric = true;
            n.to_string()
        }
        (V::I(n), Some('f')) => {
            numeric = true;
            format!("{:.*}", prec.unwrap_or(6), *n as f64)
        }
        (V::F(f), Some('f')) => {
            numeric = true;
            format!("{:.*}", prec.unwrap_or(6), f)
        }
        (V::F(f), None) => {
            numeric = true;
            match prec {
                Some(p) => format!("{:.*}", p, f),
                None => pyrepr_f(*f),
            }
        }
        (V::S(s), None | Some('s')) => match prec {
            Some(p) => s.chars().take(p).collect(),
            None => s.to_string(),
        },
        (V::I(n), Some('x')) => format!("{:x}", n),
        (V::I(n), Some('b')) => format!("{:b}", n),
        _ => return Err("ValueError: unsupported format spec for this type".into()),
    };
    let n = body.chars().count();
    if n >= width {
        return Ok(body);
    }
    let pad = width - n;
    if zero && numeric {
        let (sign, digits) = match body.strip_prefix('-') {
            Some(r) => ("-", r.to_string()),
            None => ("", body.clone()),
        };
        return Ok(format!("{}{}{}", sign, "0".repeat(pad), digits));
    }
    let al = align.unwrap_or(if numeric { '>' } else { '<' });
    let mk = |k: usize| fill.to_string().repeat(k);
    Ok(match al {
        '<' => format!("{}{}", body, mk(pad)),
        '>' => format!("{}{}", mk(pad), body),
        _ => format!("{}{}{}", mk(pad / 2), body, mk(pad - pad / 2)),
    })
}

fn truthy(v: &V) -> bool {
    match v {
        V::I(i) => *i != 0,
        V::F(f) => *f != 0.0,
        V::B(b) => *b,
        V::S(s) => !s.is_empty(),
        V::O(_) => true,
        V::X(x) | V::T(x) => !x.is_empty(),
        V::L(l) => !l.borrow().is_empty(),
        V::D(d) => !d.borrow().is_empty(),
        V::Z(l) => !l.borrow().is_empty(),
        V::N => false,
    }
}

fn num(v: &V) -> Option<f64> {
    match v {
        V::I(i) => Some(*i as f64),
        V::F(f) => Some(*f),
        V::B(b) => Some(*b as i64 as f64),
        _ => None,
    }
}

fn int_of(v: &V) -> Option<i64> {
    match v {
        V::I(i) => Some(*i),
        V::B(b) => Some(*b as i64),
        _ => None,
    }
}

fn binop(op: Bop, a: V, b: V) -> R<V> {
    if let (Bop::Sub, V::Z(x), V::Z(y)) = (op, &a, &b) {
        let v: Vec<V> = x.borrow().iter().filter(|e| !y.borrow().iter().any(|o| veq(o, e))).cloned().collect();
        return Ok(V::Z(Rc::new(std::cell::RefCell::new(v))));
    }
    if matches!(a, V::X(_)) || matches!(b, V::X(_)) {
        let (la, lb) = (match &a { V::X(x) => x.len(), _ => 0 }, match &b { V::X(x) => x.len(), _ => 0 });
        if la != 0 && lb != 0 && la != lb {
            return Err("ValueError: SIMD width mismatch".into());
        }
        let n = la.max(lb);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let x = match &a { V::X(x) => x[i].clone(), o => o.clone() };
            let y = match &b { V::X(y) => y[i].clone(), o => o.clone() };
            out.push(binop(op, x, y)?);
        }
        return Ok(V::X(Rc::new(out)));
    }
    if let (Some(x), Some(y)) = (int_of(&a), int_of(&b)) {
        let ovf = || "OverflowError: integer overflow (RMojo integers are 64-bit)".to_string();
        return Ok(match op {
            Bop::Add => V::I(x.checked_add(y).ok_or_else(ovf)?),
            Bop::Sub => V::I(x.checked_sub(y).ok_or_else(ovf)?),
            Bop::Mul => V::I(x.checked_mul(y).ok_or_else(ovf)?),
            Bop::Div => {
                if y == 0 {
                    return Err("ZeroDivisionError: division by zero".into());
                }
                V::F(x as f64 / y as f64)
            }
            Bop::FloorDiv => {
                if y == 0 {
                    return Err("ZeroDivisionError: division by zero".into());
                }
                let q = x.checked_div(y).ok_or_else(ovf)?;
                V::I(if (x % y != 0) && ((x < 0) != (y < 0)) { q - 1 } else { q })
            }
            Bop::Mod => {
                if y == 0 {
                    return Err("ZeroDivisionError: division by zero".into());
                }
                let r = x.checked_rem(y).unwrap_or(0);
                V::I(if r != 0 && ((r < 0) != (y < 0)) { r + y } else { r })
            }
            Bop::Pow => {
                if y >= 0 {
                    let e = u32::try_from(y).map_err(|_| ovf())?;
                    V::I(x.checked_pow(e).ok_or_else(ovf)?)
                } else {
                    V::F((x as f64).powf(y as f64))
                }
            }
        });
    }
    if op == Bop::Mul {
        let rep = |seq: &V, n: i64| -> Option<V> {
            let n = n.max(0) as usize;
            match seq {
                V::L(l) => {
                    let l = l.borrow();
                    let mut v = Vec::with_capacity(l.len() * n);
                    for _ in 0..n {
                        v.extend(l.iter().cloned());
                    }
                    Some(new_list(v))
                }
                V::S(s) => Some(V::S(Rc::new(s.repeat(n)))),
                _ => None,
            }
        };
        if let Some(n) = int_of(&b) {
            if let Some(r) = rep(&a, n) {
                return Ok(r);
            }
        }
        if let Some(n) = int_of(&a) {
            if let Some(r) = rep(&b, n) {
                return Ok(r);
            }
        }
    }
    if let (V::L(x), V::L(y), Bop::Add) = (&a, &b, op) {
        let mut n = x.borrow().clone();
        n.extend(y.borrow().iter().cloned());
        return Ok(V::L(Rc::new(std::cell::RefCell::new(n))));
    }
    if let (V::S(x), V::S(y), Bop::Add) = (&a, &b, op) {
        return Ok(V::S(Rc::new(format!("{x}{y}"))));
    }
    if let (Some(x), Some(y)) = (num(&a), num(&b)) {
        return Ok(match op {
            Bop::Add => V::F(x + y),
            Bop::Sub => V::F(x - y),
            Bop::Mul => V::F(x * y),
            Bop::Div => {
                if y == 0.0 {
                    return Err("ZeroDivisionError: float division by zero".into());
                }
                V::F(x / y)
            }
            Bop::FloorDiv => {
                if y == 0.0 {
                    return Err("ZeroDivisionError: float floor division by zero".into());
                }
                V::F((x / y).floor())
            }
            Bop::Mod => {
                if y == 0.0 {
                    return Err("ZeroDivisionError: float modulo".into());
                }
                let r = x % y;
                V::F(if r != 0.0 && ((r < 0.0) != (y < 0.0)) { r + y } else { r })
            }
            Bop::Pow => V::F(x.powf(y)),
        });
    }
    Err("TypeError: unsupported operand types".into())
}

fn compare(op: Cop, a: &V, b: &V) -> R<bool> {
    if let (V::S(x), V::S(y)) = (a, b) {
        return Ok(match op {
            Cop::Eq => x == y,
            Cop::Ne => x != y,
            Cop::Lt => x < y,
            Cop::Le => x <= y,
            Cop::Gt => x > y,
            Cop::Ge => x >= y,
        });
    }
    if let (V::N, V::N) = (a, b) {
        return Ok(matches!(op, Cop::Eq | Cop::Le | Cop::Ge));
    }
    if let (Some(x), Some(y)) = (int_of(a), int_of(b)) {
        return Ok(match op {
            Cop::Eq => x == y,
            Cop::Ne => x != y,
            Cop::Lt => x < y,
            Cop::Le => x <= y,
            Cop::Gt => x > y,
            Cop::Ge => x >= y,
        });
    }
    if let (Some(x), Some(y)) = (num(a), num(b)) {
        return Ok(match op {
            Cop::Eq => x == y,
            Cop::Ne => x != y,
            Cop::Lt => x < y,
            Cop::Le => x <= y,
            Cop::Gt => x > y,
            Cop::Ge => x >= y,
        });
    }
    let seq_eq = match (a, b) {
        (V::T(x), V::T(y)) => Some(x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| veq(p, q))),
        (V::L(x), V::L(y)) => Some({
            let (x, y) = (x.borrow(), y.borrow());
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| veq(p, q))
        }),
        _ => None,
    };
    if let Some(e) = seq_eq {
        return match op {
            Cop::Eq => Ok(e),
            Cop::Ne => Ok(!e),
            _ => Err("TypeError: unorderable types".into()),
        };
    }
    match op {
        Cop::Eq => Ok(false),
        Cop::Ne => Ok(true),
        _ => Err("TypeError: unorderable types".into()),
    }
}

impl<'p> Vm<'p> {
    /// `str()`/`print()` of a value; structs with a `__str__` method use it (Mojo `Stringable`).
    fn show_vm(&mut self, v: &V) -> R<String> {
        if let V::O(ob) = v {
            if let Some(&fi) = self.p.classes[ob.cls].methods.get("__str__") {
                let r = self.call_values(fi, vec![v.clone()])?;
                return Ok(show(&r));
            }
        }
        Ok(show(v))
    }

    fn index_value(&self, ov: V, iv: V) -> R<V> {
        Ok(
                match (&ov, &iv) {
                    (V::L(l), _) => {
                        let Some(ix) = int_of(&iv) else { return Err("TypeError: list indices must be integers".into()) };
                        let l = l.borrow();
                        match norm_index(ix, l.len()) {
                            Some(k) => l[k].clone(),
                            None => return Err("IndexError: list index out of range".into()),
                        }
                    }
                    (V::T(x), _) => {
                        let Some(ix) = int_of(&iv) else { return Err("TypeError: tuple indices must be integers".into()) };
                        match norm_index(ix, x.len()) {
                            Some(k) => x[k].clone(),
                            None => return Err("IndexError: tuple index out of range".into()),
                        }
                    }
                    (V::X(x), _) => {
                        let Some(ix) = int_of(&iv) else { return Err("TypeError: SIMD indices must be integers".into()) };
                        match norm_index(ix, x.len()) {
                            Some(k) => x[k].clone(),
                            None => return Err("IndexError: SIMD index out of range".into()),
                        }
                    }
                    (V::D(d), _) => match d.borrow().iter().find(|(k, _)| veq(k, &iv)) {
                        Some((_, v)) => v.clone(),
                        None => return Err(format!("KeyError: {}", repr(&iv)).into()),
                    },
                    (V::S(st), _) => {
                        let Some(ix) = int_of(&iv) else { return Err("TypeError: string indices must be integers".into()) };
                        let cs: Vec<char> = st.chars().collect();
                        match norm_index(ix, cs.len()) {
                            Some(k) => V::S(Rc::new(cs[k].to_string())),
                            None => return Err("IndexError: string index out of range".into()),
                        }
                    }
                    _ => return Err("TypeError: object is not subscriptable".into()),
                }
        )
    }

    fn set_index(&mut self, ov: V, iv: V, v: V) -> R<()> {
                    match &ov {
                        V::L(l) => {
                            let Some(ix) = int_of(&iv) else { return Err("TypeError: list indices must be integers".into()) };
                            let mut l = l.borrow_mut();
                            let n = l.len();
                            match norm_index(ix, n) {
                                Some(k) => l[k] = v,
                                None => return Err("IndexError: list assignment index out of range".into()),
                            }
                        }
                        V::D(d) => {
                            let mut d = d.borrow_mut();
                            match d.iter().position(|(k, _)| veq(k, &iv)) {
                                Some(k) => d[k].1 = v,
                                None => d.push((iv, v)),
                            }
                        }
                        _ => return Err("TypeError: object does not support item assignment".into()),
                    }
        Ok(())
    }

    #[inline(never)]
    fn eval_cold(&mut self, e: &E, base: usize) -> R<V> {
        Ok(match e {
            E::List(items) => {
                let mut v = Vec::with_capacity(items.len());
                for i in items.iter() {
                    v.push(self.eval(i, base)?);
                }
                V::L(Rc::new(std::cell::RefCell::new(v)))
            }
            E::Dict(kv) => {
                let mut d: Vec<(V, V)> = Vec::new();
                for (k, v) in kv.iter() {
                    let (k, v) = (self.eval(k, base)?, self.eval(v, base)?);
                    match d.iter().position(|(x, _)| veq(x, &k)) {
                        Some(i) => d[i].1 = v,
                        None => d.push((k, v)),
                    }
                }
                V::D(Rc::new(std::cell::RefCell::new(d)))
            }
            E::Fmt(inner, spec) => {
                let v = self.eval(inner, base)?;
                if spec.is_empty() {
                    V::S(Rc::new(self.show_vm(&v)?))
                } else {
                    V::S(Rc::new(format_spec(&v, spec)?))
                }
            }
            E::Concat(parts) => {
                let mut o = String::new();
                for p in parts.iter() {
                    let v = self.eval(p, base)?;
                    o.push_str(&self.show_vm(&v)?);
                }
                V::S(Rc::new(o))
            }
            E::ToSet(inner) => {
                let lv = self.eval(inner, base)?;
                let V::L(l) = &lv else { return Err("TypeError: set".into()) };
                let items = l.borrow().clone();
                V::Z(Rc::new(std::cell::RefCell::new(dedup(items))))
            }
            E::ToDict(inner) => {
                let lv = self.eval(inner, base)?;
                let V::L(l) = &lv else { return Err("TypeError: dict comprehension".into()) };
                let mut d: Vec<(V, V)> = Vec::new();
                for item in l.borrow().iter() {
                    let V::T(kv) = item else { continue };
                    let (k, v) = (kv[0].clone(), kv[1].clone());
                    match d.iter().position(|(x, _)| veq(x, &k)) {
                        Some(i) => d[i].1 = v,
                        None => d.push((k, v)),
                    }
                }
                V::D(Rc::new(std::cell::RefCell::new(d)))
            }
            E::Comp(gens, elt) => {
                let mut out = Vec::new();
                self.comp_rec(gens, 0, elt, base, &mut out)?;
                V::L(Rc::new(std::cell::RefCell::new(out)))
            }
            E::Tuple(items) => {
                let mut v = Vec::with_capacity(items.len());
                for i in items.iter() {
                    v.push(self.eval(i, base)?);
                }
                V::T(Rc::new(v))
            }
            E::Slice(o, lo, hi, st) => {
                let ov = self.eval(o, base)?;
                let mut opt = |x: &Option<Box<E>>, me: &mut Self| -> R<Option<i64>> {
                    Ok(match x {
                        Some(e) => Some(int_of(&me.eval(e, base)?).ok_or("TypeError: slice indices must be integers")?),
                        None => None,
                    })
                };
                let (l, h, s) = (opt(lo, self)?, opt(hi, self)?, opt(st, self)?);
                match &ov {
                    V::L(v) => {
                        let v = v.borrow();
                        V::L(Rc::new(std::cell::RefCell::new(slice_indices(v.len(), l, h, s)?.into_iter().map(|k| v[k].clone()).collect())))
                    }
                    V::T(v) => V::T(Rc::new(slice_indices(v.len(), l, h, s)?.into_iter().map(|k| v[k].clone()).collect())),
                    V::S(st) => {
                        let cs: Vec<char> = st.chars().collect();
                        V::S(Rc::new(slice_indices(cs.len(), l, h, s)?.into_iter().map(|k| cs[k]).collect()))
                    }
                    _ => return Err("TypeError: object is not sliceable".into()),
                }
            }
            E::In(x, c, neg) => {
                let xv = self.eval(x, base)?;
                let cv = self.eval(c, base)?;
                let found = match &cv {
                    V::L(l) => l.borrow().iter().any(|e| veq(e, &xv)),
                    V::T(l) => l.iter().any(|e| veq(e, &xv)),
                    V::D(d) => d.borrow().iter().any(|(k, _)| veq(k, &xv)),
                    V::Z(l) => l.borrow().iter().any(|e| veq(e, &xv)),
                    V::S(h) => match &xv {
                        V::S(n) => h.contains(n.as_str()),
                        _ => return Err("TypeError: 'in <string>' requires string as left operand".into()),
                    },
                    _ => return Err("TypeError: argument of type is not iterable".into()),
                };
                V::B(found != *neg)
            }
            E::Attr(o, name) => match self.eval(o, base)? {
                V::O(ob) => {
                    let idx = self.p.classes[ob.cls].fields.iter().position(|f| f == name).ok_or_else(|| format!("AttributeError: no attribute '{name}'"))?;
                    let v = ob.f.borrow()[idx].clone();
                    v
                }
                _ => return Err(format!("AttributeError: no attribute '{name}'").into()),
            },
            E::New(ci, args) => {
                let cl = &self.p.classes[*ci];
                let ob = Rc::new(ObjBox { cls: *ci, f: std::cell::RefCell::new(vec![V::N; cl.fields.len()]) });
                let mut vals = Vec::with_capacity(args.len());
                for a in args.iter() {
                    vals.push(self.eval(a, base)?);
                }
                match cl.init {
                    Some(fi) => {
                        let mut all = vec![V::O(ob.clone())];
                        all.extend(vals);
                        self.call_values(fi, all)?;
                    }
                    None => {
                        if vals.len() > cl.fields.len() {
                            return Err("TypeError: too many constructor arguments".into());
                        }
                        for (i, v) in vals.into_iter().enumerate() {
                            ob.f.borrow_mut()[i] = v;
                        }
                    }
                }
                V::O(ob)
            }
            E::Method(o, name, args) => {
                let ov = self.eval(o, base)?;
                // fast path: list.append(x) without building an argument vector or matching names
                if let (V::L(l), 1, "append") = (&ov, args.len(), name.as_str()) {
                    let x = self.eval(&args[0], base)?;
                    l.borrow_mut().push(x);
                    return Ok(V::N);
                }
                if !matches!(ov, V::O(_)) {
                    let mut vals = Vec::new();
                    for a in args.iter() {
                        vals.push(self.eval(a, base)?);
                    }
                    return builtin_method(&ov, name, vals);
                }
                let V::O(ob) = &ov else { return Err(format!("AttributeError: no method '{name}'").into()) };
                let fi = *self.p.classes[ob.cls].methods.get(name).ok_or_else(|| format!("AttributeError: no method '{name}'"))?;
                let mut all = vec![ov.clone()];
                for a in args.iter() {
                    all.push(self.eval(a, base)?);
                }
                self.call_values(fi, all)?
            }
            _ => return Err("internal: eval_cold".into()),
        })
    }

    fn comp_rec(&mut self, gens: &[(Vec<Target>, E, Vec<E>)], gi: usize, elt: &E, base: usize, out: &mut Vec<V>) -> R<()> {
        if gi == gens.len() {
            out.push(self.eval(elt, base)?);
            return Ok(());
        }
        let (tg, it, conds) = &gens[gi];
        let itv = self.eval(it, base)?;
        let items: Vec<V> = match &itv {
            V::L(l) => l.borrow().clone(),
            V::T(l) => l.to_vec(),
            V::D(d) => d.borrow().iter().map(|(k, _)| k.clone()).collect(),
            V::Z(l) => l.borrow().clone(),
            V::S(st) => st.chars().map(|c| V::S(Rc::new(c.to_string()))).collect(),
            _ => return Err("TypeError: object is not iterable".into()),
        };
        'next: for item in items {
            let parts: Vec<V> = if tg.len() == 1 {
                vec![item]
            } else {
                match &item {
                    V::L(p) if p.borrow().len() == tg.len() => p.borrow().clone(),
                    V::T(p) if p.len() == tg.len() => p.to_vec(),
                    _ => return Err("ValueError: cannot unpack comprehension item".into()),
                }
            };
            for (t, v) in tg.iter().zip(parts) {
                match t {
                    Target::Local(k) => self.stack[base + *k] = v,
                    Target::Global(k) => self.globals[*k] = v,
                }
            }
            for c in conds {
                if !truthy(&self.eval(c, base)?) {
                    continue 'next;
                }
            }
            self.comp_rec(gens, gi + 1, elt, base, out)?;
        }
        Ok(())
    }

    fn call_values(&mut self, fi: usize, vals: Vec<V>) -> R<V> {
        let f = &self.p.funcs[fi];
        if vals.len() != f.nparams {
            return Err(format!("TypeError: function takes {} arguments but {} were given", f.nparams, vals.len()).into());
        }
        if self.depth >= MAX_DEPTH {
            return Err("RecursionError: maximum recursion depth exceeded".into());
        }
        let nbase = self.stack.len();
        self.stack.extend(vals);
        self.stack.resize(nbase + f.nlocals, V::N);
        self.depth += 1;
        let r = self.exec_block(&f.body, nbase);
        self.depth -= 1;
        self.stack.truncate(nbase);
        Ok(match r? {
            Flow::Return(v) => v,
            _ => V::N,
        })
    }

    /// Condition evaluation without materialising a `V` for the common int comparison.
    #[inline]
    fn cond(&mut self, e: &E, base: usize) -> R<bool> {
        if let E::Cmp(first, rest) = e {
            if rest.len() == 1 {
                let l = self.eval(first, base)?;
                let r = self.eval(&rest[0].1, base)?;
                if let (V::I(p), V::I(q)) = (&l, &r) {
                    return Ok(match rest[0].0 {
                        Cop::Eq => p == q,
                        Cop::Ne => p != q,
                        Cop::Lt => p < q,
                        Cop::Le => p <= q,
                        Cop::Gt => p > q,
                        Cop::Ge => p >= q,
                    });
                }
                return compare(rest[0].0, &l, &r);
            }
        }
        Ok(truthy(&self.eval(e, base)?))
    }

    #[inline]
    fn eval(&mut self, e: &E, base: usize) -> R<V> {
        Ok(match e {
            E::Const(v) => v.clone(),
            E::Local(i) => self.stack[base + *i].clone(),
            E::Global(i) => self.globals[*i].clone(),
            E::Bin(op, a, b) => {
                let x = self.eval(a, base)?;
                let y = self.eval(b, base)?;
                if let (V::I(p), V::I(q)) = (&x, &y) {
                    let r = match op {
                        Bop::Add => p.checked_add(*q),
                        Bop::Sub => p.checked_sub(*q),
                        Bop::Mul => p.checked_mul(*q),
                        _ => None,
                    };
                    if let Some(r) = r {
                        return Ok(V::I(r));
                    }
                }
                binop(*op, x, y)?
            }
            E::Neg(a) => match self.eval(a, base)? {
                V::I(i) => V::I(i.checked_neg().ok_or("OverflowError: integer overflow")?),
                V::F(f) => V::F(-f),
                V::B(b) => V::I(-(b as i64)),
                _ => return Err("TypeError: bad operand type for unary -".into()),
            },
            E::Not(a) => V::B(!truthy(&self.eval(a, base)?)),
            E::Cmp(first, rest) => {
                if rest.len() == 1 {
                    let l = self.eval(first, base)?;
                    let r = self.eval(&rest[0].1, base)?;
                    if let (V::I(p), V::I(q)) = (&l, &r) {
                        return Ok(V::B(match rest[0].0 {
                            Cop::Eq => p == q,
                            Cop::Ne => p != q,
                            Cop::Lt => p < q,
                            Cop::Le => p <= q,
                            Cop::Gt => p > q,
                            Cop::Ge => p >= q,
                        }));
                    }
                    return Ok(V::B(compare(rest[0].0, &l, &r)?));
                }
                let mut l = self.eval(first, base)?;
                let mut ok = true;
                for (op, r) in rest {
                    let rv = self.eval(r, base)?;
                    if !compare(*op, &l, &rv)? {
                        ok = false;
                        break;
                    }
                    l = rv;
                }
                V::B(ok)
            }
            E::And(vs) => {
                let mut last = V::B(true);
                for v in vs {
                    last = self.eval(v, base)?;
                    if !truthy(&last) {
                        break;
                    }
                }
                last
            }
            E::Or(vs) => {
                let mut last = V::B(false);
                for v in vs {
                    last = self.eval(v, base)?;
                    if truthy(&last) {
                        break;
                    }
                }
                last
            }
            E::If(c, a, b) => {
                if self.cond(c, base)? {
                    self.eval(a, base)?
                } else {
                    self.eval(b, base)?
                }
            }
            E::Call(fi, args) => {
                let f = &self.p.funcs[*fi];
                if args.len() != f.nparams {
                    return Err(format!("TypeError: function takes {} arguments but {} were given", f.nparams, args.len()).into());
                }
                if self.depth >= MAX_DEPTH {
                    return Err("RecursionError: maximum recursion depth exceeded".into());
                }
                if self.p.ifuncs[*fi].is_some() {
                    let mut iv = IVm { funcs: &self.p.ifuncs, stack: Vec::with_capacity(64), depth: self.depth, fuel: self.fuel };
                    for a in args.iter() {
                        let v = match self.eval(a, base)? {
                            V::I(i) => i,
                            V::B(b) => b as i64,
                            _ => return Err("TypeError: argument must be int".into()),
                        };
                        iv.stack.push(v);
                    }
                    let r = iv.run(*fi, 0)?;
                    self.fuel = iv.fuel;
                    return Ok(V::I(r));
                }
                let nbase = self.stack.len();
                for a in args.iter() {
                    let v = self.eval(a, base)?;
                    self.stack.push(v);
                }
                self.stack.resize(nbase + f.nlocals, V::N);
                self.depth += 1;
                let r = self.exec_block(&f.body, nbase);
                self.depth -= 1;
                self.stack.truncate(nbase);
                match r? {
                    Flow::Return(v) => v,
                    _ => V::N,
                }
            }
            E::Index(o, i) => {
                // fast path: list held in a local/global, integer index (no Rc clone, no re-borrow of the stack)
                if matches!(**o, E::Local(_) | E::Global(_)) {
                    let iv = self.eval(i, base)?;
                    if let V::I(ix) = iv {
                        let slot = match &**o {
                            E::Local(k) => &self.stack[base + *k],
                            E::Global(k) => &self.globals[*k],
                            _ => unreachable!(),
                        };
                        if let V::L(l) = slot {
                            let l = l.borrow();
                            return match norm_index(ix, l.len()) {
                                Some(k) => Ok(l[k].clone()),
                                None => Err("IndexError: list index out of range".into()),
                            };
                        }
                    }
                    let ov = self.eval(o, base)?;
                    return self.index_value(ov, iv);
                }
                let ov = self.eval(o, base)?;
                let iv = self.eval(i, base)?;
                return self.index_value(ov, iv);
            }
            E::List(..) | E::Dict(..) | E::Tuple(..) | E::Slice(..) | E::Comp(..) | E::Fmt(..) | E::Concat(..) | E::ToDict(..) | E::ToSet(..) | E::In(..) | E::Attr(..) | E::New(..) | E::Method(..) => return self.eval_cold(e, base),
            E::Builtin(name, args) => {
                let vals = args.iter().map(|a| self.eval(a, base)).collect::<R<Vec<_>>>()?;
                self.builtin(name, vals)?
            }
        })
    }

    fn builtin(&mut self, name: &str, a: Vec<V>) -> R<V> {
        Ok(match name {
            "sqrt" | "sin" | "cos" | "tan" | "exp" | "log" | "floor" | "ceil" => {
                let x = a.first().and_then(num).ok_or_else(|| format!("TypeError: {name}() argument must be a number"))?;
                V::F(match name {
                    "sqrt" => x.sqrt(),
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "tan" => x.tan(),
                    "exp" => x.exp(),
                    "log" => x.ln(),
                    "floor" => x.floor(),
                    _ => x.ceil(),
                })
            }
            "__fmtdyn" => {
                let spec = show(&a[1]);
                if spec.is_empty() {
                    V::S(Rc::new(show(&a[0])))
                } else {
                    V::S(Rc::new(format_spec(&a[0], &spec)?))
                }
            }
            "__dmerge" => {
                let mut d: Vec<(V, V)> = Vec::new();
                for part in a.iter() {
                    let V::D(x) = part else { return Err("TypeError: ** argument must be a dict".into()) };
                    for (k, v) in x.borrow().iter() {
                        match d.iter().position(|(q, _)| veq(q, k)) {
                            Some(i) => d[i].1 = v.clone(),
                            None => d.push((k.clone(), v.clone())),
                        }
                    }
                }
                V::D(Rc::new(std::cell::RefCell::new(d)))
            }
            "__setop" => {
                let op = show(&a[0]);
                match (&a[1], &a[2]) {
                    (V::Z(x), V::Z(y)) => {
                        let (x, y) = (x.borrow().clone(), y.borrow().clone());
                        let v: Vec<V> = match op.as_str() {
                            "|" => dedup(x.into_iter().chain(y.into_iter()).collect()),
                            "&" => x.into_iter().filter(|e| y.iter().any(|o| veq(o, e))).collect(),
                            "^" => {
                                let mut r: Vec<V> = x.iter().filter(|e| !y.iter().any(|o| veq(o, e))).cloned().collect();
                                r.extend(y.iter().filter(|e| !x.iter().any(|o| veq(o, e))).cloned());
                                r
                            }
                            _ => return Err("TypeError: unsupported operand types for sets".into()),
                        };
                        V::Z(Rc::new(std::cell::RefCell::new(v)))
                    }
                    (l, r) => match (int_of(l), int_of(r)) {
                        (Some(x), Some(y)) => V::I(match op.as_str() {
                            "|" => x | y,
                            "&" => x & y,
                            "^" => x ^ y,
                            "<<" => x.checked_shl(y as u32).ok_or("OverflowError: shift")?,
                            _ => x >> (y.min(63) as u32),
                        }),
                        _ => return Err("TypeError: unsupported operand types".into()),
                    },
                }
            }
            "__reraise" => return Err(self.cur.clone().into()),
            "__raise" => {
                let name = show(&a[0]);
                return Err(match &a[1] {
                    V::N => name.into(),
                    m => format!("{name}: {}", show(m)).into(),
                });
            }
            "range" => {
                let (lo, hi, st) = match a.as_slice() {
                    [h] => (0, int_of(h).ok_or("TypeError: range() arguments must be integers")?, 1),
                    [l, h] => (int_of(l).ok_or("TypeError: range() arguments must be integers")?, int_of(h).ok_or("TypeError: range() arguments must be integers")?, 1),
                    [l, h, s] => (
                        int_of(l).ok_or("TypeError: range() arguments must be integers")?,
                        int_of(h).ok_or("TypeError: range() arguments must be integers")?,
                        int_of(s).ok_or("TypeError: range() arguments must be integers")?,
                    ),
                    _ => return Err("TypeError: range() takes 1 to 3 arguments".into()),
                };
                if st == 0 {
                    return Err("ValueError: range() arg 3 must not be zero".into());
                }
                let mut v = Vec::new();
                let mut i = lo;
                while (st > 0 && i < hi) || (st < 0 && i > hi) {
                    v.push(V::I(i));
                    i += st;
                    if v.len() > 100_000_000 {
                        return Err("MemoryError: range too large".into());
                    }
                }
                new_list(v)
            }
            "__copy" => deep_copy(a.first().unwrap_or(&V::N)),
            "simd" => V::X(Rc::new(a)),
            "print" => {
                let mut parts = Vec::new();
                for v in a.iter() {
                    parts.push(self.show_vm(v)?);
                }
                let s = parts.join(" ");
                writeln!(self.out, "{s}").map_err(|e| Er::from(e.to_string()))?;
                V::N
            }
            "int" => match a.first() {
                Some(V::F(f)) if f.is_finite() && f.abs() < 9.2e18 => V::I(f.trunc() as i64),
                Some(V::I(i)) => V::I(*i),
                Some(V::B(b)) => V::I(*b as i64),
                Some(V::S(s)) => V::I(s.trim().parse::<i64>().map_err(|_| format!("ValueError: invalid literal for int(): '{s}'"))?),
                _ => return Err("TypeError: int() argument".into()),
            },
            "float" => match a.first() {
                Some(v) if num(v).is_some() => V::F(num(v).unwrap_or(0.0)),
                Some(V::S(s)) => V::F(s.trim().parse::<f64>().map_err(|_| format!("ValueError: could not convert string to float: '{s}'"))?),
                _ => return Err("TypeError: float() argument".into()),
            },
            "str" => V::S(Rc::new(self.show_vm(a.first().unwrap_or(&V::N))?)),
            "bool" => V::B(truthy(a.first().unwrap_or(&V::B(false)))),
            "abs" => match a.first() {
                Some(V::I(i)) => V::I(i.checked_abs().ok_or("OverflowError: integer overflow")?),
                Some(V::F(f)) => V::F(f.abs()),
                _ => return Err("TypeError: abs() argument".into()),
            },
            "len" => match a.first() {
                Some(V::S(s)) => V::I(s.chars().count() as i64),
                Some(V::L(l)) => V::I(l.borrow().len() as i64),
                Some(V::X(x)) | Some(V::T(x)) => V::I(x.len() as i64),
                Some(V::D(d)) => V::I(d.borrow().len() as i64),
                Some(V::Z(l)) => V::I(l.borrow().len() as i64),
                _ => return Err("TypeError: len() argument".into()),
            },
            "sum" => {
                let Some(V::L(l)) = a.first() else { return Err("TypeError: sum() argument".into()) };
                let mut acc = V::I(0);
                for v in l.borrow().iter() {
                    acc = binop(Bop::Add, acc, v.clone())?;
                }
                acc
            }
            "set" => {
                let items: Vec<V> = match a.first() {
                    None => Vec::new(),
                    Some(V::L(l)) | Some(V::Z(l)) => l.borrow().clone(),
                    Some(V::T(l)) | Some(V::X(l)) => l.to_vec(),
                    Some(V::D(d)) => d.borrow().iter().map(|(k, _)| k.clone()).collect(),
                    Some(V::S(st)) => st.chars().map(|c| V::S(Rc::new(c.to_string()))).collect(),
                    _ => return Err("TypeError: set() argument".into()),
                };
                V::Z(Rc::new(std::cell::RefCell::new(dedup(items))))
            }
            "list" => match a.first() {
                Some(V::L(l)) | Some(V::Z(l)) => V::L(Rc::new(std::cell::RefCell::new(l.borrow().clone()))),
                Some(V::D(d)) => V::L(Rc::new(std::cell::RefCell::new(d.borrow().iter().map(|(k, _)| k.clone()).collect()))),
                None => V::L(Rc::new(std::cell::RefCell::new(Vec::new()))),
                _ => return Err("TypeError: list() argument".into()),
            },
            "sorted" => {
                let mut v: Vec<V> = match a.first() {
                    Some(V::L(l)) | Some(V::Z(l)) => l.borrow().clone(),
                    Some(V::T(l)) | Some(V::X(l)) => l.to_vec(),
                    Some(V::D(d)) => d.borrow().iter().map(|(k, _)| k.clone()).collect(),
                    Some(V::S(st)) => st.chars().map(|c| V::S(Rc::new(c.to_string()))).collect(),
                    _ => return Err("TypeError: sorted() argument".into()),
                };
                sort_values(&mut v)?;
                V::L(Rc::new(std::cell::RefCell::new(v)))
            }
            "min" | "max" => {
                if a.is_empty() {
                    return Err(format!("TypeError: {name}() expected arguments").into());
                }
                let mut best = a[0].clone();
                for v in &a[1..] {
                    let better = if name == "min" { compare(Cop::Lt, v, &best)? } else { compare(Cop::Gt, v, &best)? };
                    if better {
                        best = v.clone();
                    }
                }
                best
            }
            _ => return Err(format!("unknown builtin {name}").into()),
        })
    }

    #[inline(never)]
    fn exec_cold(&mut self, s: &St, base: usize) -> R<Flow> {
        match s {
                St::Raise(e) => {
                    self.eval(e, base)?;
                }
                St::Try(body, hs, els, fin) => {
                    let mut res = self.exec_block(body, base); let body_ok = res.is_ok();
                    if let Err(e) = &res {
                        if !hs.is_empty() && !e.0.starts_with("RecursionError") {
                            self.cur = (*e.0).clone();
                            let msg = e.0.as_str();
                            let msg = match msg.split_once(": ") {
                                Some((cls, rest)) if cls.ends_with("Error") || cls == "Exception" || self.p.exc_parent.contains_key(cls) => rest,
                                _ => msg,
                            };
                            let cls_name = e.0.split(':').next().unwrap_or("").trim().to_string();
                            // ancestors of the raised class (user-defined hierarchy)
                            let mut chain = vec![cls_name.clone()];
                            while let Some(par) = self.p.exc_parent.get(chain.last().unwrap()) {
                                if chain.contains(par) {
                                    break;
                                }
                                chain.push(par.clone());
                            }
                            let hit = hs.iter().find(|(tys, _, _)| {
                                tys.is_empty()
                                    || tys.iter().any(|t| {
                                        t == "Exception"
                                            || t == "BaseException"
                                            || chain.contains(t)
                                            || (t == "ArithmeticError" && (cls_name == "ZeroDivisionError" || cls_name == "OverflowError"))
                                            || (t == "LookupError" && (cls_name == "KeyError" || cls_name == "IndexError"))
                                    })
                            });
                            if let Some((_, tg, hb)) = hit {
                            if let Some(t) = tg {
                                let v = V::S(Rc::new(msg.to_string()));
                                match t {
                                    Target::Local(k) => self.stack[base + *k] = v,
                                    Target::Global(k) => self.globals[*k] = v,
                                }
                            }
                            res = self.exec_block(hb, base);
                            }
                        }
                    }
                    if body_ok && matches!(&res, Ok(Flow::Next)) && !els.is_empty() {
                        res = self.exec_block(els, base);
                    }
                    let fr = self.exec_block(fin, base)?;
                    match res? {
                        Flow::Next => match fr {
                            Flow::Next => {}
                            other => return Ok(other),
                        },
                        other => return Ok(other),
                    }
                }
                St::Unpack(tg, e) => {
                    let v = self.eval(e, base)?;
                    let parts: Vec<V> = match &v {
                        V::T(p) => p.to_vec(),
                        V::L(p) => p.borrow().clone(),
                        _ => return Err("TypeError: cannot unpack non-sequence".into()),
                    };
                    if parts.len() != tg.len() {
                        return Err(format!("ValueError: expected {} values to unpack, got {}", tg.len(), parts.len()).into());
                    }
                    for (t, x) in tg.iter().zip(parts) {
                        match t {
                            Target::Local(k) => self.stack[base + *k] = x,
                            Target::Global(k) => self.globals[*k] = x,
                        }
                    }
                }
                St::ForIn(tg, it, b) => {
                    let itv = self.eval(it, base)?;
                    let items: Vec<V> = match &itv {
                        V::L(l) => l.borrow().clone(),
                        V::T(l) => l.to_vec(),
                        V::D(d) => d.borrow().iter().map(|(k, _)| k.clone()).collect(),
                        V::Z(l) => l.borrow().clone(),
                        V::S(st) => st.chars().map(|c| V::S(Rc::new(c.to_string()))).collect(),
                        _ => return Err("TypeError: object is not iterable".into()),
                    };
                    let mut broke = false;
                    for item in items {
                        let parts: Vec<V> = if tg.len() == 1 {
                            vec![item]
                        } else {
                            match &item {
                                V::L(p) if p.borrow().len() == tg.len() => p.borrow().clone(),
                                V::T(p) if p.len() == tg.len() => p.to_vec(),
                                _ => return Err("ValueError: cannot unpack for-loop item".into()),
                            }
                        };
                        for (t, v) in tg.iter().zip(parts) {
                            match t {
                                Target::Local(k) => self.stack[base + *k] = v,
                                Target::Global(k) => self.globals[*k] = v,
                            }
                        }
                        match self.exec_block(b, base)? {
                            Flow::Break => {
                                broke = true;
                                break;
                            }
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            _ => {}
                        }
                    }
                    self.brk = broke;
                }
                St::SetAttr(o, name, e) => {
                    let ov = self.eval(o, base)?;
                    let v = self.eval(e, base)?;
                    let V::O(ob) = &ov else { return Err(format!("AttributeError: cannot set '{name}'").into()) };
                    let idx = self.p.classes[ob.cls].fields.iter().position(|f| f == name).ok_or_else(|| format!("AttributeError: no attribute '{name}'"))?;
                    ob.f.borrow_mut()[idx] = v;
                }
            _ => {}
        }
        Ok(Flow::Next)
    }

    fn exec_block(&mut self, body: &[St], base: usize) -> R<Flow> {
        for s in body {
            match s {
                St::SetLocal(i, e) => {
                    let v = self.eval(e, base)?;
                    self.stack[base + *i] = v;
                }
                St::SetGlobal(i, e) => {
                    let v = self.eval(e, base)?;
                    self.globals[*i] = v;
                }
                St::Multi(ts, es) => {
                    let mut vals = Vec::with_capacity(es.len());
                    for e in es {
                        vals.push(self.eval(e, base)?);
                    }
                    for (t, v) in ts.iter().zip(vals) {
                        match t {
                            Target::Local(k) => self.stack[base + *k] = v,
                            Target::Global(k) => self.globals[*k] = v,
                        }
                    }
                }
                St::Expr(e) => {
                    self.eval(e, base)?;
                }
                St::If(c, a, b) => {
                    let f = if self.cond(c, base)? { self.exec_block(a, base)? } else { self.exec_block(b, base)? };
                    if !matches!(f, Flow::Next) {
                        return Ok(f);
                    }
                }
                St::While(c, b) => {
                    let mut broke = false;
                    while self.cond(c, base)? {
                        self.fuel = self.fuel.checked_sub(1).ok_or_else(|| Er::from("LimitError: step limit exceeded"))?;
                        match self.exec_block(b, base)? {
                            Flow::Break => {
                                broke = true;
                                break;
                            }
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            _ => {}
                        }
                    }
                    self.brk = broke;
                }
                St::LoopElse(lp, orelse) => {
                    match self.exec_block(std::slice::from_ref(&**lp), base)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                    if !self.brk {
                        match self.exec_block(orelse, base)? {
                            Flow::Next => {}
                            f => return Ok(f),
                        }
                    }
                }
                St::ForRange(t, start, stop, step, b) => {
                    let (Some(mut i), Some(end), Some(st)) =
                        (int_of(&self.eval(start, base)?), int_of(&self.eval(stop, base)?), int_of(&self.eval(step, base)?))
                    else {
                        return Err("TypeError: range() arguments must be integers".into());
                    };
                    if st == 0 {
                        return Err("ValueError: range() arg 3 must not be zero".into());
                    }
                    let mut broke = false;
                    while (st > 0 && i < end) || (st < 0 && i > end) {
                        self.fuel = self.fuel.checked_sub(1).ok_or_else(|| Er::from("LimitError: step limit exceeded"))?;
                        match t {
                            Target::Local(k) => self.stack[base + *k] = V::I(i),
                            Target::Global(k) => self.globals[*k] = V::I(i),
                        }
                        match self.exec_block(b, base)? {
                            Flow::Break => {
                                broke = true;
                                break;
                            }
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            _ => {}
                        }
                        i = match i.checked_add(st) {
                            Some(n) => n,
                            None => break,
                        };
                    }
                    self.brk = broke;
                }
                St::SetIndex(o, i, e) => {
                    if matches!(o, E::Local(_) | E::Global(_)) {
                        let iv = self.eval(i, base)?;
                        let v = self.eval(e, base)?;
                        if let V::I(ix) = iv {
                            let slot = match o {
                                E::Local(k) => &self.stack[base + *k],
                                E::Global(k) => &self.globals[*k],
                                _ => unreachable!(),
                            };
                            if let V::L(l) = slot {
                                let mut l = l.borrow_mut();
                                let n = l.len();
                                match norm_index(ix, n) {
                                    Some(k) => l[k] = v,
                                    None => return Err("IndexError: list assignment index out of range".into()),
                                }
                                continue;
                            }
                        }
                        let ov = self.eval(o, base)?;
                        self.set_index(ov, iv, v)?;
                        continue;
                    }
                    let ov = self.eval(o, base)?;
                    let iv = self.eval(i, base)?;
                    let v = self.eval(e, base)?;
                    self.set_index(ov, iv, v)?;
                }
                St::Raise(..) | St::Try(..) | St::Unpack(..) | St::ForIn(..) | St::SetAttr(..) => match self.exec_cold(s, base)? {
                    Flow::Next => {}
                    f => return Ok(f),
                },
                St::Return(e) => {
                    let v = match e {
                        Some(e) => self.eval(e, base)?,
                        None => V::N,
                    };
                    return Ok(Flow::Return(v));
                }
                St::Break => return Ok(Flow::Break),
                St::Continue => return Ok(Flow::Continue),
            }
        }
        Ok(Flow::Next)
    }
}

fn sort_values(v: &mut Vec<V>) -> R<()> {
    let mut err: Option<Er> = None;
    v.sort_by(|a, b| match compare(Cop::Lt, a, b) {
        Ok(true) => std::cmp::Ordering::Less,
        Ok(false) => match compare(Cop::Gt, a, b) {
            Ok(true) => std::cmp::Ordering::Greater,
            Ok(false) => std::cmp::Ordering::Equal,
            Err(e) => {
                err = Some(e);
                std::cmp::Ordering::Equal
            }
        },
        Err(e) => {
            err = Some(e);
            std::cmp::Ordering::Equal
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn new_list(v: Vec<V>) -> V {
    V::L(Rc::new(std::cell::RefCell::new(v)))
}

fn builtin_method(o: &V, name: &str, a: Vec<V>) -> R<V> {
    let nomethod = || Er::from(format!("AttributeError: no method '{name}'"));
    match o {
        V::L(l) => Ok(match (name, a.as_slice()) {
            ("append", [x]) => {
                l.borrow_mut().push(x.clone());
                V::N
            }
            ("extend", [V::L(o2)]) => {
                let add = o2.borrow().clone();
                l.borrow_mut().extend(add);
                V::N
            }
            ("insert", [i, x]) => {
                let n = l.borrow().len() as i64;
                let i = int_of(i).ok_or("TypeError: index must be int")?;
                let k = if i < 0 { (i + n).max(0) } else { i.min(n) } as usize;
                l.borrow_mut().insert(k, x.clone());
                V::N
            }
            ("pop", rest) if rest.len() <= 1 => {
                let n = l.borrow().len();
                if n == 0 {
                    return Err("IndexError: pop from empty list".into());
                }
                let i = match rest.first() {
                    Some(v) => int_of(v).ok_or("TypeError: index must be int")?,
                    None => -1,
                };
                let k = norm_index(i, n).ok_or("IndexError: pop index out of range")?;
                l.borrow_mut().remove(k)
            }
            ("clear", []) => {
                l.borrow_mut().clear();
                V::N
            }
            ("reverse", []) => {
                l.borrow_mut().reverse();
                V::N
            }
            ("sort", []) => {
                let mut v = l.borrow().clone();
                sort_values(&mut v)?;
                *l.borrow_mut() = v;
                V::N
            }
            ("index", [x]) => match l.borrow().iter().position(|e| veq(e, x)) {
                Some(k) => V::I(k as i64),
                None => return Err("ValueError: value is not in list".into()),
            },
            ("count", [x]) => V::I(l.borrow().iter().filter(|e| veq(e, x)).count() as i64),
            _ => return Err(nomethod()),
        }),
        V::Z(z) => Ok(match (name, a.as_slice()) {
            ("add", [x]) => {
                if !z.borrow().iter().any(|e| veq(e, x)) {
                    z.borrow_mut().push(x.clone());
                }
                V::N
            }
            ("remove", [x]) => {
                let pos = z.borrow().iter().position(|e| veq(e, x));
                match pos {
                    Some(i) => {
                        z.borrow_mut().remove(i);
                        V::N
                    }
                    None => return Err(format!("KeyError: {}", repr(x)).into()),
                }
            }
            ("discard", [x]) => {
                let pos = z.borrow().iter().position(|e| veq(e, x));
                if let Some(i) = pos {
                    z.borrow_mut().remove(i);
                }
                V::N
            }
            ("clear", []) => {
                z.borrow_mut().clear();
                V::N
            }
            ("union", [V::L(o)]) | ("union", [V::Z(o)]) => {
                let mut v = z.borrow().clone();
                v.extend(o.borrow().iter().cloned());
                V::Z(Rc::new(std::cell::RefCell::new(dedup(v))))
            }
            ("intersection", [V::L(o)]) | ("intersection", [V::Z(o)]) => {
                let v: Vec<V> = z.borrow().iter().filter(|e| o.borrow().iter().any(|x| veq(x, e))).cloned().collect();
                V::Z(Rc::new(std::cell::RefCell::new(v)))
            }
            ("difference", [V::L(o)]) | ("difference", [V::Z(o)]) => {
                let v: Vec<V> = z.borrow().iter().filter(|e| !o.borrow().iter().any(|x| veq(x, e))).cloned().collect();
                V::Z(Rc::new(std::cell::RefCell::new(v)))
            }
            _ => return Err(nomethod()),
        }),
        V::D(d) => Ok(match (name, a.as_slice()) {
            ("get", [k]) | ("get", [k, _]) => {
                let dflt = a.get(1).cloned().unwrap_or(V::N);
                let r = d.borrow().iter().find(|(x, _)| veq(x, k)).map(|(_, v)| v.clone());
                r.unwrap_or(dflt)
            }
            ("keys", []) => new_list(d.borrow().iter().map(|(k, _)| k.clone()).collect()),
            ("values", []) => new_list(d.borrow().iter().map(|(_, v)| v.clone()).collect()),
            ("items", []) => new_list(d.borrow().iter().map(|(k, v)| new_list(vec![k.clone(), v.clone()])).collect()),
            ("pop", [k]) => {
                let pos = d.borrow().iter().position(|(x, _)| veq(x, k));
                match pos {
                    Some(i) => d.borrow_mut().remove(i).1,
                    None => return Err(format!("KeyError: {}", repr(k)).into()),
                }
            }
            ("clear", []) => {
                d.borrow_mut().clear();
                V::N
            }
            _ => return Err(nomethod()),
        }),
        V::T(t) => Ok(match (name, a.as_slice()) {
            ("count", [x]) => V::I(t.iter().filter(|e| veq(e, x)).count() as i64),
            ("index", [x]) => match t.iter().position(|e| veq(e, x)) {
                Some(k) => V::I(k as i64),
                None => return Err("ValueError: value is not in tuple".into()),
            },
            _ => return Err(nomethod()),
        }),
        V::X(x) => Ok(match (name, a.as_slice()) {
            ("reduce_add", []) => {
                let mut acc = x.first().cloned().ok_or("ValueError: empty SIMD")?;
                for v in x.iter().skip(1) {
                    acc = binop(Bop::Add, acc, v.clone())?;
                }
                acc
            }
            ("reduce_mul", []) => {
                let mut acc = x.first().cloned().ok_or("ValueError: empty SIMD")?;
                for v in x.iter().skip(1) {
                    acc = binop(Bop::Mul, acc, v.clone())?;
                }
                acc
            }
            _ => return Err(nomethod()),
        }),
        V::S(s) => Ok(match (name, a.as_slice()) {
            ("upper", []) => V::S(Rc::new(s.to_uppercase())),
            ("lower", []) => V::S(Rc::new(s.to_lowercase())),
            ("strip", []) => V::S(Rc::new(s.trim().to_string())),
            ("startswith", [V::S(p)]) => V::B(s.starts_with(p.as_str())),
            ("endswith", [V::S(p)]) => V::B(s.ends_with(p.as_str())),
            // 1バイトの検索語は open-cpu の find_byte / count_byte(AVX-512BW > AVX2 > NEON > スカラーを実行時に自動選択)
            ("find", [V::S(p)]) => {
                let pos = if p.len() == 1 { open_cpu::find_byte(s.as_bytes(), p.as_bytes()[0]) } else { s.find(p.as_str()) };
                V::I(pos.map(|b| s[..b].chars().count() as i64).unwrap_or(-1))
            }
            ("count", [V::S(p)]) => V::I(if p.is_empty() {
                s.chars().count() as i64 + 1
            } else if p.len() == 1 {
                open_cpu::count_byte(s.as_bytes(), p.as_bytes()[0]) as i64
            } else {
                s.matches(p.as_str()).count() as i64
            }),
            ("replace", [V::S(x), V::S(y)]) => V::S(Rc::new(s.replace(x.as_str(), y.as_str()))),
            ("split", []) => new_list(s.split_whitespace().map(|w| V::S(Rc::new(w.to_string()))).collect()),
            ("split", [V::S(sep)]) if !sep.is_empty() => new_list(s.split(sep.as_str()).map(|w| V::S(Rc::new(w.to_string()))).collect()),
            ("join", [V::L(l)]) => {
                let mut parts = Vec::new();
                for v in l.borrow().iter() {
                    match v {
                        V::S(x) => parts.push(x.as_str().to_string()),
                        _ => return Err("TypeError: join() expects strings".into()),
                    }
                }
                V::S(Rc::new(parts.join(s.as_str())))
            }
            _ => return Err(nomethod()),
        }),
        _ => Err(nomethod()),
    }
}

fn run_source_inner(src: &str) -> R<()> {
    let prog = compile(src)?;
    let mut vm = Vm { p: &prog, globals: vec![V::N; prog.nglobals], depth: 0, stack: Vec::new(), brk: false, cur: String::new(), out: std::io::BufWriter::new(std::io::stdout()), fuel: u64::MAX };
    let r = vm.exec_block(&prog.main_body, 0).map(|_| ());
    let r = r.and_then(|_| match prog.entry {
        Some(fi) => {
            let f = &prog.funcs[fi];
            vm.stack.resize(f.nlocals, V::N);
            vm.exec_block(&f.body, 0).map(|_| ())
        }
        None => Ok(()),
    });
    let _ = vm.out.flush();
    r
}

pub mod mojo_syntax;
pub mod update;

/// Mojo mode (.mojo): assignment copies lists/dicts/structs.
pub fn set_copy_mode(on: bool) {
    COPY_MODE.store(on, AOrd::Relaxed);
}

/// Plain values crossing the embedding boundary (the `mfastapi.handler.v1` scalar subset).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Null,
}

/// A compiled program that can be called many times (state of globals is per call).
pub struct CompiledProgram(Program);

/// Compile Python-syntax source. Errors are returned as text.
pub fn compile_source(src: &str) -> Result<CompiledProgram, String> {
    compile(src).map(CompiledProgram).map_err(|e| e.to_string())
}

impl CompiledProgram {
    /// Call function `name` with scalar arguments. Run on a thread with a large stack for deep recursion.
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        self.call_limited(name, args, &Limits::default())
    }

    /// Like `call`, with a loop-iteration budget (`max_steps`) so runaway handlers stop with `LimitError`.
    pub fn call_limited(&self, name: &str, args: &[Value], lim: &Limits) -> Result<Value, String> {
        let p = &self.0;
        let fi = *p.names.get(name).ok_or_else(|| format!("NameError: no function {name}"))?;
        let mut vm = Vm { p, globals: vec![V::N; p.nglobals], depth: 0, stack: Vec::new(), brk: false, cur: String::new(), out: std::io::BufWriter::new(std::io::stdout()), fuel: lim.max_steps };
        vm.exec_block(&p.main_body, 0).map_err(|e| e.to_string())?;
        let vals = args
            .iter()
            .map(|a| match a {
                Value::Int(i) => V::I(*i),
                Value::Float(f) => V::F(*f),
                Value::Bool(b) => V::B(*b),
                Value::Str(s) => V::S(Rc::new(s.clone())),
                Value::Null => V::N,
            })
            .collect();
        let r = vm.call_values(fi, vals).map_err(|e| e.to_string())?;
        let _ = vm.out.flush();
        match r {
            V::I(i) => Ok(Value::Int(i)),
            V::F(f) => Ok(Value::Float(f)),
            V::B(b) => Ok(Value::Bool(b)),
            V::S(s) => Ok(Value::Str((*s).clone())),
            V::N => Ok(Value::Null),
            other => Err(format!("TypeError: unsupported return type: {}", repr(&other))),
        }
    }
}

/// Run a whole program (used by `rmojo run`).
pub fn run_source(src: &str) -> Result<(), String> {
    run_source_inner(src).map_err(|e| e.to_string())
}

/// Execution limits for embedded calls.
#[derive(Clone, Debug)]
pub struct Limits {
    /// Maximum number of loop iterations (while / for) across the whole call, including module-level code.
    pub max_steps: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Limits { max_steps: u64::MAX }
    }
}
