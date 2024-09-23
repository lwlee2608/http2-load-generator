use crate::error::Error;
use crate::script::Global;
use crate::script::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

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
