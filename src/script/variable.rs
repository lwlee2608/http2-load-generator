use crate::error::Error;
use crate::script::value::Value;
use crate::script::ScriptContext;

pub enum Variable {
    Constant(Value),                                  // constant_value
    Variable(String),                                 // variable_name
    VariableMap(String, String),                      // (variable_name, map_key)
    VariableList(String, i32),                        // (variable_name, index)
    NestedVariables(String, Vec<NestedVariableType>), // (variable_name, keys)
}

#[derive(Debug)]
pub enum NestedVariableType {
    Map(String),
    List(i32),
}

impl Variable {
    // TODO This should be parser module?
    pub fn parse_square_brackets(s: &str) -> (String, Vec<String>) {
        // Example: responseHeaders['content-type'][0]
        // variable_name = responseHeaders
        // keys = ['content-type', '0']
        //
        let mut keys = Vec::new();
        let mut current_key = String::new();
        let mut variable_name = String::new();
        let mut reading_variable = true;

        for ch in s.chars() {
            if ch == '[' {
                reading_variable = false;
                continue;
            } else if ch == ']' {
                // When closing a bracket, push the collected key
                keys.push(current_key.clone());
                // keys.push(current_key.trim_matches('\'').to_string());
                current_key.clear();
                continue;
            }

            if reading_variable {
                variable_name.push(ch);
            } else {
                current_key.push(ch);
            }
        }

        (variable_name, keys)
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Variable {
        let (str, keys) = Variable::parse_square_brackets(s);

        if keys.is_empty() {
            if str.starts_with("'") && str.ends_with("'") {
                // String constant
                let v = &str[1..str.len() - 1];
                let v = Value::String(v.to_string());
                Variable::Constant(v)
            } else if let Ok(v) = str.parse::<i32>() {
                // Integer constant
                let v = Value::Int(v);
                Variable::Constant(v)
            } else {
                // Variable
                let var_name = str;
                Variable::Variable(var_name)
            }
        } else {
            // Square bracket exist, but be a map or list
            // Just use first key for now
            // let key = &keys[0];
            //
            if keys.len() > 1 {
                // Nested Variable
                let mut nested_var_keys = Vec::new();
                for key in keys.iter() {
                    let k = if key.starts_with("'") && key.ends_with("'") {
                        // Key is String constant
                        let k = &key[1..key.len() - 1];
                        NestedVariableType::Map(k.to_string())
                    } else if let Ok(v) = key.parse::<i32>() {
                        NestedVariableType::List(v)
                    } else {
                        // Not tested
                        NestedVariableType::Map(key.into())
                    };
                    nested_var_keys.push(k);
                }
                Variable::NestedVariables(str.into(), nested_var_keys)
            } else {
                // Just use first key for now
                let key = &keys[0];
                if key.starts_with("'") && key.ends_with("'") {
                    // Key is String constant
                    let v = &key[1..key.len() - 1];
                    let v = Value::String(v.to_string());
                    Variable::VariableMap(str.into(), v.to_string())
                } else if let Ok(v) = key.parse::<i32>() {
                    // Key is Integer constant
                    Variable::VariableList(str.into(), v)
                } else {
                    // Key is Variable
                    Variable::VariableMap(str.into(), key.into())
                }
            }
        }
    }

    pub fn get_value(&self, ctx: &ScriptContext) -> Result<Value, Error> {
        match self {
            Variable::Constant(v) => Ok(v.clone()),
            Variable::Variable(name) => ctx.must_get_variable(name),
            Variable::VariableMap(map_name, key) => {
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
            Variable::VariableList(list_name, index) => {
                let list = ctx.must_get_variable(list_name)?;
                let list = list.as_list()?;
                let index = *index as usize;
                if index >= list.len() {
                    return Err(Error::ScriptError(format!(
                        "Index '{}' out of range in list '{}'",
                        index, list_name
                    )));
                }
                return Ok(list[index].clone());
            }
            Variable::NestedVariables(var_name, keys) => {
                // Get first variable
                let mut var = ctx.must_get_variable(var_name)?;
                // Traverse the keys
                for key in keys.iter() {
                    match key {
                        NestedVariableType::Map(k) => {
                            let map = var.as_map()?;
                            let value = map.get(k).unwrap();
                            var = value.clone();
                        }
                        NestedVariableType::List(i) => {
                            let list = var.as_list()?;
                            let index = *i as usize;
                            if index >= list.len() {
                                return Err(Error::ScriptError(format!(
                                    "Index '{}' out of range in list '{}'",
                                    index, var_name
                                )));
                            }
                            var = list[index].clone();
                        }
                    }
                }
                return Ok(var);
            }
        }
    }
}

impl std::fmt::Debug for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Variable::Constant(v) => write!(f, "Constant({:?})", v),
            Variable::Variable(name) => write!(f, "Variable({})", name),
            Variable::VariableMap(name, key) => write!(f, "VariableMap({}, {})", name, key),
            Variable::VariableList(name, index) => write!(f, "VariableList({}, {})", name, index),
            Variable::NestedVariables(name, keys) => {
                write!(f, "NestedVariables({}, {:?})", name, keys)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::global::Global;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_get_values_constant() {
        let ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));

        let a = Variable::from_str("'hello'");
        let a = a.get_value(&ctx).unwrap();
        assert_eq!(a, Value::String("hello".into()));

        let b = Variable::from_str("123");
        let b = b.get_value(&ctx).unwrap();
        assert_eq!(b, Value::Int(123));
    }

