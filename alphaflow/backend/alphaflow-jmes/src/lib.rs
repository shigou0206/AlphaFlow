#![cfg_attr(feature = "specialized", feature(specialization))]

pub use crate::errors::{ErrorReason, JmespathError, RuntimeError};
pub use crate::parser::{parse, ParseResult};
pub use crate::runtime::Runtime;
pub use crate::variable::Variable;

pub mod ast;
pub mod functions;

use serde::ser;
use std::fmt;

use lazy_static::*;

use crate::ast::Ast;
use crate::interpreter::{interpret, SearchResult};

mod errors;
mod interpreter;
mod lexer;
mod parser;
mod runtime;
mod variable;

lazy_static! {
    pub static ref DEFAULT_RUNTIME: Runtime = {
        let mut runtime = Runtime::new();
        runtime.register_builtin_functions();
        runtime
    };
}

use std::ops::Deref;

// 1) 根据特性同步/异步，选择 Rc / Arc。
#[cfg(not(feature="sync"))]
use std::rc::Rc as Container;
#[cfg(feature="sync")]
use std::sync::Arc as Container;

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// `Arcvar` newtype 包裹 `Container<Variable>`.
// 让它具备Clone+Debug+PartialEq(后面会手动/自动)...
#[derive(Clone)]
pub struct Arcvar(pub Container<Variable>);

impl Arcvar {
    /// 构造函数：将一个 Variable 包裹进 Arcvar
    pub fn new(var: Variable) -> Self {
        Arcvar(Container::new(var))
    }
}

// 2) Deref: 让 `*arcvar` -> `&Variable`
impl Deref for Arcvar {
    type Target = Variable;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// 3) Debug: 如果你想自动推导，也可 #[derive(Debug)]。
//    但 Container<Variable> 也要 Debug。若 Variable: Debug，就没问题。
impl fmt::Debug for Arcvar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 例如把内部 Debug 打印
        write!(f, "Arcvar({:?})", self.0)
    }
}

// 4) PartialEq + Eq：让 Arcvar 可以做 ==/!=，且满足完全相等的语义
impl PartialEq for Arcvar {
    fn eq(&self, other: &Self) -> bool {
        // 比较底层 Variable 的值
        *self.0 == *other.0
    }
}
impl Eq for Arcvar {}

// 5) PartialOrd + Ord：让 Arcvar 可以做 <,>,<=,>=,sort等。 
//   必须 Variable 也要支持 PartialOrd/Ord，否则你这里实现也会报错。
use std::cmp::Ordering;

impl PartialOrd for Arcvar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // 假设 Variable: PartialOrd
        self.0.partial_cmp(&other.0)
    }
}
impl Ord for Arcvar {
    fn cmp(&self, other: &Self) -> Ordering {
        // 假设 Variable: Ord
        self.0.cmp(&other.0)
    }
}

// 6) 实现 serde::Serialize: 
impl Serialize for Arcvar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 若 Container<Variable> & Variable: Serialize
        self.0.serialize(serializer)
    }
}

// 7) 实现 serde::Deserialize:
impl<'de> Deserialize<'de> for Arcvar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 先把 JSON -> Variable
        let var = Variable::deserialize(deserializer)?;
        // 再包装进 Arcvar
        Ok(Arcvar::new(var))
    }
}
use std::convert::AsRef;

impl AsRef<Variable> for Arcvar {
    fn as_ref(&self) -> &Variable {
        &self.0 // 或者 `&*self.0`，其实都一样
    }
}

#[inline]
pub fn compile(expression: &str) -> Result<Expression<'static>, JmespathError> {
    DEFAULT_RUNTIME.compile(expression)
}

/// Implement `ToString` or custom methods as you like. For example:
/// impl std::fmt::Display for Arcvar { ... } // optional

/// If you want to add is_null(), as_string(), etc., typically you'd do that
/// in `variable.rs` for the `Variable` type. Then `Arcvar` calls `self.0.as_ref()` to get `&Variable`.

