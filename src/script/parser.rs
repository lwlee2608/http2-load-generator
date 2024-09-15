use crate::error::Error;
use crate::error::Error::ScriptError;
use crate::function;
use crate::script::assert::AssertOperator;
use crate::script::assert::AssertScript;
use crate::script::{Script, ScriptVariable};
use regex::Regex;

// Experimental

// Future Features:
// Support simple scripting language similiar to Karate
//
// def location = responseHeaders.location[0]
// def chargingDataRef = location.substring(location.lastIndexOf('/') + 1)
//
// def count = 0
// def count = count + 1
//

fn parse_line(line: &str) -> Result<Script, Error> {
    let parts: Vec<&str> = line.split(' ').collect();

    if parts.len() < 4 {
        return Err(ScriptError(
            "invalid script, expected at least 4 parts".into(),
        ));
    }

    match parts[0] {
        "def" => parse_def_script(parts.clone()),
        "assert" => parse_assert_script(parts.clone()),
        _ => Err(ScriptError(
            "invalid script, expected 'def' or 'assert'".into(),
        )),
    }
}

fn parse_assert_script(parts: Vec<&str>) -> Result<Script, Error> {
    let operator = parts[2];
    // if operator == "==" {
    //     // assert equal
    // }
    match operator {
        "==" => {
            // assert responseStatus == 200
            let lhs = ScriptVariable::from_str(parts[1]);
            let rhs = ScriptVariable::from_str(parts[3]);

            let _script = AssertScript {
                lhs,
                rhs,
                operator: AssertOperator::Equal,
            };

            // Ok(script)
            todo!()
        }
        "!=" => {
            todo!()
        }
        // _ => Err(ScriptError("invalid script, expected '=='".into())),
        _ => todo!(),
    }
}

fn parse_def_script(parts: Vec<&str>) -> Result<Script, Error> {
    if parts[2] != "=" {
        return Err(ScriptError("invalid script, expected '='".into()));
    }

    let mut args = vec![];

    // Crude way to determine function
    // * doesn't work with space char in string
    // This part need refactoring
    let function = match parts.len() {
        4 => {
            let rhs = parts[3];
            // Check if '.' is present
            // Example: location.substring(location.lastIndexOf('/'))
            // location will be the variable, substring will be the function
            let parts: Vec<&str> = rhs.split('.').collect();
            if parts.len() == 2 {
                if parts[1].contains('(') {
                    // function with arguments
                    // extract functions arguments using regex
                    let re = Regex::new(r"(\w+)\((.*)\)").unwrap();
                    let caps = re.captures(parts[1]).unwrap();
                    let func_name = caps.get(1).unwrap().as_str();
                    let func_arg = caps.get(2).unwrap().as_str();

                    // TODO recursive function make more sense
                    if func_name == "substring" {
                        let arg0 = ScriptVariable::from_str(parts[0]);
                        let arg1 = ScriptVariable::from_str(func_arg);
                        args.push(arg0);
                        args.push(arg1);

                        function::Function::SubString(function::SubStringFunction {})
                    } else if func_name == "lastIndexOf" {
                        let arg0 = ScriptVariable::from_str(parts[0]);
                        let arg1 = ScriptVariable::from_str(func_arg);
                        args.push(arg0);
                        args.push(arg1);

                        function::Function::LastIndexOf(function::LastIndexOfFunction {})
                    } else {
                        return Err(ScriptError("invalid script, expected function".into()));
                    }
                } else {
                    return Err(ScriptError("invalid script, expected function".into()));
                }
            } else {
                // check if it's a function
                let re = Regex::new(r"(\w+)\((.*)\)").unwrap();
                let caps = re.captures(rhs);
                if let Some(caps) = caps {
                    let func_name = caps.get(1).unwrap().as_str();
                    let func_args = caps.get(2).unwrap().as_str();
                    if func_name == "now" {
                        // no arg
                        // let arg0 = ScriptVariable::from_str(func_arg);
                        // args.push(arg0);
                        function::Function::Now(function::NowFunction {})
                    } else if func_name == "random" {
                        // expect two args
                        let func_args: Vec<&str> = func_args.split(',').collect();
                        if func_args.len() != 2 {
                            return Err(ScriptError(
                                "invalid script, random function requires 2 arguments".into(),
                            ));
                        }
                        let min = func_args[0].parse::<i32>().unwrap();
                        let max = func_args[1].parse::<i32>().unwrap();

                        function::Function::Random(function::RandomFunction { min, max })
                    } else {
                        return Err(ScriptError(format!(
                            "invalid script, function '{}' not found",
                            func_name
                        )));
                    }
                } else {
                    // else it's a simple assignment
                    let arg0 = ScriptVariable::from_str(rhs);
                    args.push(arg0);
                    function::Function::Copy(function::CopyFunction {})
                }
            }
        }
        6 => {
            let operator = parts[4];
            if operator != "+" {
                return Err(ScriptError(
                    "invalid script, only '+' operator is supported".into(),
                ));
            }
            let arg0 = ScriptVariable::from_str(parts[3]);
            let arg1 = ScriptVariable::from_str(parts[5]);
            args.push(arg0);
            args.push(arg1);

            function::Function::Plus(function::PlusFunction {})
        }
        _ => {
            return Err(ScriptError("invalid script, expected function".into()));
        }
    };

    let return_var_name = parts[1].to_string();

    let script = Script {
        return_var_name,
        function,
        args,
    };

    Ok(script)
}

