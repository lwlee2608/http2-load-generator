use crate::error::Error;
use crate::function;
use crate::function::FunctionApply;
use crate::scenario::Global;
use crate::variable::Value;
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
        self.local.variables.insert(name.into(), value.clone());

        // Set to global
        let mut global = self.global.write().unwrap();
        global.update_variable_value(name, value);
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

pub struct Script {
    pub return_var_name: String,
    pub function: function::Function,
    pub args: Vec<ScriptVariable>,
}

impl Script {
    pub fn execute(&self, ctx: &mut ScriptContext) -> Result<(), Error> {
        let value = match &self.function {
            function::Function::Plus(f) => {
                if self.args.len() == 2 {
                    let arg0 = self.args[0].get_value(ctx)?.as_int();
                    let arg1 = self.args[1].get_value(ctx)?.as_int();
                    let value = f.apply(arg0, arg1);
                    Value::Int(value)
                } else {
                    return Err(Error::ScriptError("Expects 2 arguments".into()));
                }
            }
            function::Function::Now(f) => {
                if self.args.len() == 1 {
                    let arg0 = self.args[0].get_value(ctx)?;
                    let arg0 = arg0.as_string();
                    let value = f.apply(Some(arg0));
                    Value::String(value)
                } else if self.args.len() == 0 {
                    let value = f.apply(None);
                    Value::String(value)
                } else {
                    return Err(Error::ScriptError("Expects 0 or 1 argument".into()));
                }
            }
            function::Function::Random(f) => {
                if self.args.len() == 0 {
                    let value = f.apply();
                    Value::Int(value)
                } else {
                    return Err(Error::ScriptError("Expects 0 arguments".into()));
                }
            }
            function::Function::Split(f) => {
                if self.args.len() == 1 {
                    let arg0 = self.args[0].get_value(ctx)?;
                    let arg0 = arg0.as_string();
                    let value = f.apply(arg0);
                    Value::String(value)
                } else {
                    return Err(Error::ScriptError("Expects 1 argument".into()));
                }
            }
            function::Function::Copy(f) => {
                let args = self
                    .args
                    .iter()
                    .map(|arg| arg.get_value(ctx))
                    .collect::<Result<Vec<Value>, Error>>()?;
                f.apply(args)?
            }
            function::Function::SubString(f) => {
                let args = self
                    .args
                    .iter()
                    .map(|arg| arg.get_value(ctx))
                    .collect::<Result<Vec<Value>, Error>>()?;
                f.apply(args)?
            }
            function::Function::LastIndexOf(f) => {
                let args = self
                    .args
                    .iter()
                    .map(|arg| arg.get_value(ctx))
                    .collect::<Result<Vec<Value>, Error>>()?;
                f.apply(args)?
            }
        };

        // Set the return value to the context
        ctx.set_variable(self.return_var_name.as_str(), value);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // let now = Now("%Y-%m-%d")
    #[test]
    fn test_script_now() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "now".to_string(),
            function: function::Function::Now(function::NowFunction {}),
            args: vec![ScriptVariable::Constant(Value::String(
                "%Y-%m-%d".to_string(),
            ))],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("now").unwrap();
        let value = result.as_string();

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(value.len() > 0);
        assert!(value.starts_with(&today));
    }

