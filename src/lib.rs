mod merge;

use std::str;
use std::time::Duration;

use log::{error, trace};

use proxy_wasm::hostcalls;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, ContextType, LogLevel, MetricType, Status};

use serde::Deserialize;

use merge::Merge;

/// `Configuration` represents the possible filter configuration values.
#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Configuration {
    upstream: String,
    endpoint: String,
    authority: Option<String>,
    timeout: Option<u64>,
}

impl Default for Configuration {
    fn default() -> Configuration {
        Configuration {
            upstream: "auth".to_string(),
            endpoint: "/authenticate".to_string(),
            authority: None,
            timeout: Some(10),
        }
    }
}

impl Merge for Configuration {
    fn merge(&self, other: Configuration) -> Configuration {
        Configuration {
            upstream: other.upstream,
            endpoint: other.endpoint,
            authority: self.authority.merge(other.authority),
            timeout: self.timeout.merge(other.timeout),
        }
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(TokenAuthnRoot {
            configuration: Configuration::default(),
            success_metric: hostcalls::define_metric(
                MetricType::Counter,
                "token_authn_filter_authentication_success_count",
            ).unwrap(),
            failure_metric: hostcalls::define_metric(
                MetricType::Counter,
                "token_authn_filter_authentication_failure_count",
            ).unwrap(),
        })
    });
}

struct TokenAuthnRoot {
    configuration: Configuration,
    success_metric: u32,
    failure_metric: u32,

}

impl Context for TokenAuthnRoot {}

impl RootContext for TokenAuthnRoot {
    fn on_configure(&mut self, _: usize) -> bool {
        if let Some(configuration) = self.get_configuration() {
            match serde_json::from_slice(&configuration) {
                Ok(configuration) => {
                    self.configuration = self.configuration.merge(configuration);
                    return true
                },
                Err(err) => error!("failed to configure filter: {}", err),
            }
        }
        false
    }

    fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(TokenAuthn {
            context_id,
            configuration: self.configuration.clone(),
            success_metric: self.success_metric,
            failure_metric: self.failure_metric,
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}

struct TokenAuthn {
    context_id: u32,
    configuration: Configuration,
    success_metric: u32,
    failure_metric: u32,
}

impl Context for TokenAuthn {
    fn on_http_call_response(&mut self, _: u32, _: usize, body_size: usize, _: usize) {
        if let Some(body) = self.get_http_call_response_body(0, body_size) {
            increment_metric(self.success_metric);

            self.set_http_request_header("authorization", Some(str::from_utf8(&body).unwrap()));
            self.resume_http_request();
            return;
        }

        increment_metric(self.failure_metric);

        trace!("#{} unauthorized.", self.context_id);
        self.send_http_response(401, vec![], None);
    }
}

impl HttpContext for TokenAuthn {
    fn on_http_request_headers(&mut self, _: usize) -> Action {
        if let Some(token) = self.get_http_request_header("authorization") {
            return match self.authenticate(&token) {
                Ok(_) => Action::Pause,
                Err(status) => {
                    trace!("#{} internal error: #{:?}", self.context_id, status);
                    Action::Continue
                }
            };
        }

        increment_metric(self.failure_metric);

        trace!("#{} header missing. ignoring.", self.context_id);
        Action::Continue
    }
}

impl TokenAuthn {
    fn authenticate(&self, token: &str) -> Result<u32, Status> {
        let authority = &self.configuration.authority.as_ref().unwrap(); // This should be safe.
        let timeout = self.configuration.timeout.unwrap(); // This should be safe.

        self.dispatch_http_call(
            &self.configuration.upstream,
            vec![
                (":method", "POST"),
                (":path", &self.configuration.endpoint),
                (":authority", &authority),
                ("authorization", token),
            ],
            None,
            vec![],
            Duration::from_secs(timeout),
        )
    }
}

fn increment_metric(metric_id: u32) {
    hostcalls::increment_metric(metric_id, 1).unwrap();
}

#[cfg(test)]
mod tests {
    use super::{Configuration, Merge};

    #[test]
    fn merges_default_values_with_overrides() {
        assert_eq!(
            Configuration::default().merge(Configuration {
                upstream: "enterprise".to_string(),
                endpoint: "/ready-room".to_string(),
                authority: Some("starfleet".to_string()),
                timeout: Some(5),
            }),
            Configuration {
                upstream: "enterprise".to_string(),
                endpoint: "/ready-room".to_string(),
                authority: Some("starfleet".to_string()),
                timeout: Some(5),
            },
        )
    }

    #[test]
    fn merges_configuration_values_with_overrides() {
        assert_eq!(
            Configuration{
                upstream: "".to_string(),
                endpoint: "/ready-room".to_string(),
                authority: None,
                timeout: Some(10),
            }.merge(Configuration {
                upstream: "enterprise".to_string(),
                endpoint: "".to_string(),
                authority: Some("starfleet".to_string()),
                timeout: None,
            }),
            Configuration {
                upstream: "enterprise".to_string(),
                endpoint: "".to_string(),
                authority: Some("starfleet".to_string()),
                timeout: Some(10),
            },
        )
    }
}
