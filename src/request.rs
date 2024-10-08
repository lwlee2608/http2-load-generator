use crate::config;
use crate::http_api::HttpRequest;
use crate::http_api::HttpResponse;
use crate::script::ScriptContext;
use crate::script::Scripts;
use crate::script::Value;
use http::Method;
use regex::Regex;
use std::collections::HashMap;
use std::time::Duration;

pub struct Request {
    pub name: String,
    pub base_url: String,
    pub method: Method,
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub uri: String,
    pub uri_var_name: Vec<String>,
    pub body: Option<String>,
    pub body_var_name: Vec<String>,
    // pub body: Option<serde_json::Value>,
    pub timeout: Duration,
    pub before: Option<Scripts>,
    pub after: Option<Scripts>,
}

impl Request {
    pub fn new(config: &config::Request, base_url: &str) -> Self {
        // Find variables in body and url
        let body_var_name = Request::find_variable_name(&config.body.clone().unwrap_or_default());
        let uri_var_name = Request::find_variable_name(&config.path);

        let before = match &config.before {
            Some(s) => {
                let scripts = Scripts::parse(&s.scripts).unwrap();
                Some(scripts)
            }
            None => None,
        };

        let after = match &config.after {
            Some(s) => {
                let scripts = Scripts::parse(&s.scripts).unwrap();
                Some(scripts)
            }
            None => None,
        };

        Request {
            name: config.name.clone(),
            base_url: base_url.into(),
            method: config.method.parse().unwrap(),
            headers: config.headers.clone(),
            uri: config.path.clone(),
            uri_var_name,
            body: config.body.clone(),
            body_var_name,
            timeout: config.timeout,
            before,
            after,
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

    pub fn new_http_request(
        &mut self,
        ctx: &ScriptContext,
    ) -> Result<HttpRequest, Box<dyn std::error::Error>> {
        let body = match &self.body {
            Some(body) => {
                let mut body = body.clone();

                // Apply vairables replace in body
                for name in &self.body_var_name {
                    let value = ctx.must_get_variable(&name)?;
                    let value = value.as_string()?;
                    body = body.replace(&format!("${{{}}}", name), &value);
                }

                Some(serde_json::from_str(&body).unwrap())
            }
            None => None,
        };

        let uri = {
            let mut uri = self.uri.clone();

            // Apply vairables replace in uri
            for name in &self.uri_var_name {
                let value = ctx.must_get_variable(&name)?;
                let value = value.as_string()?;
                uri = uri.replace(&format!("${{{}}}", name), &value);
            }
            uri
        };

        // Add base_url to uri
        let uri = format!("{}{}", self.base_url, uri);

        Ok(HttpRequest {
            uri,
            method: self.method.clone(),
            headers: self.headers.clone(),
            body,
            timeout: self.timeout.clone(),
        })
    }

    pub fn from_response(
        &self,
        ctx: &mut ScriptContext,
        response: &HttpResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TODO Skip these if post-script don't use the response variables
        //
        // Http Status
        ctx.set_variable(
            "responseStatus",
            Value::Int(response.status.as_u16().into()),
        );

        // Http Headers
        let mut header_map: HashMap<String, Value> = HashMap::new();
        for (name, value) in response.headers.iter() {
            let name = name.as_str();
            let name = name.to_lowercase();
            let value = value.to_str().unwrap();

            let header_list = if let Some(v) = header_map.get(&name) {
                let mut header_list = v.as_list().unwrap();
                header_list.push(Value::String(value.into()));
                header_list
            } else {
                vec![Value::String(value.into())]
            };

            header_map.insert(name.into(), Value::List(header_list));
        }
        ctx.set_variable("responseHeaders", Value::Map(header_map));

        // Http Body
        fn body_to_script_value(value: &serde_json::Value) -> Value {
            match value {
                serde_json::Value::String(s) => return Value::String(s.clone()),
                serde_json::Value::Number(n) => return Value::Int(n.as_i64().unwrap() as i32),
                serde_json::Value::Object(map) => {
                    let mut body_map = HashMap::new();
                    for (k, v) in map.iter() {
                        body_map.insert(k.clone(), body_to_script_value(v));
                    }
                    return Value::Map(body_map);
                }
                serde_json::Value::Array(a) => {
                    let mut list = vec![];
                    for v in a.iter() {
                        list.push(body_to_script_value(v));
                    }
                    Value::List(list)
                }
                // serde_json::Value::Bool(b) => Value::Bool(*b),
                // serde_json::Value::Null => Value::Null,
                _ => todo!(),
            }
        }
        if let Some(body) = &response.body {
            ctx.set_variable("response", body_to_script_value(&body));
        }

        Ok(())
    }

    pub fn run_before(&self, ctx: &mut ScriptContext) {
        log::debug!("run_before");

        if let Some(s) = &self.before {
            s.execute(ctx).unwrap();
        }

        // print all variables from context
        if log::log_enabled!(log::Level::Debug) {
            for (k, v) in ctx.local.variables.iter() {
                log::debug!("pre context variable: {} = {:?}", k, v);
            }
        }
    }

    pub fn run_after(&self, ctx: &mut ScriptContext) {
        log::debug!("run_after");

        if let Some(s) = &self.after {
            s.execute(ctx).unwrap();
        }

        // print all variables from context
        if log::log_enabled!(log::Level::Debug) {
            for (k, v) in ctx.local.variables.iter() {
                log::debug!("post context variable: {} = {:?}", k, v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Global;
    use http::StatusCode;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_request_new_http_request() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let body = r#"{"test": "${var1}_${var2}"}"#;
        let uri = "/endpoint/foo/${foo_id}";
        let body_var_name = Request::find_variable_name(&body);
        let uri_var_name = Request::find_variable_name(&uri);

        let mut request = Request {
            name: "Request_1".into(),
            base_url: "http://localhost:8080".into(),
            method: Method::GET,
            headers: Some(vec![headers]),
            uri: uri.into(),
            uri_var_name,
            body: Some(body.into()),
            body_var_name,
            timeout: Duration::from_secs(3),
            before: None,
            after: None,
        };

        let mut ctx = ScriptContext::new(global);
        ctx.set_variable("var1", Value::Int(0));
        ctx.set_variable("var2", Value::Int(100));
        ctx.set_variable("foo_id", Value::String("1-2-3-4".into()));

        let request = request.new_http_request(&ctx).unwrap();
        assert_eq!(request.uri, "http://localhost:8080/endpoint/foo/1-2-3-4");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "0_100"}"#).unwrap())
        );
    }

    #[test]
    fn test_request_from_response() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let request = Request {
            name: "Request_1".into(),
            base_url: "http://localhost:8080".into(),
            method: Method::GET,
            headers: None,
            uri: "/endpoint".into(),
            uri_var_name: vec![],
            body: None,
            body_var_name: vec![],
            timeout: Duration::from_secs(3),
            before: None,
            after: None,
        };

        let mut ctx = ScriptContext::new(global);

        request
            .from_response(
                &mut ctx,
                &HttpResponse {
                    status: StatusCode::OK,
                    headers: http::HeaderMap::new(),
                    body: Some(
                        serde_json::from_str(
                            r#"
                            {
                                "Result": 0,
                                "ObjectId": "0-1-2-3",
                                "Attr": {
                                    "Name": "Test",
                                    "Age": 30
                                },
                                "List": [1, 2, 3]
                            }
                            "#,
                        )
                        .unwrap(),
                    ),
                    request_start: std::time::Instant::now(),
                    retry_count: 0,
                },
            )
            .unwrap();

        let response = ctx.get_variable("response").unwrap();
        let response = response.as_map().unwrap();

        // Verify Result field
        let result = response.get("Result").unwrap();
        assert_eq!(result, &Value::Int(0));

        // Verify ObjectId field
        let object_id = response.get("ObjectId").unwrap();
        assert_eq!(object_id, &Value::String("0-1-2-3".into()));

        // Verify Attr Name and Age field
        let attr = response.get("Attr").unwrap();
        let attr = attr.as_map().unwrap();
        let name = attr.get("Name").unwrap();
        assert_eq!(name, &Value::String("Test".into()));
        let age = attr.get("Age").unwrap();
        assert_eq!(age, &Value::Int(30));

        // Verify List
        let list = response.get("List").unwrap();
        let list = list.as_list().unwrap();
        println!("{:?}", list);
        assert_eq!(
            list,
            &Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_request_from_response_extract_header() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));

        let request = Request {
            name: "Request_2".into(),
            base_url: "http://localhost:8080".into(),
            method: Method::GET,
            headers: None,
            uri: "/endpoint".into(),
            uri_var_name: vec![],
            body: None,
            body_var_name: vec![],
            timeout: Duration::from_secs(3),
            before: None,
            after: None,
        };

        let mut ctx = ScriptContext::new(global);

        request
            .from_response(
                &mut ctx,
                &HttpResponse {
                    status: StatusCode::OK,
                    headers: {
                        let mut map = http::HeaderMap::new();
                        map.append("Content-Type", "application/json".parse().unwrap());
                        map.append("Location", "https://localhost:8080/foo1".parse().unwrap());
                        map.append("Location", "https://localhost:8080/foo2".parse().unwrap());
                        map
                    },
                    body: None,
                    request_start: std::time::Instant::now(),
                    retry_count: 0,
                },
            )
            .unwrap();

        let response_headers = ctx
            .get_variable("responseHeaders")
            .unwrap()
            .as_map()
            .unwrap();

        let content_type = response_headers.get("content-type").unwrap();
        assert_eq!(
            content_type,
            &Value::List(vec![Value::String("application/json".into())])
        );

        let locations = response_headers.get("location").unwrap();
        assert_eq!(
            locations,
            &Value::List(vec![
                Value::String("https://localhost:8080/foo1".into()),
                Value::String("https://localhost:8080/foo2".into())
            ])
        );
    }
}
