//! JMESPath compliance tests.
//!
//! Test cases are generated using build.rs. This may eventually be exposed
//! as a library (leading to possibilities like a compliance test runner CLI).

use serde_json::Value;
use std::fmt;

use alphaflow_jmes::{compile, Expression, Arcvar, RuntimeError, Variable};

/// Available benchmark types.
pub enum BenchType {
    /// The benchmark must only parse an expression.
    Parse,
    /// The benchmark must benchmark only the interpreter
    Interpret,
    /// The benchmark must benchmark both the parser and interpreter.
    Full,
}

impl BenchType {
    /// Try to create a benchmark assertion from a JSON value.
    fn from_json(bench_type: &Value) -> Result<Self, TestCaseError> {
        bench_type
            .as_str()
            .ok_or(TestCaseError::BenchIsNotString)
            .and_then(|b| match b {
                "parse" => Ok(BenchType::Parse),
                "interpret" => Ok(BenchType::Interpret),
                "full" => Ok(BenchType::Full),
                other => Err(TestCaseError::UnknownBenchType(other.to_string())),
            })
    }
}

impl fmt::Display for BenchType {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            BenchType::Parse => write!(fmt, "parse"),
            BenchType::Interpret => write!(fmt, "interpret"),
            BenchType::Full => write!(fmt, "full"),
        }
    }
}

/// Available error types.
pub enum ErrorType {
    /// Ensures that the expression fails due to an invalid-arity error.
    InvalidArity,
    /// Ensures that the expression fails due to an invalid-type error.
    InvalidType,
    /// Ensures that the expression fails due to an invalid-value error.
    InvalidSlice,
    /// Ensures that the expression fails due to an unknown-function error.
    UnknownFunction,
    /// Ensures that an expression cannot be parsed due to a syntax error.
    SyntaxError,
}

impl ErrorType {
    /// Try to create an error assertion from a JSON value.
    fn from_json(error_type: &Value) -> Result<Self, TestCaseError> {
        error_type
            .as_str()
            .ok_or(TestCaseError::ErrorIsNotString)
            .and_then(|b| match b {
                "syntax" => Ok(ErrorType::SyntaxError),
                "invalid-type" => Ok(ErrorType::InvalidType),
                "invalid-value" => Ok(ErrorType::InvalidSlice),
                "invalid-arity" => Ok(ErrorType::InvalidArity),
                "unknown-function" => Ok(ErrorType::UnknownFunction),
                other => Err(TestCaseError::UnknownErrorType(other.to_string())),
            })
    }
}

impl fmt::Display for ErrorType {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use self::ErrorType::*;
        match self {
            InvalidArity => write!(fmt, "invalid-arity"),
            InvalidType => write!(fmt, "invalid-type"),
            InvalidSlice => write!(fmt, "invalid-value"),
            UnknownFunction => write!(fmt, "unknown-function"),
            SyntaxError => write!(fmt, "syntax"),
        }
    }
}

/// Test case assertions.
pub enum Assertion {
    /// Ensures that a test fails with a particular error type.
    Error(ErrorType),
    /// Ignores the result and marks the test as a benchmark
    Bench(BenchType),
    /// Ensures that the expression is parsed and returns an expected result.
    ///
    /// **改**：用 Arcvar 而不是 Rcvar
    ValidResult(Arcvar),
}