    // let random = Random(1, 10)
    // let value = random.run()
    #[test]
    fn test_script_random() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "value".to_string(),
            function: function::Function::Random(function::RandomFunction { min: 1, max: 10 }),
            args: vec![],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("value").unwrap();
        let value = result.as_int();
        assert!(value >= 1 && value <= 10);
    }

    // let var1 = var2
    #[test]
    fn test_script_copy() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        // let script = Script::new(config::ScriptVariable {
        //     name: "var1".to_string(),
        //     function: function::Function::Copy(function::CopyFunction {}),
        //     args: Some(vec![Value::String("$var2".to_string())]),
        // });
        let script = Script {
            return_var_name: "var1".to_string(),
            function: function::Function::Copy(function::CopyFunction {}),
            args: vec![ScriptVariable::Variable("var2".into())],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        ctx.set_variable("var2", Value::Int(123456789));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("var1").unwrap();
        assert_eq!(result.as_int(), 123456789);
    }

    // let split = Split(":", 1)
    // let chargingDataRef = split.run("123:456")
    #[test]
    fn test_script_split() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "chargingDataRef".to_string(),
            function: function::Function::Split(function::SplitFunction {
                delimiter: ":".to_string(),
                index: function::SplitIndex::Nth(1),
            }),
            args: vec![ScriptVariable::Constant(Value::String(
                "123:456".to_string(),
            ))],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("chargingDataRef").unwrap();
        assert_eq!(result.as_string(), "456");
    }

    #[test]
    fn test_script_substring() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "world".to_string(),
            function: function::Function::SubString(function::SubStringFunction {}),
            args: vec![
                ScriptVariable::Constant(Value::String("Hello World".to_string())),
                ScriptVariable::Constant(Value::Int(6)),
            ],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("world").unwrap();
        assert_eq!(result.as_string(), "World");
    }

    // def chargingDataRef = location.substring(location.lastIndexOf('/') + 1)
    #[test]
    fn test_script_extract_location_header() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        let location = Value::String("http://location:8080/test/v1/foo/123456".to_string());

        // def index = location.lastIndexOf('/')
        let script = Script {
            return_var_name: "location".to_string(),
            function: function::Function::LastIndexOf(function::LastIndexOfFunction {}),
            args: vec![
                ScriptVariable::Constant(location.clone()),
                ScriptVariable::Constant(Value::String("/".to_string())),
            ],
        };
        script.execute(&mut ctx).unwrap();

        let index = ctx.get_variable("location").unwrap().as_int();
        assert_eq!(index, 32);

        // def chargingDataRef = location.substring(index + 1)
        let script = Script {
            return_var_name: "chargingDataRef".to_string(),
            function: function::Function::SubString(function::SubStringFunction {}),
            args: vec![
                ScriptVariable::Constant(location),
                ScriptVariable::Constant(Value::Int(index + 1)),
            ],
        };
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("chargingDataRef").unwrap();
        assert_eq!(result.as_string(), "123456");
    }

    // let imsi = 1 + 2
    #[test]
    fn test_script_plus_constant() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "imsi".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: vec![
                ScriptVariable::Constant(Value::Int(1)),
                ScriptVariable::Constant(Value::Int(2)),
            ],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let imsi = ctx.get_variable("imsi").unwrap();
        assert_eq!(imsi.as_int(), 3);
    }

    // local var2 = 22
    // local var3 = var2 + 1
    #[test]
    fn test_script_plus_constant_and_var() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "var3".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: vec![
                ScriptVariable::Variable("var2".into()),
                ScriptVariable::Constant(Value::Int(1)),
            ],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        ctx.set_variable("var2", Value::Int(22));
        script.execute(&mut ctx).unwrap();

        let var3 = ctx.get_variable("var3").unwrap();
        assert_eq!(var3.as_int(), 23);
    }

    // global VAR1 = 11
    // local var2 = 22
    // local var3 = VAR1 + var2
    #[test]
    fn test_script_plus_global_var() {
        // Global
        let global = Global {
            variables: {
                let mut map = HashMap::new();
                map.insert("VAR1".to_string(), Value::Int(11));
                map
            },
        };
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "var3".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: vec![
                ScriptVariable::Variable("VAR1".into()),
                ScriptVariable::Variable("var2".into()),
            ],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        ctx.set_variable("var2", Value::Int(22));
        script.execute(&mut ctx).unwrap();

        let var3 = ctx.get_variable("var3").unwrap();
        assert_eq!(var3.as_int(), 33);
    }

    // VAR1 = 100
    // VAR1 = VAR1 + 11
    #[test]
    fn test_script_update_global_var() {
        // Global
        let global = Global {
            variables: {
                let mut map = HashMap::new();
                map.insert("VAR1".to_string(), Value::Int(100));
                map
            },
        };
        let global = Arc::new(RwLock::new(global));

        let script = Script {
            return_var_name: "VAR1".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: vec![
                ScriptVariable::Variable("VAR1".into()),
                ScriptVariable::Constant(Value::Int(11)),
            ],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let var1 = ctx.get_variable("VAR1").unwrap();
        assert_eq!(var1.as_int(), 111);

        // Check global
        let global = global.read().unwrap();
        let var1 = global.get_variable_value("VAR1").unwrap();
        assert_eq!(var1.as_int(), 111);
    }
}
