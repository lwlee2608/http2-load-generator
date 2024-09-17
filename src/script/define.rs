use crate::error::Error;
use crate::script::function;
use crate::script::function::FunctionApply;
use crate::script::Value;
use crate::script::{Script, ScriptContext, ScriptVariable};

pub struct DefScript {
    pub return_var_name: String,
    pub function: function::Function,
    pub args: Vec<ScriptVariable>,
}

impl Script for DefScript {
    fn execute(&self, ctx: &mut ScriptContext) -> Result<(), Error> {
        let value = match &self.function {
            function::Function::Plus(f) => {
                if self.args.len() == 2 {
                    let arg0 = self.args[0].get_value(ctx)?.as_int()?;
                    let arg1 = self.args[1].get_value(ctx)?.as_int()?;
                    let value = f.apply(arg0, arg1);
                    Value::Int(value)
                } else {
                    return Err(Error::ScriptError("Expects 2 arguments".into()));
                }
            }
            function::Function::Now(f) => {
                if self.args.len() == 1 {
                    let arg0 = self.args[0].get_value(ctx)?;
                    let arg0 = arg0.as_string()?;
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
                    let arg0 = arg0.as_string()?;
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
    use crate::scenario::Global;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    // let now = Now("%Y-%m-%d")
    #[test]
    fn test_script_now() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = DefScript {
            return_var_name: "now".to_string(),
            function: function::Function::Now(function::NowFunction {}),
            args: vec![ScriptVariable::Constant(Value::String(
                "%Y-%m-%d".to_string(),
            ))],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("now").unwrap();
        let value = result.as_string().unwrap();

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

        let script = DefScript {
            return_var_name: "value".to_string(),
            function: function::Function::Random(function::RandomFunction { min: 1, max: 10 }),
            args: vec![],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("value").unwrap();
        let value = result.as_int().unwrap();
        assert!(value >= 1 && value <= 10);
    }

    // let var1 = var2
    #[test]
    fn test_script_copy() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = DefScript {
            return_var_name: "var1".to_string(),
            function: function::Function::Copy(function::CopyFunction {}),
            args: vec![ScriptVariable::Variable("var2".into())],
        };

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        ctx.set_variable("var2", Value::Int(123456789));
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("var1").unwrap();
        assert_eq!(result.as_int().unwrap(), 123456789);
    }

    // let split = Split(":", 1)
    // let chargingDataRef = split.run("123:456")
    #[test]
    fn test_script_split() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = DefScript {
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
        assert_eq!(result.as_string().unwrap(), "456");
    }

    #[test]
    fn test_script_substring() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = DefScript {
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
        assert_eq!(result.as_string().unwrap(), "World");
    }

    // def chargingDataRef = location.substring(location.lastIndexOf('/') + 1)
    #[test]
    fn test_script_extract_location_header() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let mut ctx = ScriptContext::new(Arc::clone(&global));
        let location = Value::String("http://location:8080/test/v1/foo/123456".to_string());

        // def index = location.lastIndexOf('/')
        let script = DefScript {
            return_var_name: "location".to_string(),
            function: function::Function::LastIndexOf(function::LastIndexOfFunction {}),
            args: vec![
                ScriptVariable::Constant(location.clone()),
                ScriptVariable::Constant(Value::String("/".to_string())),
            ],
        };
        script.execute(&mut ctx).unwrap();

        let index = ctx.get_variable("location").unwrap().as_int().unwrap();
        assert_eq!(index, 32);

        // def chargingDataRef = location.substring(index + 1)
        let script = DefScript {
            return_var_name: "chargingDataRef".to_string(),
            function: function::Function::SubString(function::SubStringFunction {}),
            args: vec![
                ScriptVariable::Constant(location),
                ScriptVariable::Constant(Value::Int(index + 1)),
            ],
        };
        script.execute(&mut ctx).unwrap();

        let result = ctx.get_variable("chargingDataRef").unwrap();
        assert_eq!(result.as_string().unwrap(), "123456");
    }

    // let imsi = 1 + 2
    #[test]
    fn test_script_plus_constant() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = DefScript {
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
        assert_eq!(imsi.as_int().unwrap(), 3);
    }

    // local var2 = 22
    // local var3 = var2 + 1
    #[test]
    fn test_script_plus_constant_and_var() {
        // Global
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let script = DefScript {
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
        assert_eq!(var3.as_int().unwrap(), 23);
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

        let script = DefScript {
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
        assert_eq!(var3.as_int().unwrap(), 33);
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

        let script = DefScript {
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
        assert_eq!(var1.as_int().unwrap(), 111);

        // Check global
        let global = global.read().unwrap();
        let var1 = global.get_variable_value("VAR1").unwrap();
        assert_eq!(var1.as_int().unwrap(), 111);
    }
}
