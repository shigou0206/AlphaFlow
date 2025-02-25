// src/jmes_runtime.rs
use alphaflow_jmes::{
    functions::{ArgumentType, CustomFunction, Signature},
    Arcvar, Expression, Runtime, Variable,
};
use once_cell::sync::Lazy;
use serde_json::Value;
use log::info;

pub static CUSTOM_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    let mut rt = Runtime::new();
    rt.register_builtin_functions();

    // 注册自定义函数 uppercase(s)
    rt.register_function(
        "uppercase",
        Box::new(CustomFunction::new(
            Signature::new(vec![ArgumentType::String], None),
            Box::new(|args: &[Arcvar], _ctx| {
                let s = args[0].as_string().cloned().unwrap_or_default();
                Ok(Arcvar::new(Variable::String(s.to_ascii_uppercase())))
            }),
        )),
    );

    // 注册自定义函数 split(delim, input)
    rt.register_function(
        "split",
        Box::new(CustomFunction::new(
            Signature::new(vec![ArgumentType::String, ArgumentType::String], None),
            Box::new(|args: &[Arcvar], _ctx| {
                let delim = args[0].as_string().cloned().unwrap_or_default();
                let input = args[1].as_string().cloned().unwrap_or_default();
                let splitted: Vec<Arcvar> = input
                    .split(&delim)
                    .map(|part| Arcvar::new(Variable::String(part.to_string())))
                    .collect();
                Ok(Arcvar::new(Variable::Array(splitted)))
            }),
        )),
    );

    info!("JMES runtime initialized with builtins + custom functions.");
    rt
});

#[derive(Debug, thiserror::Error)]
pub enum JmesMappingError {
    #[error("Compile error: {0}")]
    CompileError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub fn compile_and_search(expr_str: &str, input_data: &Value) -> Result<Value, JmesMappingError> {
    let expr = CUSTOM_RUNTIME
        .compile(expr_str)
        .map_err(|e| JmesMappingError::CompileError(e.to_string()))?;
    let result_var = expr
        .search(input_data)
        .map_err(|e| JmesMappingError::ExecutionError(e.to_string()))?;
    serde_json::to_value(&result_var)
        .map_err(|e| JmesMappingError::SerializationError(e.to_string()))
}