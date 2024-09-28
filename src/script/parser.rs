use crate::error::Error;
use crate::error::Error::ScriptError;
use crate::script::assert::AssertOperator;
use crate::script::assert::AssertScript;
use crate::script::define::DefScript;
use crate::script::function::Function;
use crate::script::function::{
    CopyFunction, LastIndexOfFunction, NowFunction, PlusFunction, RandomFunction, SubStringFunction,
};
use crate::script::Script;
use crate::script::Variable;
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

fn parse_line(line: &str) -> Result<Box<dyn Script>, Error> {
    // TODO trim double space

    let parts: Vec<&str> = line.split(' ').collect();

    if parts.len() < 4 {
        return Err(ScriptError(
            "invalid script, expected at least 4 parts".into(),
        ));
    }

    match parts[0] {
        "def" => {
            let s = parse_def_script(parts.clone())?;
            Ok(Box::new(s))
        }
        "assert" => {
            let s = parse_assert_script(parts.clone())?;
            Ok(Box::new(s))
        }
        _ => Err(ScriptError(
            "invalid script, expected 'def' or 'assert'".into(),
        )),
    }
}

fn parse_assert_script(parts: Vec<&str>) -> Result<impl Script, Error> {
    let operator = parts[2];
    match operator {
        "==" => {
            let lhs = Variable::from_str(parts[1]);
            let rhs = Variable::from_str(parts[3]);
            Ok(AssertScript {
                lhs,
                rhs,
                operator: AssertOperator::Equal,
            })
        }
        "!=" => {
            let lhs = Variable::from_str(parts[1]);
            let rhs = Variable::from_str(parts[3]);
            Ok(AssertScript {
                lhs,
                rhs,
                operator: AssertOperator::NotEqual,
            })
        }
        _ => Err(ScriptError(
            "invalid script, operator '==' or '!=' expected".into(),
        )),
    }
}

fn parse_def_script(parts: Vec<&str>) -> Result<impl Script, Error> {
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
                        let arg0 = Variable::from_str(parts[0]);
                        let arg1 = Variable::from_str(func_arg);
                        args.push(arg0);
                        args.push(arg1);

                        Function::SubString(SubStringFunction)
                    } else if func_name == "lastIndexOf" {
                        let arg0 = Variable::from_str(parts[0]);
                        let arg1 = Variable::from_str(func_arg);
                        args.push(arg0);
                        args.push(arg1);

                        Function::LastIndexOf(LastIndexOfFunction)
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
                        // let arg0 = Variable::from_str(func_arg);
                        // args.push(arg0);
                        Function::Now(NowFunction)
                    } else if func_name == "random" {
                        // expect two args
                        let func_args: Vec<&str> = func_args.split(',').collect();
                        if func_args.len() != 2 {
                            return Err(ScriptError(
                                "invalid script, random function requires 2 arguments".into(),
                            ));
                        }
                        let arg0 = Variable::from_str(func_args[0]);
                        let arg1 = Variable::from_str(func_args[1]);
                        args.push(arg0);
                        args.push(arg1);
                        Function::Random(RandomFunction)
                    } else {
                        return Err(ScriptError(format!(
                            "invalid script, function '{}' not found",
                            func_name
                        )));
                    }
                } else {
                    // else it's a simple assignment
                    let arg0 = Variable::from_str(rhs);
                    args.push(arg0);
                    Function::Copy(CopyFunction)
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
            let arg0 = Variable::from_str(parts[3]);
            let arg1 = Variable::from_str(parts[5]);
            args.push(arg0);
            args.push(arg1);

            Function::Plus(PlusFunction)
        }
        _ => {
            return Err(ScriptError("invalid script, expected function".into()));
        }
    };

    let return_var_name = parts[1].to_string();

    let script = DefScript {
        return_var_name,
        function,
        args,
    };

    Ok(script)
}

pub struct Scripts {
    scripts: Vec<Box<dyn Script>>,
}

impl Scripts {
    pub fn parse(raw_script: &str) -> Result<Scripts, Error> {
        let mut scripts: Vec<Box<dyn Script>> = vec![];

        for line in raw_script.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
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
    use crate::script::Global;
    use crate::script::ScriptContext;
    use crate::script::Value;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_scripting_now() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut context = ScriptContext::new(global);

        // def now = now()
        let script = parse_line("def now = now()").unwrap();
        script.execute(&mut context).unwrap();

        let now = context.get_variable("now").unwrap().as_string().unwrap();

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

        let random = context.get_variable("random").unwrap().as_int().unwrap();
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
            context
                .get_variable("location")
                .unwrap()
                .as_string()
                .unwrap(),
            "http://localhost:8080/chargingData/123"
        );
        assert_eq!(context.get_variable("index").unwrap().as_int().unwrap(), 35,);
        assert_eq!(
            context
                .get_variable("chargingDataRef")
                .unwrap()
                .as_string()
                .unwrap(),
            "123"
        );
    }

    #[test]
    fn test_scripting_assert_status() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut context = ScriptContext::new(global);
        context.set_local_variable("responseStatus", Value::Int(200));

        let scripts = Scripts::parse(
            r"
                assert responseStatus == 200
            ",
        )
        .unwrap();

        scripts.execute(&mut context).unwrap();
    }

    // def contentType = responseHeaders['contentType'][0]
    // assert contentType == 'application/json'
    #[test]
    fn test_script_assert_headers() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut ctx = ScriptContext::new(Arc::clone(&global));
        let mut headers = HashMap::new();
        headers.insert(
            "contentType".to_string(),
            Value::List(vec!["application/json".into()]),
        );
        ctx.set_variable("responseHeaders", Value::Map(headers));

        let script = Scripts::parse(
            r"
                def contentTypes = responseHeaders['contentType']
                def contentType = contentTypes[0]
                assert contentType == 'application/json'
            ",
        )
        .unwrap();

        script.execute(&mut ctx).unwrap();

        assert_eq!(
            ctx.get_variable("contentType")
                .unwrap()
                .as_string()
                .unwrap(),
            "application/json"
        );
    }
}