/// The `ToJmespath` trait is updated to return Arcvar instead of Rcvar.
/// So all searching ends up with Arcvar as the final result.
#[cfg_attr(
    feature = "specialized",
    doc = "\
There is a generic serde Serialize implementation, and since this
documentation was compiled with the `specialized` feature turned
**on**, there are also a number of specialized implementations for
`ToJmespath` built into the library that should work for most
cases."
)]
#[cfg_attr(
    not(feature = "specialized"),
    doc = "\
There is a generic serde Serialize implementation. Since this
documentation was compiled with the `specialized` feature turned
**off**, this is the only implementation available.
"
)]
pub trait ToJmespath {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError>;
}

/// Implement `ToJmespath` for all serde-serializable types, returning an `Arcvar`.
impl<'a, T: ser::Serialize> ToJmespath for T {
    #[cfg(not(feature = "specialized"))]
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Variable::from_serializable(self).map(Arcvar::new)
    }

    #[cfg(feature = "specialized")]
    default fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        let var = Variable::from_serializable(self)?;
        Ok(Arcvar::new(var))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for Value {
    #[inline]
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        let var: Variable = self.try_into()?;
        Ok(Arcvar::new(var))
    }
}
#[cfg(feature = "specialized")]
impl<'a> ToJmespath for &'a Value {
    #[inline]
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        let var: Variable = self.try_into()?;
        Ok(Arcvar::new(var))
    }
}

#[cfg(feature = "specialized")]
/// Identity coercion.
impl ToJmespath for Arcvar {
    #[inline]
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(self)
    }
}
#[cfg(feature = "specialized")]
impl<'a> ToJmespath for &'a Arcvar {
    #[inline]
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(self.clone())
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for Variable {
    #[inline]
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(self))
    }
}
#[cfg(feature = "specialized")]
impl<'a> ToJmespath for &'a Variable {
    #[inline]
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(self.clone()))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for String {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::String(self)))
    }
}
#[cfg(feature = "specialized")]
impl<'a> ToJmespath for &'a str {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::String(self.to_owned())))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for i8 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for i16 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for i32 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for i64 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for u8 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for u16 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for u32 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for u64 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for isize {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for usize {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Number(serde_json::Number::from(self))))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for f32 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        (self as f64).to_jmespath()
    }
}
#[cfg(feature = "specialized")]
impl ToJmespath for f64 {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        let num = serde_json::Number::from_f64(self).ok_or_else(|| {
            JmespathError::new(
                "",
                0,
                ErrorReason::Parse(format!("Cannot parse {} into a Number", self)),
            )
        })?;
        Ok(Arcvar::new(Variable::Number(num)))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for () {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Null))
    }
}

#[cfg(feature = "specialized")]
impl ToJmespath for bool {
    fn to_jmespath(self) -> Result<Arcvar, JmespathError> {
        Ok(Arcvar::new(Variable::Bool(self)))
    }
}

/// A compiled JMESPath expression.
///
/// The compiled expression can be used multiple times without incurring
/// the cost of re-parsing the expression each time. The expression may
/// be shared between threads if JMESPath is compiled with the `sync`
/// feature, which forces the use of an `Arc` instead of an `Rc` for
/// runtime variables.
#[derive(Clone)]
pub struct Expression<'a> {
    ast: Ast,
    expression: String,
    runtime: &'a Runtime,
}