    #[test]
    fn test_get_values_variable() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        ctx.set_variable("a", Value::Int(1));
        ctx.set_variable("b", Value::String("hello".into()));

        let a = Variable::from_str("a");
        let a = a.get_value(&ctx).unwrap();
        assert_eq!(a, Value::Int(1));

        let b = Variable::from_str("b");
        let b = b.get_value(&ctx).unwrap();
        assert_eq!(b, Value::String("hello".into()));

        let c = Variable::from_str("c");
        let c = c.get_value(&ctx);
        assert!(c.is_err());
    }

    #[test]
    fn test_get_values_variable_map() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        let mut map = HashMap::new();
        map.insert("content-type".into(), "applicaiton/json".into());
        ctx.set_variable("responseHeaders", Value::Map(map));

        let a = Variable::from_str("responseHeaders['content-type']");
        let a = a.get_value(&ctx).unwrap();
        assert_eq!(a, Value::String("applicaiton/json".into()));
    }

    #[test]
    fn test_get_values_variable_map_not_found() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        let mut map = HashMap::new();
        map.insert("content-type".into(), "applicaiton/json".into());
        ctx.set_variable("responseHeaders", Value::Map(map));

        let v = Variable::from_str("responseHeaders['content-length']");
        let v = v.get_value(&ctx);
        assert!(v.is_err());
        assert_eq!(
            v.unwrap_err().to_string(),
            "Script error: Key 'content-length' not found in map 'responseHeaders'"
        );
    }

    #[test]
    fn test_get_values_variable_list() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        let list = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        ctx.set_variable("numbers", Value::List(list));

        let v = Variable::from_str("numbers[1]");
        let v = v.get_value(&ctx).unwrap();
        assert_eq!(v, Value::Int(2));
    }

    #[test]
    fn test_get_values_variable_list_out_of_range() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        let list = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        ctx.set_variable("numbers", Value::List(list));

        let v = Variable::from_str("numbers[3]");
        let v = v.get_value(&ctx);
        assert!(v.is_err());
        assert_eq!(
            v.unwrap_err().to_string(),
            "Script error: Index '3' out of range in list 'numbers'"
        );
    }

    #[test]
    fn test_get_values_variable_headers() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));

        let mut list = Vec::new();
        list.push(Value::String("application/json".into()));
        list.push(Value::String("application/xml".into()));

        let mut map = HashMap::new();
        map.insert("content-type".into(), Value::List(list));

        ctx.set_variable("responseHeaders", Value::Map(map));

        let v = Variable::from_str("responseHeaders['content-type'][0]");
        let v = v.get_value(&ctx).unwrap();
        let v = v.as_string().unwrap();
        assert_eq!(v, "application/json");

        let v = Variable::from_str("responseHeaders['content-type'][1]");
        let v = v.get_value(&ctx).unwrap();
        let v = v.as_string().unwrap();
        assert_eq!(v, "application/xml");
    }
}
