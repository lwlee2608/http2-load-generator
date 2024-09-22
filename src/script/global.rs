use crate::config;
use crate::script::Value;
use std::collections::HashMap;

pub struct Global {
    pub variables: HashMap<String, Value>,
}

impl Global {
    pub fn new(_configs: config::Global) -> Self {
        Global {
            variables: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Global {
            variables: HashMap::new(),
        }
    }

    pub fn get_variable_value(&self, variable_name: &str) -> Option<&Value> {
        self.variables.get(variable_name)
        // .map(|v| v.clone())
    }

    pub fn update_variable_value(&mut self, variable_name: &str, value: Value) {
        if self.variables.contains_key(variable_name) {
            self.variables.insert(variable_name.into(), value);
        }
    }

    pub fn insert_variable(&mut self, variable_name: &str, value: Value) {
        self.variables.insert(variable_name.into(), value);
    }
}
