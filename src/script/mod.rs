pub mod assert;
pub mod define;
pub mod parser;

pub use crate::script::parser::Scripts;

use crate::error::Error;
use crate::scenario::Global;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, PartialEq, Clone)]
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

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::String(ref v) => write!(f, "{}", v),
            Value::Int(v) => write!(f, "{}", v),
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
    Variable(String),
    Constant(Value),
}

impl ScriptVariable {
    // Could be constant integer, string, or variable
    pub fn from_str(str: &str) -> ScriptVariable {
        if str.starts_with("'") && str.ends_with("'") {
            // String constant
            let v = &str[1..str.len() - 1];
            let v = Value::String(v.to_string());
            ScriptVariable::Constant(v)
        } else {
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
            ScriptVariable::Variable(name) => ctx.must_get_variable(name),
            ScriptVariable::Constant(v) => Ok(v.clone()),
        }
    }
}

pub trait Script {
    fn execute(&self, ctx: &mut ScriptContext) -> Result<(), Error>;
}
