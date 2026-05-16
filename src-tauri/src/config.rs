use std::env;

use thiserror::Error;
use url::{Host, Url};

const DEFAULT_ENDPOINT_URL: &str = "http://localhost:4566";
const DEFAULT_REGION: &str = "us-east-1";
const DEFAULT_ACCESS_KEY_ID: &str = "test";
const DEFAULT_SECRET_ACCESS_KEY: &str = "test";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub endpoint_url: Url,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let endpoint_url = Url::parse(&env_value("FLOCI_AWS_ENDPOINT_URL", DEFAULT_ENDPOINT_URL))
            .map_err(ConfigError::InvalidEndpointUrl)?;

        validate_local_endpoint(&endpoint_url)?;

        Ok(Self {
            endpoint_url,
            region: env_value("FLOCI_AWS_REGION", DEFAULT_REGION),
            access_key_id: env_value("AWS_ACCESS_KEY_ID", DEFAULT_ACCESS_KEY_ID),
            secret_access_key: env_value("AWS_SECRET_ACCESS_KEY", DEFAULT_SECRET_ACCESS_KEY),
        })
    }

    pub fn credentials_status(&self) -> &'static str {
        if self.access_key_id.is_empty() || self.secret_access_key.is_empty() {
            "Missing"
        } else {
            "Configured"
        }
    }
}

fn env_value(name: &str, fallback: &str) -> String {
    env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

fn validate_local_endpoint(endpoint_url: &Url) -> Result<(), ConfigError> {
    match endpoint_url.host() {
        Some(Host::Domain(host)) if is_allowed_domain(host) => Ok(()),
        Some(Host::Ipv4(addr)) if addr.is_loopback() => Ok(()),
        Some(Host::Ipv6(addr)) if addr.is_loopback() => Ok(()),
        Some(host) => Err(ConfigError::RemoteEndpoint(host.to_string())),
        None => Err(ConfigError::MissingEndpointHost),
    }
}

fn is_allowed_domain(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host.eq_ignore_ascii_case("floci")
        || host.eq_ignore_ascii_case("host.docker.internal")
        || host.eq_ignore_ascii_case("localhost.floci.io")
        || host.eq_ignore_ascii_case("localhost.localstack.cloud")
        || host.ends_with(".localhost.floci.io")
        || host.ends_with(".localhost.localstack.cloud")
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("FLOCI_AWS_ENDPOINT_URL is invalid")]
    InvalidEndpointUrl(#[source] url::ParseError),
    #[error("FLOCI_AWS_ENDPOINT_URL must include a host")]
    MissingEndpointHost,
    #[error("refusing to connect to non-local Floci endpoint: {0}")]
    RemoteEndpoint(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_local_floci_endpoint_aliases() {
        let endpoints = [
            "http://localhost:4566",
            "http://127.0.0.1:4566",
            "http://[::1]:4566",
            "http://floci:4566",
            "http://host.docker.internal:4566",
            "http://localhost.floci.io:4566",
            "http://s3.localhost.floci.io:4566",
            "http://s3.localhost.localstack.cloud:4566",
        ];

        for endpoint in endpoints {
            let url = Url::parse(endpoint).expect("test URL should parse");
            assert!(
                validate_local_endpoint(&url).is_ok(),
                "{endpoint} should be accepted"
            );
        }
    }

    #[test]
    fn rejects_remote_endpoint() {
        let url = Url::parse("https://example.com").expect("test URL should parse");

        assert!(matches!(
            validate_local_endpoint(&url),
            Err(ConfigError::RemoteEndpoint(host)) if host == "example.com"
        ));
    }
}
