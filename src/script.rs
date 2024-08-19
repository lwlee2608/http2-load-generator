use crate::function;
use crate::variable::Value;
use crate::variable::Variable;

#[derive(Debug)]
pub struct ScriptError(String);

// #[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ScriptArgument {
    Variable(Variable),
    Constant(Value),
}

pub struct ScriptVariable {
    pub name: String,
    pub function: function::Function,
}

impl ScriptVariable {
    pub fn exec(&self, args: Vec<Value>) -> Result<Value, ScriptError> {
        log::debug!("executing script variable: {}", self.name);
        match &self.function {
            function::Function::Increment(f) => {
                if args.len() == 1 {
                    let arg0 = args[0].as_int();
                    let value = f.apply(arg0);
                    log::debug!("value: {}", value);
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expects 1 argument".into()));
                }
            }
            function::Function::Split(f) => {
                if args.len() == 1 {
                    let arg0 = args[0].as_string();
                    let value = f.apply(arg0);
                    Ok(Value::String(value))
                } else {
                    return Err(ScriptError("Expects 1 argument".into()));
                }
            }
            function::Function::Random(f) => {
                if args.len() == 0 {
                    let value = f.apply();
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expects 0 arguments".into()));
                }
            }
            function::Function::Now(f) => {
                return if args.len() == 1 {
                    let arg0 = args[0].as_string();
                    let value = f.apply(Some(arg0));
                    Ok(Value::String(value))
                } else if args.len() == 0 {
                    let value = f.apply(None);
                    Ok(Value::String(value))
                } else {
                    return Err(ScriptError("Expects 0 or 1 argument".into()));
                };
            }
            function::Function::Plus(f) => {
                return if args.len() == 2 {
                    let arg0 = args[0].as_int();
                    let arg1 = args[1].as_int();
                    let value = f.apply(arg0, arg1);
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expects 2 arguments".into()));
                }
            }
        }
    }
}

// #[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Script {
    pub variables: Vec<(ScriptVariable, Vec<ScriptArgument>)>,
}

impl Script {
    pub fn exec(&self, new_variables: Vec<Variable>) -> Vec<Variable> {
        let mut variables = vec![];
        for (v, args) in &self.variables {
            let args = args
                .iter()
                .map(|arg| match arg {
                    ScriptArgument::Variable(v) => {
                        // Check if the variable is in the new_variables
                        let new_variable = new_variables.iter().find(|nv| nv.name == v.name);
                        if let Some(nv) = new_variable {
                            // If it is, use the new value
                            nv.value.clone()
                        } else {
                            // Otherwise, use the old value
                            v.value.clone()
                        }
                    }
                    ScriptArgument::Constant(v) => v.clone(),
                })
                .collect::<Vec<Value>>();

            let value = v.exec(args).unwrap();
            variables.push(Variable {
                name: v.name.clone(),
                value,
                function: None,
            });
        }
        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // let now = Now()
    #[test]
    fn test_script_now() {
        let now = ScriptVariable {
            name: "now".to_string(),
            function: function::Function::Now(function::NowFunction {}),
        };
        let value = now
            .exec(vec![Value::String("%Y-%m-%d".to_string())])
            .unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(value.as_string().len() > 0);
        assert!(value.as_string().starts_with(&today));
    }

    // let random = Random(1, 10)
    #[test]
    fn test_script_random() {
        let random = ScriptVariable {
            name: "random".to_string(),
            function: function::Function::Random(function::RandomFunction { min: 1, max: 10 }),
        };
        let value = random.exec(vec![]).unwrap().as_int();
        assert!(value >= 1 && value <= 10);
    }

    // let counter = 5 + 1
    #[test]
    fn test_script_increment() {
        let counter = ScriptVariable {
            name: "counter".to_string(),
            function: function::Function::Increment(function::IncrementFunction {
                start: 0,
                step: 1,
                threshold: 10,
            }),
        };
        let value = counter.exec(vec![Value::Int(5)]).unwrap().as_int();
        assert_eq!(value, 6);
    }

    // let imsi = Split(":", 1)
    #[test]
    fn test_script_split() {
        let imsi = ScriptVariable {
            name: "imsi".to_string(),
            function: function::Function::Split(function::SplitFunction {
                delimiter: ":".to_string(),
                index: function::SplitIndex::Nth(1),
            }),
        };
        let value = imsi
            .exec(vec![Value::String("123:456".to_string())])
            .unwrap()
            .as_string();
        assert_eq!(value, "456".to_string());
    }

    // let imsi = 1 + 2
    #[test]
    fn test_script_plus() {
        let imsi = ScriptVariable {
            name: "imsi".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
        };
        let value = imsi
            .exec(vec![Value::Int(1), Value::Int(2)])
            .unwrap()
            .as_int();
        assert_eq!(value, 3);
    }

    // let var1 = 10
    // let var2 = var1 + 20
    // let var1 = 100
    // let var2 = var1 + 20
    #[test]
    fn test_script_exec() {
        // var1 = 10
        let var1 = ScriptArgument::Variable(Variable {
            name: "var1".to_string(),
            value: Value::Int(10),
            function: None,
        });

        let var2 = ScriptVariable {
            name: "var2".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
        };

        let const20 = ScriptArgument::Constant(Value::Int(20));

        let script = Script {
            variables: vec![(var2, vec![var1, const20])],
        };
        // var2 = var1 + 20
        let variables = script.exec(vec![]);
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0].name, "var2".to_string());
        assert_eq!(variables[0].value.as_int(), 30);

        // var1 = 100
        let var1 = Variable {
            name: "var1".to_string(),
            value: Value::Int(100),
            function: None,
        };

        // var2 = var1 + 20
        let variables = script.exec(vec![var1]);
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0].name, "var2".to_string());
        assert_eq!(variables[0].value.as_int(), 120);
    }
}
