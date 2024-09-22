use serde::Deserialize;
use serde::Serialize;
use serde_yaml;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub log_level: LogLevel,
    pub parallel: u8,
    pub runner: RunnerConfig,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let json = serde_json::to_string_pretty(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Copy, Clone)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Into<log::LevelFilter> for LogLevel {
    fn into(self) -> log::LevelFilter {
        match self {
            LogLevel::Off => log::LevelFilter::Off,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RunnerConfig {
    pub target_rps: u32,
    #[serde(deserialize_with = "humantime_duration_deserializer")]
    pub duration: Duration,
    pub batch_size: BatchSize,
    // pub auto_throttle: bool,
    pub base_url: String,
    pub global: Global,
    // #[serde(deserialize_with = "humantime_duration_deserializer")]
    pub requests: Vec<Request>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum BatchSize {
    Auto(String),
    Fixed(u32),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Global {
    pub scripts: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Request {
    pub name: String,
    pub method: String,
    pub path: String,
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub body: Option<String>,
    #[serde(deserialize_with = "humantime_duration_deserializer")]
    pub timeout: Duration,
    #[serde(rename = "before")]
    pub before: Option<Script>,
    #[serde(rename = "after")]
    pub after: Option<Script>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Script {
    pub scripts: String,
}

fn parse_override(override_str: &str) -> Result<(String, String), Box<dyn Error>> {
    let parts: Vec<&str> = override_str.split('=').collect();
    if parts.len() != 2 {
        return Err("Invalid override".into());
    }
    let key = parts[0];
    let value = parts[1];
    Ok((key.to_string(), value.to_string()))
}

fn apply_overrides(config: &mut serde_yaml::Value, overrides: Vec<String>) {
    for override_str in overrides {
        let (key, value) = parse_override(&override_str).unwrap();
        // println!("Override: {}={}", key, value);
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = &mut *config;
        for k in keys.iter().take(keys.len() - 1) {
            current = current
                .get_mut(k)
                .unwrap_or_else(|| panic!("Config not found: {}", k));
        }

        // Check if config exist
        if current.get(keys[keys.len() - 1]).is_none() {
            panic!("Config not found: {}", keys[keys.len() - 1]);
        }

        // Override the value
        current[keys[keys.len() - 1]] = if value.parse::<i64>().is_ok() {
            serde_yaml::Value::Number(serde_yaml::Number::from(value.parse::<i64>().unwrap()))
        } else {
            serde_yaml::Value::String(value)
        };
    }
}

pub fn read_yaml_file(path: &str, overrides: Vec<String>) -> Result<Config, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut value: serde_yaml::Value = serde_yaml::from_str(&contents)?;

    apply_overrides(&mut value, overrides);

    let config: Config = serde_yaml::from_value(value)?;
    Ok(config)
}

fn humantime_duration_deserializer<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    humantime::parse_duration(&s).map_err(|e| serde::de::Error::custom(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_serde() {
        let yaml_str = r#"
        log_level: "Debug"
        parallel: 1
        runner:
          target_rps: 100
          duration: 10s
          batch_size: 5
          # auto_throttle: true
          base_url: "http://localhost:8080/"
          global:
            scripts: |
                def COUNTER = 0
                def IMSI = 11000

          requests:
            - name: createSubscriber
              method: POST
              path: "/rsgateway/data/json/subscriber"
              headers:
              - content-type: "application/json"
              body: |
                {
                  "$": "MtxRequestSubscriberCreate",
                  "Name": "James Bond",
                  "FirstName": "James",
                  "LastName": "Bond",
                  "ContactEmail": "james.bond@email.com"
                }
              timeout: 3s  
              after:
                scripts: |
                  assert responseStatus == 200

            - name: querySubscriber
              method: GET
              path: "/rsgateway/data/json/subscriber/query/ExternalId/${externalId}"
              timeout: 3s  
              after:
                scripts: |
                  assert responseStatus == 200
    "#;
        let config: Config = serde_yaml::from_str(yaml_str).unwrap();

        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(config.parallel, 1);
        assert_eq!(config.runner.target_rps, 100);
        assert_eq!(config.runner.duration, Duration::from_secs(10));
        assert_eq!(config.runner.batch_size, BatchSize::Fixed(5));
        assert_eq!(config.runner.base_url, "http://localhost:8080/".to_string());
        assert_eq!(
            config.runner.global.scripts,
            "def COUNTER = 0\ndef IMSI = 11000\n"
        );
        assert_eq!(config.runner.requests.len(), 2);
        assert_eq!(config.runner.requests[0].name, "createSubscriber");
        assert_eq!(config.runner.requests[0].method, "POST");
        assert_eq!(
            config.runner.requests[0].path,
            "/rsgateway/data/json/subscriber"
        );
        assert_eq!(
            config.runner.requests[0].headers.as_ref().unwrap()[0]["content-type"],
            "application/json"
        );
        assert_eq!(
            config.runner.requests[0].body,
            Some(
                r#"{
  "$": "MtxRequestSubscriberCreate",
  "Name": "James Bond",
  "FirstName": "James",
  "LastName": "Bond",
  "ContactEmail": "james.bond@email.com"
}
"#
                .to_string()
            )
        );
        assert_eq!(
            config.runner.requests[0].after.as_ref().unwrap().scripts,
            "assert responseStatus == 200\n"
        );

        assert_eq!(config.runner.requests[1].name, "querySubscriber");
        assert_eq!(config.runner.requests[1].method, "GET");
        assert_eq!(
            config.runner.requests[1].path,
            "/rsgateway/data/json/subscriber/query/ExternalId/${externalId}"
        );
        assert_eq!(config.runner.requests[1].headers, None);
        assert_eq!(config.runner.requests[1].body, None);
        assert_eq!(
            config.runner.requests[1].after.as_ref().unwrap().scripts,
            "assert responseStatus == 200\n"
        );
    }
}
