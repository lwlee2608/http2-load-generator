use crate::error::Error;
use crate::error::Error::ScriptError;
use crate::variable;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum Function {
    Split(SplitFunction),
    Random(RandomFunction),
    Now(NowFunction),
    Plus(PlusFunction),
    Copy(CopyFunction),
    SubString(SubStringFunction),
    LastIndexOf(LastIndexOfFunction),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct SplitFunction {
    pub delimiter: String,
    pub index: SplitIndex,
}

impl SplitFunction {
    pub fn apply(&self, input: String) -> String {
        match self.index {
            SplitIndex::First => input
                .split(&self.delimiter)
                .next()
                .unwrap_or("")
                .to_string(),
            SplitIndex::Last => input
                .split(&self.delimiter)
                .last()
                .unwrap_or("")
                .to_string(),
            SplitIndex::Nth(index) => input
                .split(&self.delimiter)
                .nth(index)
                .unwrap_or("")
                .to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type", content = "value")]
pub enum SplitIndex {
    First,
    Last,
    Nth(usize),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct PlusFunction {}

impl PlusFunction {
    pub fn apply(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct CopyFunction {}

impl CopyFunction {
    pub fn apply(&self, input: &variable::Value) -> variable::Value {
        input.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct SubStringFunction {}

impl SubStringFunction {
    pub fn apply(&self, args: Vec<variable::Value>) -> Result<variable::Value, Error> {
        let (str, start, end) = match args.len() {
            2 => {
                let str = args[0].as_string();
                let start = args[1].as_int() as usize;
                let end = str.len();
                (str, start, end)
            }
            3 => {
                let str = args[0].as_string();
                let start = args[1].as_int() as usize;
                let end = args[2].as_int() as usize;
                (str, start, end)
            }
            _ => {
                return Err(ScriptError(
                    "substring function requires 2 or 3 arguments".to_string(),
                ))
            }
        };

        return Ok(variable::Value::String(
            str.chars().skip(start).take(end - start).collect(),
        ));
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct LastIndexOfFunction {
    pub pattern: String,
}

impl LastIndexOfFunction {
    pub fn apply(&self, input: String) -> i32 {
        input.rfind(&self.pattern).unwrap_or(0) as i32
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_split_function() {
        let f = SplitFunction {
            delimiter: ",".to_string(),
            index: SplitIndex::Nth(1),
        };
        assert_eq!(f.apply("a,b,c".to_string()), "b".to_string());
    }

    #[test]
    fn test_split_function_nth() {
        let f = SplitFunction {
            delimiter: ",".to_string(),
            index: SplitIndex::Nth(10),
        };
        assert_eq!(f.apply("a,b,c".to_string()), "".to_string());
    }

    #[test]
    fn test_split_function_last_index() {
        let f = SplitFunction {
            delimiter: "/".to_string(),
            index: SplitIndex::Last,
        };
        assert_eq!(
            f.apply("http://localhost:8080/test/v1/foo/12345".to_string()),
            "12345".to_string()
        );
    }

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

        assert_eq!(
            f.apply(args).unwrap(),
            variable::Value::String("bc".to_string())
        );

        let args = vec!["http://location:8080/test/v1/foo/123456".into(), 33.into()];

        assert_eq!(
            f.apply(args).unwrap(),
            variable::Value::String("123456".to_string())
        );
    }

    #[test]
    fn test_last_index_of_function() {
        let f = LastIndexOfFunction {
            pattern: "/".to_string(),
        };
        assert_eq!(
            f.apply("http://localhost:8080/test/v1/foo/12345".to_string()),
            33,
        );
    }
}
