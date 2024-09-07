// TODO REMOVE ME
#![allow(dead_code)]
use crate::function;
use crate::script::{Script, ScriptVariable};

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
pub struct Parser {}

impl Parser {
    pub fn parse_line(line: &str) -> Result<Script, String> {
        let parts: Vec<&str> = line.split(' ').collect();

        if parts.len() < 4 {
            return Err("invalid script, expected at least 4 parts".into());
        }

        if parts[0] != "def" {
            return Err("invalid script, expected 'def'".into());
        }

        if parts[2] != "=" {
            return Err("invalid script, expected '='".into());
        }

        // Crude way to determine function
        // * doesn't work with space char in string

        let mut args = vec![];
        let function = if parts.len() == 4 {
            let arg0 = ScriptVariable::from_str(parts[3]);
            args.push(arg0);

            function::Function::Copy(function::CopyFunction {})
        } else if parts.len() == 5 {
            return Err("invalid script, expected function".into());
        } else if parts.len() == 6 {
            let operator = parts[4];
            if operator != "+" {
                return Err("invalid script, only '+' operator is supported".into());
            }
            let arg0 = ScriptVariable::from_str(parts[3]);
            let arg1 = ScriptVariable::from_str(parts[5]);
            args.push(arg0);
            args.push(arg1);

            function::Function::Plus(function::PlusFunction {})
        } else {
            todo!()
        };

        let return_var_name = parts[1].to_string();

        let script = Script {
            return_var_name,
            function,
            args,
        };

        Ok(script)
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
        let script = Parser::parse_line("def foo = 16").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 16);

        // def count = foo
        let script = Parser::parse_line("def count = foo").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("count").unwrap().as_int(), 16);
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 16);

        // def count = count + 1
        let script = Parser::parse_line("def count = count + 1").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("count").unwrap().as_int(), 17);
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 16);

        // def foo = count + 1
        let script = Parser::parse_line("def foo = count + 10").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(context.get_variable("count").unwrap().as_int(), 17);
        assert_eq!(context.get_variable("foo").unwrap().as_int(), 27);
    }

    #[test]
    fn test_scripting_split() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut context = ScriptContext::new(global);

        // def location = "http://localhost:8080/chargingData/123"
        let script =
            Parser::parse_line("def location = 'http://localhost:8080/chargingData/123'").unwrap();
        script.execute(&mut context).unwrap();
        assert_eq!(
            context.get_variable("location").unwrap().as_string(),
            "http://localhost:8080/chargingData/123"
        );

        // def chargingDataRef = location.substring(location.lastIndexOf('/') + 1)
        //
        // def index = location.lastIndexOf('/')
        // let script = Parser::parse_line("def index = location.lastIndexOf('/')").unwrap();

        // def index = index + 1
        // def chargingDataRef = location.substring(index)
        //
        //
        // TODO
    }
}