impl<'a> Expression<'a> {
    /// Creates a new JMESPath expression.
    ///
    /// Normally you will create expressions using either `jmespath::compile()`
    /// or using a jmespath::Runtime.
    #[inline]
    pub fn new<S>(expression: S, ast: Ast, runtime: &'a Runtime) -> Expression<'a>
    where
        S: Into<String>,
    {
        Expression {
            expression: expression.into(),
            ast,
            runtime,
        }
    }

    /// Returns the result of searching data with the compiled expression.
    ///
    /// The SearchResult contains an `Arcvar` reference-counted
    /// `Variable`. This value can be used directly like a JSON object.
    /// Alternatively, `Variable` does implement Serde serialization and
    /// deserialization, so it can easily be marshalled to another type.
    pub fn search<T: ToJmespath>(&self, data: T) -> SearchResult {
        let mut ctx = Context::new(&self.expression, self.runtime);
        // interpret(...) returns `Result<Arcvar, JmespathError>`
        interpret(&data.to_jmespath()?, &self.ast, &mut ctx)
    }

    /// Returns the JMESPath expression from which the Expression was compiled.
    ///
    /// Note that this is the same value that is returned by calling
    /// `to_string`.
    pub fn as_str(&self) -> &str {
        &self.expression
    }

    /// Returns the AST of the parsed JMESPath expression.
    ///
    /// This can be useful for debugging purposes, caching, etc.
    pub fn as_ast(&self) -> &Ast {
        &self.ast
    }
}

impl<'a> fmt::Display for Expression<'a> {
    /// Shows the jmespath expression as a string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'a> fmt::Debug for Expression<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<'a> PartialEq for Expression<'a> {
    fn eq(&self, other: &Expression<'_>) -> bool {
        self.as_str() == other.as_str()
    }
}

/// Context object used for error reporting.
///
/// The Context struct is mostly used when interacting between the
/// interpreter and function implementations. Unless you're writing custom
/// JMESPath functions, this struct is an implementation detail.
pub struct Context<'a> {
    /// Expression string that is being interpreted.
    pub expression: &'a str,
    /// JMESPath runtime used to compile the expression and call functions.
    pub runtime: &'a Runtime,
    /// Ast offset that is currently being evaluated.
    pub offset: usize,
}

impl<'a> Context<'a> {
    /// Create a new context struct.
    #[inline]
    pub fn new(expression: &'a str, runtime: &'a Runtime) -> Context<'a> {
        Context {
            expression,
            runtime,
            offset: 0,
        }
    }
}

#[cfg(test)]
mod test {
    use super::ast::Ast;
    use super::*;

    #[test]
    fn formats_expression_as_string_or_debug() {
        let expr = compile("foo | baz").unwrap();
        assert_eq!("foo | baz/foo | baz", format!("{}/{:?}", expr, expr));
    }

    #[test]
    fn implements_partial_eq() {
        let a = compile("@").unwrap();
        let b = compile("@").unwrap();
        assert!(a == b);
    }

    #[test]
    fn can_evaluate_jmespath_expression() {
        let expr = compile("foo.bar").unwrap();
        let var = Variable::from_json("{\"foo\":{\"bar\":true}}").unwrap();
        // originally: `assert_eq!(Rcvar::new(Variable::Bool(true)), expr.search(var).unwrap());`
        // now: 
        assert_eq!(
            Arcvar::new(Variable::Bool(true)), 
            expr.search(var).unwrap()
        );
    }

    #[test]
    fn can_get_expression_ast() {
        let expr = compile("foo").unwrap();
        assert_eq!(
            &Ast::Field {
                offset: 0,
                name: "foo".to_string(),
            },
            expr.as_ast()
        );
    }

    #[test]
    fn test_creates_arcvar_from_tuple_serialization() {
        use super::ToJmespath;
        let t = (true, false);
        // original was: t.to_jmespath().unwrap().to_string()
        // now we have an Arcvar, so let's do:
        assert_eq!(
            "[true,false]",
            t.to_jmespath().unwrap().to_string()
        );
        // you'd need to ensure `Arcvar::to_string()` or `Variable::to_string()` is implemented 
        // to get a JSON string from the underlying value. 
    }

    #[test]
    fn expression_clone() {
        let expr = compile("foo").unwrap();
        let _ = expr.clone();
    }

    #[test]
    fn test_invalid_number() {
        let _ = compile("6455555524");
    }
}