pub struct Scripts {
    scripts: Vec<Script>,
}

impl Scripts {
    pub fn parse(raw_script: &str) -> Result<Scripts, Error> {
        let mut scripts = vec![];

        for line in raw_script.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let script = parse_line(line)?;
            scripts.push(script);
        }

        Ok(Scripts { scripts })
    }

    pub fn execute(&self, context: &mut crate::script::ScriptContext) -> Result<(), Error> {
        for script in &self.scripts {
            script.execute(context)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Global;
    use crate::script::ScriptContext;
    use crate::script::Value;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_scripting_now() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut context = ScriptContext::new(global);

        // def now = now()
        let script = parse_line("def now = now()").unwrap();
        script.execute(&mut context).unwrap();

        let now = context.get_variable("now").unwrap().as_string();

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(now.starts_with(&today));
    }

    #[test]
    fn test_scripting_random() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut context = ScriptContext::new(global);

        // TODO Support space in args
        // def random = random(100, 999)
        let script = parse_line("def random = random(100,999)").unwrap();
        script.execute(&mut context).unwrap();

        let random = context.get_variable("random").unwrap().as_int();
        assert!(random >= 100 && random <= 999);
    }

    #[test]
    fn test_scripting_extract_location_header() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut context = ScriptContext::new(global);

        let scripts = Scripts::parse(
            r"
                def location = 'http://localhost:8080/chargingData/123'
                def index = location.lastIndexOf('/')
                def index = index + 1
                def chargingDataRef = location.substring(index)
            ",
        )
        .unwrap();

        scripts.execute(&mut context).unwrap();

        assert_eq!(
            context.get_variable("location").unwrap().as_string(),
            "http://localhost:8080/chargingData/123"
        );
        assert_eq!(context.get_variable("index").unwrap().as_int(), 35,);
        assert_eq!(
            context.get_variable("chargingDataRef").unwrap().as_string(),
            "123"
        );
    }

    // #[test]
    // fn test_scripting_assert_status() {
    //     let global = Global::empty();
    //     let global = Arc::new(RwLock::new(global));
    //     let mut context = ScriptContext::new(global);
    //     context.set_local_variable("responseStatus", Value::Int(200));
    //
    //     let scripts = Scripts::parse(
    //         r"
    //             assert responseStatus == 200
    //         ",
    //     )
    //     .unwrap();
    //
    //     scripts.execute(&mut context).unwrap();
    // }
}
