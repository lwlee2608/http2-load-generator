pub mod assert;
pub mod define;
pub mod parser;

pub use crate::script::parser::Scripts;

use crate::error::Error;
use crate::scenario::Global;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(String),
    Int(i32),
    //Float(f64),
    Map(HashMap<String, Value>),
    // List(Vec<Value>),
}

impl Value {
    pub fn as_string(&self) -> Result<String, Error> {
        match self {
            Value::String(ref v) => Ok(v.clone()),
            Value::Int(v) => Ok(v.to_string()),
            Value::Map(_) => Err(Error::ScriptError(
                "Map cannot be converted to String".into(),
            )),
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

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::String(ref v) => write!(f, "{}", v),
            Value::Int(v) => write!(f, "{}", v),
            Value::Map(ref v) => write!(f, "{:?}", v),
        }
    }
}

pub struct Local {
    pub variables: HashMap<String, Value>,
}

pub struct ScriptContext {
    pub local: Local,
    pub global: Arc<RwLock<Global>>,
}

impl ScriptContext {
    pub fn new(global: Arc<RwLock<Global>>) -> Self {
        let local = Local {
            variables: HashMap::new(),
        };
        ScriptContext { local, global }
    }

    pub fn get_variable(&self, name: &str) -> Option<Value> {
        let value = self.local.variables.get(name);
        // Get from local first
        if let Some(value) = value {
            return Some(value.clone());
        }

        // Then check global
        let global = self.global.read().unwrap();
        let value = global.get_variable_value(name);
        if let Some(value) = value {
            return Some(value.clone());
        }
        None
    }

    pub fn must_get_variable(&self, name: &str) -> Result<Value, Error> {
        let value = self.get_variable(name);
        if let Some(value) = value {
            return Ok(value);
        }
        Err(Error::ScriptError(format!("Variable '{}' not found", name)))
    }

    pub fn set_variable(&mut self, name: &str, value: Value) {
        // Set to local
        self.set_local_variable(name, value.clone());

        // Set to global
        let mut global = self.global.write().unwrap();
        global.update_variable_value(name, value);
    }

    pub fn set_local_variable(&mut self, name: &str, value: Value) {
        self.local.variables.insert(name.into(), value);
    }

    // used in global init
    pub fn save_variables_as_global(&self) {
        let mut global = self.global.write().unwrap();
        for (name, value) in &self.local.variables {
            global.insert_variable(name, value.clone());
        }
    }
}

pub enum ScriptVariable {
    VariableMap(String, String), // (map_name, key)
    Variable(String),
    Constant(Value),
}

impl ScriptVariable {
    pub fn from_str(str: &str) -> ScriptVariable {
        if str.starts_with("'") && str.ends_with("'") {
            // String constant
            let v = &str[1..str.len() - 1];
            let v = Value::String(v.to_string());
            ScriptVariable::Constant(v)
        } else {
            // Check if it's a map using regex. ie. responseHeaders['contentType']
            let re = Regex::new(r"(\w+)\[\'(\w+)\'\]").unwrap();
            if let Some(captures) = re.captures(str) {
                if captures.len() == 3 {
                    let map_name = captures.get(1).unwrap().as_str();
                    let key = captures.get(2).unwrap().as_str();
                    return ScriptVariable::VariableMap(map_name.into(), key.into());
                }
            }

            if let Ok(v) = str.parse::<i32>() {
                // Integer constant
                let v = Value::Int(v);
                ScriptVariable::Constant(v)
            } else {
                // Variable
                let var_name = str;
                ScriptVariable::Variable(var_name.into())
            }
        }
    }

    pub fn get_value(&self, ctx: &ScriptContext) -> Result<Value, Error> {
        match self {
            ScriptVariable::VariableMap(map_name, key) => {
                let map = ctx.must_get_variable(map_name)?;
                let map = map.as_map()?;
                let value = map.get(key);
                if let Some(value) = value {
                    return Ok(value.clone());
                }
                return Err(Error::ScriptError(format!(
                    "Key '{}' not found in map '{}'",
                    key, map_name
                )));
            }
            ScriptVariable::Variable(name) => ctx.must_get_variable(name),
            ScriptVariable::Constant(v) => Ok(v.clone()),
        }
    }
}

pub trait Script {
    fn execute(&self, ctx: &mut ScriptContext) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_values_constant() {
        let ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));

        let a = ScriptVariable::from_str("'hello'");
        let a = a.get_value(&ctx).unwrap();
        assert_eq!(a, Value::String("hello".into()));

        let b = ScriptVariable::from_str("123");
        let b = b.get_value(&ctx).unwrap();
        assert_eq!(b, Value::Int(123));
    }

    #[test]
    fn test_get_values_variable() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        ctx.set_variable("a", Value::Int(1));
        ctx.set_variable("b", Value::String("hello".into()));

        let a = ScriptVariable::from_str("a");
        let a = a.get_value(&ctx).unwrap();
        assert_eq!(a, Value::Int(1));

        let b = ScriptVariable::from_str("b");
        let b = b.get_value(&ctx).unwrap();
        assert_eq!(b, Value::String("hello".into()));

        let c = ScriptVariable::from_str("c");
        let c = c.get_value(&ctx);
        assert!(c.is_err());
    }

    #[test]
    fn test_get_values_variable_map() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        let mut map = HashMap::new();
        map.insert("contentType".into(), "applicaiton/json".into());
        ctx.set_variable("responseHeaders", Value::Map(map));

        let a = ScriptVariable::from_str("responseHeaders['contentType']");
        let a = a.get_value(&ctx).unwrap();
        assert_eq!(a, Value::String("applicaiton/json".into()));
    }
}
