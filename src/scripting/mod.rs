// TODO REMOVE ME
#![allow(dead_code)]
use crate::error::Error;
use crate::error::Error::ScriptError;
use crate::function;
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

    if parts[0] != "def" {
        return Err(ScriptError("invalid script, expected 'def'".into()));
    }

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
                    let func_args = caps.get(2).unwrap().as_str();

                    // TODO recursive function make more sense
                    if func_name == "substring" {
                        let arg0 = ScriptVariable::from_str(parts[0]);
                        let arg1 = ScriptVariable::from_str(func_args);
                        args.push(arg0);
                        args.push(arg1);

                        function::Function::SubString(function::SubStringFunction {})
                    } else if func_name == "lastIndexOf" {
                        let arg0 = ScriptVariable::from_str(parts[0]);
                        let arg1 = ScriptVariable::from_str(func_args);
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
                // else it's a simple assignment
                let arg0 = ScriptVariable::from_str(rhs);
                args.push(arg0);

                function::Function::Copy(function::CopyFunction {})
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

    pub fn add(&mut self, script: Script) {
        self.scripts.push(script);
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
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_scripting_plus() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut context = ScriptContext::new(global);

        // def foo = 16
        let script = parse_line("def foo = 16").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 16);

        // def count = foo
        let script = parse_line("def count = foo").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("count").unwrap().as_int(), 16);
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 16);

        // def count = count + 1
        let script = parse_line("def count = count + 1").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("count").unwrap().as_int(), 17);
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 16);

        // def foo = count + 1
        let script = parse_line("def foo = count + 10").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("count").unwrap().as_int(), 17);
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 27);
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
}
