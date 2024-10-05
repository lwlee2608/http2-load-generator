use crate::error::Error;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(String),
    Int(i32),
    //Float(f64),
    Map(HashMap<String, Value>),
    List(Vec<Value>),
    Null,
}

impl PartialEq<&Value> for Vec<Value> {
    fn eq(&self, other: &&Value) -> bool {
        if let Value::List(ref v) = other {
            return self == v;
        }
        false
    }
}

impl Value {
    pub fn as_string(&self) -> Result<String, Error> {
        match self {
            Value::String(ref v) => Ok(v.clone()),
            Value::Int(v) => Ok(v.to_string()),
            Value::Map(_) => Err(Error::ScriptError(
                "Map cannot be converted to String".into(),
            )),
            Value::List(_) => Err(Error::ScriptError(
                "List cannot be converted to String".into(),
            )),
            Value::Null => Ok("".to_string()),
        }
    }

    pub fn as_int(&self) -> Result<i32, Error> {
        match self {
            Value::String(ref v) => {
                if let Ok(v) = v.parse::<i32>() {
                    return Ok(v);
                }
                return Err(Error::ScriptError(format!(
                    "String '{}' cannot be converted to Int",
                    v
                )));
            }
            Value::Int(v) => Ok(*v),
            Value::Map(_) => Err(Error::ScriptError("Map cannot be converted to Int".into())),
            Value::List(_) => Err(Error::ScriptError("List cannot be converted to Int".into())),
            Value::Null => Ok(0),
        }
    }

    pub fn as_map(&self) -> Result<HashMap<String, Value>, Error> {
        match self {
            Value::String(v) => Err(Error::ScriptError(format!(
                "String '{}' cannot be converted to Map",
                v
            ))),
            Value::Int(v) => Err(Error::ScriptError(format!(
                "Int '{}' cannot be converted to Map",
                v
            ))),
            Value::Map(ref v) => Ok(v.clone()),
            Value::List(_) => Err(Error::ScriptError("List cannot be converted to Map".into())),
            Value::Null => Ok(HashMap::new()),
        }
    }

    pub fn as_list(&self) -> Result<Vec<Value>, Error> {
        match self {
            Value::String(v) => Err(Error::ScriptError(format!(
                "String '{}' cannot be converted to List",
                v
            ))),
            Value::Int(v) => Err(Error::ScriptError(format!(
                "Int '{}' cannot be converted to List",
                v
            ))),
            Value::Map(_) => Err(Error::ScriptError("Map cannot be converted to List".into())),
            Value::List(ref v) => Ok(v.clone()),
            Value::Null => Ok(Vec::new()),
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

impl From<Vec<Value>> for Value {
    fn from(list: Vec<Value>) -> Self {
        Value::List(list)
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::String(ref v) => write!(f, "{}", v),
            Value::Int(v) => write!(f, "{}", v),
            Value::Map(ref v) => write!(f, "{:?}", v),
            Value::List(ref v) => write!(f, "{:?}", v),
            Value::Null => write!(f, "null"),
        }
    }
}
