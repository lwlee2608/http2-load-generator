use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Int(i32),
    // TODO Support float
}

impl Value {
    pub fn as_string(&self) -> String {
        match self {
            Value::String(ref v) => v.clone(),
            Value::Int(v) => v.to_string(),
        }
    }

    pub fn as_int(&self) -> i32 {
        match self {
            Value::String(ref v) => v.parse::<i32>().unwrap(),
            Value::Int(v) => *v,
        }
    }
}

impl From<&str> for Value {
    fn from(str: &str) -> Self {
        Value::String(str.to_string())
    }
}

impl From<i32> for Value {
    fn from(int: i32) -> Self {
        Value::Int(int)
    }
}
