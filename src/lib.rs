use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(TrafficPluginRoot {
            config: PluginConfig::default(),
        })
    });
}}

struct TrafficPluginRoot {
    config: PluginConfig,
}

#[derive(Default, Clone)]
struct PluginConfig {
    webhook_cluster: String,
    webhook_authority: String,
    webhook_path: String,
    headers: Vec<String>,
}

#[derive(Deserialize)]
struct PluginConfigYaml {
    webhook_cluster: String,
    webhook_authority: String,
    webhook_path: String,
    headers: Vec<String>,
}

impl Context for TrafficPluginRoot {}

impl RootContext for TrafficPluginRoot {
    fn on_configure(&mut self, _: usize) -> bool {
        let Some(config_bytes) = self.get_plugin_configuration() else {
            log::error!("No configuration provided");
            return false;
        };

        let Ok(config) = serde_json::from_slice::<PluginConfigYaml>(&config_bytes) else {
            log::error!("Failed to parse configuration");
            return false;
        };

        self.config.webhook_cluster = config.webhook_cluster;
        self.config.webhook_authority = config.webhook_authority;
        self.config.webhook_path = config.webhook_path;
        self.config.headers = config.headers;
        log::info!(
            "Configured webhook_cluster: {}, webhook_authority: {}, webhook_path: {}, headers: {:?}",
            self.config.webhook_cluster,
            self.config.webhook_authority,
            self.config.webhook_path,
            self.config.headers
        );
        true
    }

    fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(TrafficPluginHttp {
            config: self.config.clone(),
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}

struct TrafficPluginHttp {
    config: PluginConfig,
}

use std::collections::HashMap;

#[derive(Serialize)]
struct EventPayload {
    headers: HashMap<String, String>,
}

impl Context for TrafficPluginHttp {
    fn on_http_call_response(
        &mut self,
        _token_id: u32,
        _num_headers: usize,
        body_size: usize,
        _num_trailers: usize,
    ) {
        log::info!("Received webhook response, body size: {}", body_size);
    }
}

impl HttpContext for TrafficPluginHttp {
    fn on_http_request_headers(&mut self, _: usize, _: bool) -> Action {
        if self.config.webhook_cluster.is_empty() {
            log::warn!("Webhook cluster not configured");
            return Action::Continue;
        }

        if self.config.webhook_path.is_empty() {
            log::warn!("Webhook path not configured");
            return Action::Continue;
        }

        if self.config.headers.is_empty() {
            log::warn!("No headers configured");
            return Action::Continue;
        }

        let mut headers = HashMap::new();

        for header_name in &self.config.headers {
            if let Some(value) = self.get_http_request_header(header_name) {
                headers.insert(header_name.clone(), value);
            }
        }

        let payload = EventPayload { headers };

        let Ok(body) = serde_json::to_vec(&payload) else {
            log::error!("Failed to serialize payload");
            return Action::Continue;
        };

        let http_headers = vec![
            (":method", "POST"),
            (":path", self.config.webhook_path.as_str()),
            (":authority", self.config.webhook_authority.as_str()),
            ("content-type", "application/json"),
        ];

        match self.dispatch_http_call(
            &self.config.webhook_cluster,
            http_headers,
            Some(&body),
            vec![],
            Duration::from_secs(5),
        ) {
            Ok(call_id) => {
                log::info!("Dispatched HTTP call with id: {}", call_id);
            }
            Err(e) => {
                log::error!("Failed to dispatch HTTP call: {:?}", e);
            }
        }

        Action::Continue
    }
}
