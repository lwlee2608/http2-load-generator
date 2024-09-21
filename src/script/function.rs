use crate::error::Error;
use crate::error::Error::ScriptError;
use crate::script::Value;
use rand::Rng;

pub trait FunctionApply {
    fn apply(&self, args: Vec<Value>) -> Result<Value, Error>;
}

#[derive(Debug, PartialEq, Clone)]
pub enum Function {
    Random(RandomFunction),
    Now(NowFunction),
    Plus(PlusFunction),
    Copy(CopyFunction),
    SubString(SubStringFunction),
    LastIndexOf(LastIndexOfFunction),
}

#[derive(Debug, PartialEq, Clone)]
pub struct RandomFunction {
    pub min: i32,
    pub max: i32,
}

impl RandomFunction {
    pub fn apply(&self) -> i32 {
        let mut rng = rand::thread_rng();
        let value = rng.gen_range(self.min..=self.max);
        value
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct NowFunction {}

impl NowFunction {
    pub fn apply(&self, format: Option<String>) -> String {
        let now = chrono::Utc::now();
        return if let Some(format) = format {
            return now.format(&format).to_string();
        } else {
            now.to_rfc3339()
        };
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PlusFunction {}

impl PlusFunction {
    pub fn apply(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CopyFunction {}

impl FunctionApply for CopyFunction {
    fn apply(&self, args: Vec<Value>) -> Result<Value, Error> {
        match args.len() {
            1 => Ok(args[0].clone()),
            _ => Err(ScriptError("copy function requires 1 argument".to_string())),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SubStringFunction {}

impl FunctionApply for SubStringFunction {
    fn apply(&self, args: Vec<Value>) -> Result<Value, Error> {
        let (input_str, start, end) = match args.len() {
            2 => {
                let input_str = args[0].as_string()?;
                let start = args[1].as_int()? as usize;
                let end = input_str.len();
                (input_str, start, end)
            }
            3 => {
                let input_str = args[0].as_string()?;
                let start = args[1].as_int()? as usize;
                let end = args[2].as_int()? as usize;
                (input_str, start, end)
            }
            _ => {
                return Err(ScriptError(
                    "substring function requires 2 or 3 arguments".to_string(),
                ))
            }
        };

        return Ok(Value::String(
            input_str.chars().skip(start).take(end - start).collect(),
        ));
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LastIndexOfFunction {}

impl FunctionApply for LastIndexOfFunction {
    fn apply(&self, args: Vec<Value>) -> Result<Value, Error> {
        match args.len() {
            2 => {
                let input_str = args[0].as_string()?;
                let pattern = args[1].as_string()?;
                let index = input_str.rfind(&pattern).unwrap_or(0) as i32;
                Ok(Value::Int(index))
            }
            _ => Err(ScriptError(
                "lastIndexOf function requires 2 argument".to_string(),
            )),
        }
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_plus_function() {
        let f = PlusFunction {};
        assert_eq!(f.apply(1, 2), 3);
    }

    #[test]
    fn test_random_function() {
        let f = RandomFunction { min: 1, max: 10 };
        let value = f.apply();
        assert!(value >= 1 && value <= 10);
    }

    #[test]
    fn test_substring_function() {
        let f = SubStringFunction {};
        let args = vec!["abcdef".into(), 1.into(), 3.into()];

        assert_eq!(f.apply(args).unwrap(), Value::String("bc".to_string()));

        let args = vec!["http://location:8080/test/v1/foo/123456".into(), 33.into()];

        assert_eq!(f.apply(args).unwrap(), Value::String("123456".to_string()));
    }

    #[test]
    fn test_last_index_of_function() {
        let f = LastIndexOfFunction {};
        let args = vec!["http://localhost:8080/test/v1/foo/12345".into(), "/".into()];
        assert_eq!(f.apply(args).unwrap(), Value::Int(33),);
    }
}
