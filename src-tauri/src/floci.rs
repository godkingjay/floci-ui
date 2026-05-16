use reqwest::{Client, StatusCode};
use serde_json::Value;
use url::Url;

use crate::models::HealthSnapshot;

#[derive(Clone)]
pub struct FlociClient {
    endpoint_url: Url,
    http: Client,
}

impl FlociClient {
    pub fn new(endpoint_url: Url, http: Client) -> Self {
        Self { endpoint_url, http }
    }

    pub async fn health(&self) -> HealthSnapshot {
        let health_url = self.endpoint_path("/_floci/health");
        match self.http.get(health_url.clone()).send().await {
            Ok(response) => read_health_response(health_url, response).await,
            Err(err) => HealthSnapshot::unavailable(&health_url, err.to_string()),
        }
    }

    fn endpoint_path(&self, path: &str) -> Url {
        let mut url = self.endpoint_url.clone();
        url.set_path(path.trim_start_matches('/'));
        url.set_query(None);
        url.set_fragment(None);
        url
    }
}

async fn read_health_response(url: Url, response: reqwest::Response) -> HealthSnapshot {
    let status = response.status();
    let url = url.to_string();

    match response.json::<Value>().await {
        Ok(body) => {
            let ok = status.is_success();
            let (floci_version, health_status) = health_metadata(&body, ok);

            HealthSnapshot {
                ok,
                url,
                status: Some(status.as_u16()),
                floci_version,
                health_status,
                body: Some(body),
                error: None,
            }
        }
        Err(err) => HealthSnapshot {
            ok: false,
            url,
            status: Some(status.as_u16()),
            floci_version: None,
            health_status: "invalid-json".to_owned(),
            body: None,
            error: Some(format!("health response was not JSON: {err}")),
        },
    }
}

fn health_metadata(body: &Value, ok: bool) -> (Option<String>, String) {
    let floci_version = string_field(body, &["floci_version", "version"]).or_else(|| {
        body.pointer("/floci/version")
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let health_status = string_field(body, &["health_status", "status", "state"])
        .or_else(|| {
            body.pointer("/floci/status")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| {
            if ok {
                "healthy".to_owned()
            } else {
                "unavailable".to_owned()
            }
        });

    (floci_version, health_status)
}

fn string_field(body: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| body.get(*key).and_then(Value::as_str).map(str::to_owned))
}

impl HealthSnapshot {
    fn unavailable(url: &Url, error: String) -> Self {
        Self {
            ok: false,
            url: url.to_string(),
            status: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
            floci_version: None,
            health_status: "unavailable".to_owned(),
            body: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use httpmock::{Method::GET, MockServer};
    use reqwest::Client;
    use serde_json::json;

    use super::FlociClient;

    #[tokio::test]
    async fn health_reads_floci_health_endpoint() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/_floci/health");
                then.status(200).json_body(json!({
                    "status": "running",
                    "version": "test"
                }));
            })
            .await;

        let client = FlociClient::new(
            server.base_url().parse().expect("mock URL should parse"),
            Client::new(),
        );

        let health = client.health().await;

        assert!(health.ok);
        assert_eq!(health.status, Some(200));
        assert_eq!(health.floci_version.as_deref(), Some("test"));
        assert_eq!(health.health_status, "running");
    }
}
