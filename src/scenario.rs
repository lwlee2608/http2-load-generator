use crate::config;
use crate::function;
use crate::http_api::HttpRequest;
use crate::http_api::HttpResponse;
use crate::script::ScriptContext;
use crate::script::Scripts;
use crate::script::Value;
use http::Method;
use http::StatusCode;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct Request {
    pub uri: String,
    pub uri_var_name: Vec<String>,
    pub method: Method,
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub body: Option<String>,
    pub body_var_name: Vec<String>,
    // pub body: Option<serde_json::Value>,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct Response {
    pub status: http::StatusCode,
    pub headers: Option<Vec<HeadersAssert>>,
    pub body: Option<Vec<BodyAssert>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HeadersAssert {
    pub name: String,
    pub value: HeadersValueAssert,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type", content = "value")]
pub enum HeadersValueAssert {
    NotNull,
    Equal(String),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BodyAssert {
    pub name: String,
    pub value: BodyValueAssert,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type", content = "value")]
pub enum BodyValueAssert {
    NotNull,
    EqualString(String),
    EqualNumber(f64),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResponseDefine {
    pub name: String,
    pub from: DefineFrom,
    pub path: String,
    pub function: Option<function::Function>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Copy, Clone)]
pub enum DefineFrom {
    Header,
    Body,
}

// #[derive(Clone)]
pub struct Scenario {
    pub name: String,
    pub base_url: String,
    pub request: Request,
    pub response: Response,
    pub response_defines: Vec<ResponseDefine>,
    pub assert_panic: bool,
    pub pre_script: Option<Scripts>,
    pub post_script: Option<Scripts>,
}

impl Scenario {
    pub fn new(config: &config::Scenario, base_url: &str) -> Self {
        // Find variables in body and url
        let body_var_name =
            Scenario::find_variable_name(&config.request.body.clone().unwrap_or_default());
        let uri_var_name = Scenario::find_variable_name(&config.request.path);

        //Local Variable
        let mut response_defines = vec![];
        match &config.response.define {
            Some(define) => {
                for v in define {
                    let response_define = v.clone();
                    response_defines.push(response_define);
                }
            }
            None => {}
        }

        // Requets
        let request = Request {
            uri: config.request.path.clone(),
            uri_var_name,
            method: config.request.method.parse().unwrap(),
            headers: config.request.headers.clone(),
            body: config.request.body.clone(),
            body_var_name,
            timeout: config.request.timeout,
        };

        // Response
        let response = Response {
            status: StatusCode::from_u16(config.response.assert.status).unwrap(),
            headers: config.response.assert.headers.clone(),
            body: config.response.assert.body.clone(),
        };

        let pre_script = match &config.pre_script {
            Some(s) => {
                let scripts = Scripts::parse(&s.scripts).unwrap();
                Some(scripts)
            }
            None => None,
        };

        let post_script = match &config.post_script {
            Some(s) => {
                let scripts = Scripts::parse(&s.scripts).unwrap();
                Some(scripts)
            }
            None => None,
        };

        Scenario {
            name: config.name.clone(),
            base_url: base_url.into(),
            request,
            response,
            response_defines,
            assert_panic: true,
            pre_script,
            post_script,
        }
    }

    fn find_variable_name(str: &str) -> Vec<String> {
        let variable_pattern = Regex::new(r"\$\{([^}]+)\}").unwrap();
        let mut var_name = vec![];
        for caps in variable_pattern.captures_iter(str) {
            let cap = caps[1].to_string();
            var_name.push(cap);
        }
        var_name
    }

    pub fn new_request(
        &mut self,
        ctx: &ScriptContext,
    ) -> Result<HttpRequest, Box<dyn std::error::Error>> {
        let body = match &self.request.body {
            Some(body) => {
                let mut body = body.clone();

                // Apply vairables replace in body
                for name in &self.request.body_var_name {
                    let value = ctx.must_get_variable(&name)?;
                    let value = match value {
                        Value::Int(v) => v.to_string(),
                        Value::String(v) => v,
                    };
                    body = body.replace(&format!("${{{}}}", name), &value);
                }

                Some(serde_json::from_str(&body).unwrap())
            }
            None => None,
        };

        let uri = {
            let mut uri = self.request.uri.clone();

            // Apply vairables replace in uri
            for name in &self.request.uri_var_name {
                let value = ctx.must_get_variable(&name)?;
                let value = match value {
                    Value::Int(v) => v.to_string(),
                    Value::String(v) => v,
                };
                uri = uri.replace(&format!("${{{}}}", name), &value);
            }
            uri
        };

        // Add base_url to uri
        let uri = format!("{}{}", self.base_url, uri);

        Ok(HttpRequest {
            uri,
            method: self.request.method.clone(),
            headers: self.request.headers.clone(),
            body,
            timeout: self.request.timeout.clone(),
        })
    }

    pub fn assert_response(&self, response: &HttpResponse) -> bool {
        match self.check_response(response) {
            Ok(_) => true,
            Err(err) => {
                if self.assert_panic {
                    panic!("{}", err);
                } else {
                    log::error!("{}", err);
                }
                false
            }
        }
    }

    fn check_response(&self, response: &HttpResponse) -> Result<(), Box<dyn std::error::Error>> {
        // Check Status
        if self.response.status != response.status {
            return Err(format!(
                "Expected status code: {:?}, got: {:?}",
                self.response.status, response.status
            )
            .into());
        }

        // Check Headers
        if self.response.headers.is_some() {
            let headers = self.response.headers.as_ref().unwrap();
            for h in headers {
                let value = h.value.clone();
                let header = response
                    .headers
                    .get(&h.name)
                    .map(|hdr| hdr.to_str().unwrap());

                match value {
                    HeadersValueAssert::NotNull => {
                        if header.is_none() {
                            return Err(
                                format!("Header '{}' is expected but not found", h.name).into()
                            );
                        }
                    }
                    HeadersValueAssert::Equal(v) => {
                        if header.is_none() {
                            return Err(
                                format!("Header '{}' is expected but not found", h.name).into()
                            );
                        }
                        if header.unwrap() != v {
                            return Err(format!(
                                "Header '{}' is expected to be '{}' but got '{}'",
                                h.name,
                                v,
                                header.unwrap()
                            )
                            .into());
                        }
                    }
                }
            }
        }

        // Check Body
        if self.response.body.is_some() {
            let body_assert = self.response.body.as_ref().unwrap();

            let body = response.body.as_ref();
            if body == None {
                return Err("Body is expected but not found".into());
            }
            let body = body.unwrap();

            for b in body_assert {
                // Support nested json
                let keys = b.name.split('.').collect::<Vec<&str>>();
                let mut current = &mut body.clone(); // not sure how to get away without clone
                for key in keys.iter().take(keys.len() - 1) {
                    match current.get_mut(*key) {
                        Some(value) => {
                            current = value;
                        }
                        None => {
                            return Err(format!(
                                "Field '{}' is expected from body assert '{}' but not found",
                                key, b.name
                            )
                            .into());
                        }
                    }
                }

                let name_assert = keys.last().unwrap();
                let value_assert = b.value.clone();
                let value = current.get(name_assert);

                if value.is_none() {
                    return Err(format!("Field '{}' is expected but not found", b.name).into());
                }

                if value.unwrap().is_array() {
                    return Err("Asserting array fields in response body is not supported".into());
                }

                if value.unwrap().is_object() {
                    return Err("Error when parsing nested json in response body".into());
                }

                let value = value.unwrap();

                match value_assert {
                    BodyValueAssert::NotNull => {}
                    BodyValueAssert::EqualString(v) => {
                        if value.as_str().unwrap() != v {
                            return Err(format!(
                                "Body '{}' is expected to be '{}' but got '{}'",
                                b.name,
                                v,
                                value.as_str().unwrap()
                            )
                            .into());
                        }
                    }
                    BodyValueAssert::EqualNumber(v) => {
                        if value.is_f64() {
                            if value.as_f64().unwrap() != v {
                                return Err(format!(
                                    "Body '{}' is expected to be '{}' but got '{}'",
                                    b.name,
                                    v,
                                    value.as_f64().unwrap()
                                )
                                .into());
                            }
                        } else if value.is_i64() {
                            if value.as_i64().unwrap() as f64 != v {
                                return Err(format!(
                                    "Body '{}' is expected to be '{}' but got '{}'",
                                    b.name,
                                    v,
                                    value.as_i64().unwrap()
                                )
                                .into());
                            }
                        } else {
                            return Err(
                                format!("Body '{}' is expected to be number", b.name).into()
                            );
                        }
                    }
                }
            }
        }

        return Ok(());
    }

    pub fn from_response(
        &self,
        ctx: &mut ScriptContext,
        response: &HttpResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Http Status
        ctx.set_variable(
            "responseStatus",
            Value::Int(response.status.as_u16().into()),
        );

        // Obsolete
        for v in &self.response_defines {
            match v.from {
                DefineFrom::Header => {
                    let headers = &response.headers;
                    if let Some(header) = headers.get(&v.path) {
                        let value = header.to_str().unwrap();
                        log::debug!(
                            "Set local var from header: '{}', name: '{}' value: '{}'",
                            v.path,
                            v.name,
                            value,
                        );
                        let value = Value::String(value.into());
                        ctx.set_variable(&v.name, value);
                    }
                }
                DefineFrom::Body => {
                    if let Some(body) = &response.body {
                        let value = jsonpath_lib::select(&body, &v.path).unwrap();
                        let value = value.get(0).unwrap();

                        log::debug!(
                            "Set local var from json field: '{}', name: '{}' value: '{}'",
                            v.path,
                            v.name,
                            value,
                        );

                        let value = if value.is_f64() {
                            Value::Int(value.as_f64().unwrap() as i32)
                        } else if value.is_i64() {
                            Value::Int(value.as_i64().unwrap() as i32)
                        } else {
                            Value::String(value.as_str().unwrap().to_string())
                        };

                        ctx.set_variable(&v.name, value);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn run_pre_script(&self, ctx: &mut ScriptContext) {
        log::debug!("run_pre_script");

        if let Some(s) = &self.pre_script {
            s.execute(ctx).unwrap();
        }

        // print all variables from context
        for (k, v) in ctx.local.variables.iter() {
            log::debug!("pre context variable: {} = {:?}", k, v);
        }
    }

    pub fn run_post_script(&self, ctx: &mut ScriptContext) {
        log::debug!("run_post_script");

        if let Some(s) = &self.post_script {
            s.execute(ctx).unwrap();
        }

        // print all variables from context
        for (k, v) in ctx.local.variables.iter() {
            log::debug!("post context variable: {} = {:?}", k, v);
        }
    }
}

pub struct Global {
    pub variables: HashMap<String, Value>,
}

impl Global {
    pub fn new(_configs: config::Global) -> Self {
        Global {
            variables: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Global {
            variables: HashMap::new(),
        }
    }

    pub fn get_variable_value(&self, variable_name: &str) -> Option<&Value> {
        self.variables.get(variable_name)
        // .map(|v| v.clone())
    }

    pub fn update_variable_value(&mut self, variable_name: &str, value: Value) {
        if self.variables.contains_key(variable_name) {
            self.variables.insert(variable_name.into(), value);
        }
    }

    pub fn insert_variable(&mut self, variable_name: &str, value: Value) {
        self.variables.insert(variable_name.into(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_scenario_new_request() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let body = r#"{"test": "${var1}_${var2}"}"#;
        let uri = "/endpoint/foo/${foo_id}";
        let body_var_name = Scenario::find_variable_name(&body);
        let uri_var_name = Scenario::find_variable_name(&uri);

        let mut scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            request: Request {
                uri: uri.into(),
                uri_var_name,
                method: Method::GET,
                headers: Some(vec![headers]),
                body: Some(body.into()),
                body_var_name,
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: None,
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        let mut ctx = ScriptContext::new(global);
        ctx.set_variable("var1", Value::Int(0));
        ctx.set_variable("var2", Value::Int(100));
        ctx.set_variable("foo_id", Value::String("1-2-3-4".into()));

        let request = scenario.new_request(&ctx).unwrap();
        assert_eq!(request.uri, "http://localhost:8080/endpoint/foo/1-2-3-4");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "0_100"}"#).unwrap())
        );
    }

    #[test]
    fn test_scenario_assert_response() {
        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            request: Request {
                uri: "/endpoint".into(),
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: None,
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        let response1 = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        let response2 = HttpResponse {
            status: StatusCode::NOT_FOUND,
            headers: http::HeaderMap::new(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        assert_eq!(true, scenario.assert_response(&response1));
        assert_eq!(false, scenario.assert_response(&response2));
    }

    #[test]
    fn test_scenario_check_response_with_body() {
        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            request: Request {
                uri: "/endpoint".into(),
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: Some(vec![HeadersAssert {
                    name: "Content-Type".into(),
                    value: HeadersValueAssert::NotNull,
                }]),
                body: Some(vec![BodyAssert {
                    name: "Result".into(),
                    value: BodyValueAssert::EqualNumber(0.0),
                }]),
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        // Missing content-type header
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!(
                "Header 'Content-Type' is expected but not found",
                err.to_string()
            ),
        }

        // Missing response body
        let mut headers = http::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!("Body is expected but not found", err.to_string()),
        }

        // Missing field 'Result' in response body
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: Some(serde_json::from_str(r#"{"ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!("Field 'Result' is expected but not found", err.to_string()),
        }

        // Mismatch value in response body
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: Some(serde_json::from_str(r#"{"Result": 1, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!(
                "Body 'Result' is expected to be '0' but got '1'",
                err.to_string()
            ),
        }

        // All good
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: Some(serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => {}
            Err(err) => panic!("{}", err),
        }
    }

    #[test]
    fn test_scenario_check_response_with_nested_body() {
        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            request: Request {
                uri: "/endpoint".into(),
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: Some(vec![BodyAssert {
                    name: "Foo.Bar".into(),
                    value: BodyValueAssert::EqualString("Baz".into()),
                }]),
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        // Test Missing Field 'Foo'
        let body = serde_json::json!({
            "Result": 0,
            "Bar": "Baz"
        });

        let response = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Some(body),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!(
                "Field 'Foo' is expected from body assert 'Foo.Bar' but not found",
                err.to_string()
            ),
        }

        // ALl Good
        let body = serde_json::json!({
            "Result": 0,
            "Foo": {
                "Bar": "Baz"
            }
        });

        let response = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Some(body),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => {}
            Err(err) => panic!("{}", err),
        }
    }

    #[test]
    fn test_scenario_from_response() {
        let response_defines = vec![ResponseDefine {
            name: "ObjectId".into(),
            from: DefineFrom::Body,
            path: "$.ObjectId".into(),
            function: None,
        }];
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            request: Request {
                uri: "/endpoint".into(),
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: None,
            },
            response_defines,
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        let mut ctx = ScriptContext::new(global);

        scenario
            .from_response(
                &mut ctx,
                &HttpResponse {
                    status: StatusCode::OK,
                    headers: http::HeaderMap::new(),
                    body: Some(
                        serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap(),
                    ),
                    request_start: std::time::Instant::now(),
                    retry_count: 0,
                },
            )
            .unwrap();

        let object_id = ctx.get_variable("ObjectId").unwrap();

        assert_eq!(object_id, Value::String("0-1-2-3".into()));
    }
}
