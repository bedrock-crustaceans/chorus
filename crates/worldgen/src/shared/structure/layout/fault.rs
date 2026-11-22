use serde_json::Value;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    Missing,
    WrongType(&'static str),
    Unknown(String),
    Undeclared(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub field: &'static str,
    pub problem: Problem,
}

impl Fault {
    pub fn unknown(field: &'static str, value: impl Into<String>) -> Self {
        Self {
            field,
            problem: Problem::Unknown(value.into()),
        }
    }

    pub fn wrong_type(field: &'static str, expected: &'static str) -> Self {
        Self {
            field,
            problem: Problem::WrongType(expected),
        }
    }

    pub fn undeclared(field: &'static str, value: impl Into<String>) -> Self {
        Self {
            field,
            problem: Problem::Undeclared(value.into()),
        }
    }

    pub fn in_document(self, document: impl Into<String>) -> PoolError {
        PoolError {
            document: document.into(),
            fault: self,
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let field = self.field;
        match &self.problem {
            Problem::Missing => write!(formatter, "field `{field}` is missing"),
            Problem::WrongType(expected) => write!(formatter, "field `{field}` should be {expected}"),
            Problem::Unknown(value) => write!(formatter, "field `{field}` has the unknown value `{value}`"),
            Problem::Undeclared(value) => write!(formatter, "field `{field}` refers to `{value}`, which no document declares"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolError {
    pub document: String,
    pub fault: Fault,
}

impl fmt::Display for PoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.document, self.fault)
    }
}

impl Error for PoolError {}

pub fn member<'a>(value: &'a Value, field: &'static str) -> Result<&'a Value, Fault> {
    value.get(field).ok_or(Fault { field, problem: Problem::Missing })
}

fn typed<'a, T>(value: &'a Value, field: &'static str, expected: &'static str, read: impl FnOnce(&'a Value) -> Option<T>) -> Result<T, Fault> {
    read(member(value, field)?).ok_or(Fault::wrong_type(field, expected))
}

pub fn text<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, Fault> {
    typed(value, field, "a string", Value::as_str)
}

pub fn items<'a>(value: &'a Value, field: &'static str) -> Result<&'a [Value], Fault> {
    typed(value, field, "a list", |member| member.as_array().map(Vec::as_slice))
}

pub fn unsigned(value: &Value, field: &'static str) -> Result<u64, Fault> {
    typed(value, field, "a non-negative integer", Value::as_u64)
}

pub fn signed(value: &Value, field: &'static str) -> Result<i64, Fault> {
    typed(value, field, "an integer", Value::as_i64)
}

pub fn float(value: &Value, field: &'static str) -> Result<f64, Fault> {
    typed(value, field, "a number", Value::as_f64)
}