impl Assertion {
    /// Runs the assertion of a test case
    pub fn assert(&self, suite: &str, case: &TestCase, given: Arcvar) -> Result<(), String> {
        match self {
            Assertion::Bench(_) => Ok(()),
            Assertion::ValidResult(expected_result) => {
                let expr = self.try_parse(suite, case)?;
                match expr.search(given) {
                    Err(e) => Err(self.err_message(suite, case, format!("{}", e))),
                    Ok(r) => {
                        // r, expected_result 都是 Arcvar
                        // 比较其内部 Variable
                        if *r.0 == *expected_result.0 {
                            Ok(())
                        } else {
                            Err(self.err_message(
                                suite,
                                case,
                                format!("{:?}, {}", r, expr.as_ast()),
                            ))
                        }
                    }
                }
            }
            Assertion::Error(error_type) => {
                use alphaflow_jmes::ErrorReason::*;
                let result = self.try_parse(suite, case);
                match error_type {
                    ErrorType::InvalidArity => match result?.search(given).map_err(|e| e.reason) {
                        Err(Runtime(RuntimeError::NotEnoughArguments { .. })) => Ok(()),
                        Err(Runtime(RuntimeError::TooManyArguments { .. })) => Ok(()),
                        Err(e) => Err(self.err_message(suite, case, format!("{}", e))),
                        Ok(r) => Err(self.err_message(suite, case, r.to_string())),
                    },
                    ErrorType::InvalidType => match result?.search(given).map_err(|e| e.reason) {
                        Err(Runtime(RuntimeError::InvalidType { .. })) => Ok(()),
                        Err(Runtime(RuntimeError::InvalidReturnType { .. })) => Ok(()),
                        Err(e) => Err(self.err_message(suite, case, format!("{}", e))),
                        Ok(r) => Err(self.err_message(suite, case, r.to_string())),
                    },
                    ErrorType::InvalidSlice => match result?.search(given).map_err(|e| e.reason) {
                        Err(Runtime(RuntimeError::InvalidSlice)) => Ok(()),
                        Err(e) => Err(self.err_message(suite, case, format!("{}", e))),
                        Ok(r) => Err(self.err_message(suite, case, r.to_string())),
                    },
                    ErrorType::UnknownFunction => {
                        match result?.search(given).map_err(|e| e.reason) {
                            Err(Runtime(RuntimeError::UnknownFunction(_))) => Ok(()),
                            Err(e) => Err(self.err_message(suite, case, format!("{}", e))),
                            Ok(r) => Err(self.err_message(suite, case, r.to_string())),
                        }
                    }
                    ErrorType::SyntaxError => match result {
                        Err(_) => Ok(()),
                        Ok(expr) => Err(self.err_message(suite, case, format!("Parsed {:?}", expr))),
                    },
                }
            }
        }
    }

    /// Attempts to parse an expression for a case, returning the expression or an error string.
    fn try_parse(&self, suite: &str, case: &TestCase) -> Result<Expression<'_>, String> {
        match compile(&case.expression) {
            Err(e) => Err(self.err_message(suite, case, format!("{}", e))),
            Ok(expr) => Ok(expr),
        }
    }

    /// Formats an error message for a test case failure.
    fn err_message(&self, suite: &str, case: &TestCase, message: String) -> String {
        format!(
            "Test suite: {}\nExpression: {}\nAssertion: {}\nResult: {}\n==============",
            suite, case.expression, self, message
        )
    }
}

impl fmt::Display for Assertion {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Assertion::Error(e) => write!(fmt, "expects error({})", e),
            Assertion::Bench(b) => write!(fmt, "expects bench({})", b),
            Assertion::ValidResult(r) => write!(fmt, "expects result({:?})", r),
        }
    }
}

/// The test suite holds a collection of test cases and has a given value.
#[allow(dead_code)]
pub struct TestSuite<'a> {
    /// Filename of the test suite
    filename: &'a str,
    /// Given data of the test suite
    given: Arcvar,
    /// Collection of test cases to perform
    cases: Vec<TestCase>,
}

