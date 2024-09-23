use crate::error::Error;
use crate::script::value::Value;
use crate::script::ScriptContext;
use regex::Regex;

pub enum ScriptVariable {
    Constant(Value),
    Variable(String),
    VariableMap(String, String), // (map_name, key)
    VariableList(String, i32),   // (list_name, index)
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
            let re = Regex::new(r"(\w+)\[\'([\w-]+)\'\]").unwrap();
            if let Some(captures) = re.captures(str) {
                if captures.len() == 3 {
                    let map_name = captures.get(1).unwrap().as_str();
                    let key = captures.get(2).unwrap().as_str();
                    return ScriptVariable::VariableMap(map_name.into(), key.into());
                }
            }

            // Check if it's a list using regex. ie. numbers[1]
            let re = Regex::new(r"(\w+)\[(\d+)\]").unwrap();
            if let Some(captures) = re.captures(str) {
                if captures.len() == 3 {
                    let list_name = captures.get(1).unwrap().as_str();
                    let index = captures.get(2).unwrap().as_str();
                    if let Ok(index) = index.parse::<i32>() {
                        return ScriptVariable::VariableList(list_name.into(), index);
                    }
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
            ScriptVariable::Constant(v) => Ok(v.clone()),
            ScriptVariable::Variable(name) => ctx.must_get_variable(name),
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
            ScriptVariable::VariableList(list_name, index) => {
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
        map.insert("content-type".into(), "applicaiton/json".into());
        ctx.set_variable("responseHeaders", Value::Map(map));

        let a = ScriptVariable::from_str("responseHeaders['content-type']");
        let a = a.get_value(&ctx).unwrap();
        assert_eq!(a, Value::String("applicaiton/json".into()));
    }

    #[test]
    fn test_get_values_variable_map_not_found() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        let mut map = HashMap::new();
        map.insert("content-type".into(), "applicaiton/json".into());
        ctx.set_variable("responseHeaders", Value::Map(map));

        let v = ScriptVariable::from_str("responseHeaders['content-length']");
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

        let v = ScriptVariable::from_str("numbers[1]");
        let v = v.get_value(&ctx).unwrap();
        assert_eq!(v, Value::Int(2));
    }

    #[test]
    fn test_get_values_variable_list_out_of_range() {
        let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
        let list = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        ctx.set_variable("numbers", Value::List(list));

        let v = ScriptVariable::from_str("numbers[3]");
        let v = v.get_value(&ctx);
        assert!(v.is_err());
        assert_eq!(
            v.unwrap_err().to_string(),
            "Script error: Index '3' out of range in list 'numbers'"
        );
    }

    // TODO
    // #[test]
    // fn test_get_values_variable_headers() {
    //     let mut ctx = ScriptContext::new(Arc::new(RwLock::new(Global::empty())));
    //
    //     let mut list = Vec::new();
    //     list.push(Value::String("application/json".into()));
    //
    //     let mut map = HashMap::new();
    //     map.insert("content-type".into(), Value::List(list));
    //
    //     ctx.set_variable("responseHeaders", Value::Map(map));
    //
    //     let v = ScriptVariable::from_str("responseHeaders['content-type']");
    //     let v = v.get_value(&ctx).unwrap();
    //
    //     let v = v.as_map().unwrap();
    // }
}
