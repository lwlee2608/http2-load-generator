use crate::config;
use crate::error::Error;
use crate::function;
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
}

pub enum ScriptVariable {
    Variable(String),
    Constant(Value),
}

impl ScriptVariable {
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
    pub fn new(config: config::ScriptVariable) -> Self {
        let mut args = vec![];
        if let Some(config_args) = config.args {
            for arg in config_args {
                if arg.is_string() {
                    let str_arg = arg.as_string();
                    if str_arg.starts_with("$") {
                        let var_name = &str_arg[1..];
                        args.push(ScriptVariable::Variable(var_name.into()));
                        continue;
                    }
                }
                let arg = ScriptVariable::Constant(arg);
                args.push(arg);
            }
        }
        Script {
            return_var_name: config.name,
            function: config.function,
            args,
        }
    }

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
                if self.args.len() == 1 {
                    let arg0 = self.args[0].get_value(ctx)?;
                    let value = f.apply(&arg0);
                    value
                } else {
                    return Err(Error::ScriptError("Expects 1 argument".into()));
                }
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

        let script = Script::new(config::ScriptVariable {
            name: "now".to_string(),
            function: function::Function::Now(function::NowFunction {}),
            args: Some(vec![Value::String("%Y-%m-%d".to_string())]),
        });

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

        let script = Script::new(config::ScriptVariable {
            name: "value".to_string(),
            function: function::Function::Random(function::RandomFunction { min: 1, max: 10 }),
            args: Some(vec![]),
        });
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
        let global = Global {
            variables: HashMap::new(),
        };
        let global = Arc::new(RwLock::new(global));

        let script = Script::new(config::ScriptVariable {
            name: "var1".to_string(),
            function: function::Function::Copy(function::CopyFunction {}),
            args: Some(vec![Value::String("$var2".to_string())]),
        });
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

        let script = Script::new(config::ScriptVariable {
            name: "chargingDataRef".to_string(),
            function: function::Function::Split(function::SplitFunction {
                delimiter: ":".to_string(),
                index: function::SplitIndex::Nth(1),
            }),
            args: Some(vec![Value::String("123:456".to_string())]),
        });
        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("chargingDataRef").unwrap();
        assert_eq!(result.as_string(), "456");
    }

    // let imsi = 1 + 2
    #[test]
    fn test_script_plus_constant() {
        // Global
        let global = Global {
            variables: HashMap::new(),
        };
        let global = Arc::new(RwLock::new(global));

        let script = Script::new(config::ScriptVariable {
            name: "imsi".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: Some(vec![Value::Int(1), Value::Int(2)]),
        });
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

        let script = Script::new(config::ScriptVariable {
            name: "var3".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: Some(vec![Value::String("$var2".to_string()), Value::Int(1)]),
        });

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

        let script = Script::new(config::ScriptVariable {
            name: "var3".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: Some(vec![
                Value::String("$VAR1".to_string()),
                Value::String("$var2".to_string()),
            ]),
        });

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        ctx.set_variable("var2", Value::Int(22));
        script.execute(&mut ctx).unwrap();

        let var3 = ctx.get_variable("var3").unwrap();
        assert_eq!(var3.as_int(), 33);
    }

    // VAR1 = 100
    // VAR1 = VAR1 + 10
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

        let script = Script::new(config::ScriptVariable {
            name: "VAR1".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
            args: Some(vec![Value::String("$VAR1".to_string()), Value::Int(11)]),
        });

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