impl<'a> TestSuite<'a> {
    /// Creates a test suite from JSON string.
    pub fn from_str(filename: &'a str, suite: &str) -> Result<TestSuite<'a>, String> {
        serde_json::from_str::<Value>(suite)
            .map_err(|e| e.to_string())
            .and_then(|j| TestSuite::from_json(filename, &j))
    }

    /// Creates a test suite from parsed JSON data.
    fn from_json(filename: &'a str, suite: &Value) -> Result<TestSuite<'a>, String> {
        let suite_obj = suite.as_object().ok_or("test suite is not an object")?;
        let test_case = suite_obj.get("cases").ok_or("No cases value".to_string())?;
        let case_array = test_case
            .as_array()
            .ok_or("cases is not an array".to_string())?;
        let mut cases = vec![];
        for case in case_array {
            cases.push(TestCase::from_json(case).map_err(|e| e.to_string())?);
        }
        let value = suite_obj
            .get("given")
            .ok_or("No given value".to_string())?
            .clone();
        // 这里把 Rcvar::new(...) -> Arcvar::new(...)
        let given_var = serde_json::from_value::<Variable>(value)
            .map_err(|e| format!("{}", e))?;
        let given_arcvar = Arcvar::new(given_var);
        Ok(TestSuite {
            filename,
            given: given_arcvar,
            cases,
        })
    }
}

/// Errors that can occur when creating a TestCase
#[derive(Debug)]
pub enum TestCaseError {
    InvalidJSON(String),
    NoCaseType,
    NoResult,
    ResultCannotToString,
    NoExpression,
    ExpressionIsNotString,
    ErrorIsNotString,
    UnknownErrorType(String),
    UnknownBenchType(String),
    BenchIsNotString,
}

impl fmt::Display for TestCaseError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use self::TestCaseError::*;
        match self {
            InvalidJSON(msg) => write!(fmt, "invalid test case JSON: {}", msg),
            NoCaseType => write!(fmt, "case has no result, error, or bench"),
            NoResult => write!(fmt, "test case has no result key"),
            ResultCannotToString => write!(fmt, "result could not be cast to string"),
            NoExpression => write!(fmt, "test case has no expression key"),
            ExpressionIsNotString => write!(fmt, "test case expression is not a string"),
            ErrorIsNotString => write!(fmt, "test case error value is not a string"),
            UnknownErrorType(t) => write!(fmt, "unknown error type: {}", t),
            BenchIsNotString => write!(fmt, "bench value is not a string"),
            UnknownBenchType(bench) => {
                write!(fmt, "unknown bench value: {}, expected parse|full|interpret", bench)
            }
        }
    }
}

/// Represents a test case that contains an expression and assertion.
pub struct TestCase {
    /// The expression being evaluated.
    pub expression: String,
    /// The assertion to perform for the test case.
    pub assertion: Assertion,
}

impl TestCase {
    /// Creates a test case from a JSON encoded string.
    pub fn from_str(case: &str) -> Result<TestCase, TestCaseError> {
        serde_json::from_str::<Value>(case)
            .map_err(|e| TestCaseError::InvalidJSON(e.to_string()))
            .and_then(|json| TestCase::from_json(&json))
    }

    /// Creates a test case from parsed JSON data.
    fn from_json(case: &Value) -> Result<TestCase, TestCaseError> {
        use TestCaseError::*;
        let case_obj = case.as_object().ok_or(InvalidJSON("not an object".to_string()))?;
        let expression_str = case_obj
            .get("expression")
            .ok_or(NoExpression)?
            .as_str()
            .ok_or(ExpressionIsNotString)?
            .to_string();

        let assertion = match case_obj.get("error") {
            Some(err_val) => Assertion::Error(ErrorType::from_json(err_val)?),
            None if case_obj.contains_key("result") => {
                let value = case_obj.get("result").unwrap().clone();
                let var = serde_json::from_value::<Variable>(value).map_err(|e| InvalidJSON(e.to_string()))?;
                // Arcvar
                Assertion::ValidResult(Arcvar::new(var))
            }
            None if case_obj.contains_key("bench") => {
                Assertion::Bench(BenchType::from_json(case_obj.get("bench").unwrap())?)
            }
            _ => return Err(NoCaseType),
        };

        Ok(TestCase {
            expression: expression_str,
            assertion,
        })
        
    }

    /// Perform the test case assertion against a given value.
    pub fn assert(&self, suite_filename: &str, given: Arcvar) -> Result<(), String> {
        self.assertion.assert(suite_filename, self, given)
    }
}

// 这里包含通过 build.rs 或其他脚本生成的测试文件
include!(concat!(env!("OUT_DIR"), "/compliance_tests.rs"));