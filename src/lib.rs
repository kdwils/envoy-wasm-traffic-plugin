use ipnet::IpNet;
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
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
    trusted_proxies: Vec<IpNet>,
}

#[derive(Deserialize)]
struct PluginConfigYaml {
    webhook_cluster: String,
    webhook_authority: String,
    webhook_path: String,
    #[serde(default)]
    headers: Vec<String>,
    #[serde(default)]
    trusted_proxies: Vec<String>,
}

impl Context for TrafficPluginRoot {}

impl RootContext for TrafficPluginRoot {
    fn on_configure(&mut self, _: usize) -> bool {
        let Some(config_bytes) = self.get_plugin_configuration() else {
            return false;
        };

        let Ok(config) = serde_json::from_slice::<PluginConfigYaml>(&config_bytes) else {
            return false;
        };

        self.config.webhook_cluster = config.webhook_cluster;
        self.config.webhook_authority = config.webhook_authority;
        self.config.webhook_path = config.webhook_path;
        self.config.headers = config.headers;

        self.config.trusted_proxies = config
            .trusted_proxies
            .iter()
            .filter_map(|cidr| cidr.parse::<IpNet>().ok().or_else(|| None))
            .collect();

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
    client_ip: String,
    authority: String,
    headers: HashMap<String, String>,
}

impl Context for TrafficPluginHttp {
    fn on_http_call_response(
        &mut self,
        _token_id: u32,
        _num_headers: usize,
        _body_size: usize,
        _num_trailers: usize,
    ) {
    }
}

impl TrafficPluginHttp {
    fn extract_real_ip(&self) -> String {
        let peer_ip = self
            .get_property(vec!["source", "address"])
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|addr| {
                if let Some(stripped) = addr.strip_prefix('[') {
                    return stripped.split(']').next().map(|s| s.to_string());
                }
                addr.split(':').next().map(|s| s.to_string())
            });

        let Some(peer_ip_str) = peer_ip else {
            return "unknown".to_string();
        };

        let Ok(peer_addr) = peer_ip_str.parse::<IpAddr>() else {
            return peer_ip_str;
        };

        let is_trusted = self
            .config
            .trusted_proxies
            .iter()
            .any(|net| net.contains(&peer_addr));

        if !is_trusted {
            return peer_ip_str;
        }

        let Some(xff) = self.get_http_request_header("x-forwarded-for") else {
            return peer_ip_str;
        };

        let ips: Vec<&str> = xff.split(',').map(|s| s.trim()).collect();
        for ip_str in ips.iter().rev() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                let is_trusted_xff = self
                    .config
                    .trusted_proxies
                    .iter()
                    .any(|net| net.contains(&ip));

                if !is_trusted_xff {
                    return ip_str.to_string();
                }
            }
        }

        ips.first().map(|s| s.to_string()).unwrap_or(peer_ip_str)
    }
}

impl HttpContext for TrafficPluginHttp {
    fn on_http_request_headers(&mut self, _: usize, _: bool) -> Action {
        if self.config.webhook_cluster.is_empty() {
            return Action::Continue;
        }

        if self.config.webhook_path.is_empty() {
            return Action::Continue;
        }

        let client_ip = self.extract_real_ip();

        let authority = self
            .get_http_request_header(":authority")
            .unwrap_or_else(|| "unknown".to_string());

        let mut headers = HashMap::new();

        for header_name in &self.config.headers {
            if let Some(value) = self.get_http_request_header(header_name) {
                headers.insert(header_name.clone(), value);
            }
        }

        let payload = EventPayload {
            client_ip,
            authority,
            headers,
        };

        let Ok(body) = serde_json::to_vec(&payload) else {
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
            Ok(_) => {}
            Err(_) => {}
        }

        Action::Continue
    }
}
