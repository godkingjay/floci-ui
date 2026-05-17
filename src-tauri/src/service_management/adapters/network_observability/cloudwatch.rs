use std::collections::BTreeMap;

use aws_sdk_cloudwatch::{
    primitives::DateTime,
    types::{ComparisonOperator, Dimension, Metric, MetricDataQuery, MetricStat, Statistic},
};
use time::OffsetDateTime;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::network_observability::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_i32, require_payload_text, require_resource_id, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

const METRIC_SEPARATOR: &str = "||";

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let metrics =
        client.list_metrics().send().await.map_err(|err| {
            ServiceManagementError::client_error("cloudwatch", "list_metrics", err)
        })?;
    let mut resources = Vec::new();

    for metric in metrics.metrics() {
        let Some(namespace) = metric.namespace() else {
            continue;
        };
        let Some(metric_name) = metric.metric_name() else {
            continue;
        };
        let dimensions = dimension_key(metric.dimensions());
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "namespace", Some(namespace));
        insert_attr(
            &mut attributes,
            "dimension_count",
            Some(metric.dimensions().len()),
        );
        if !dimensions.is_empty() {
            insert_attr(&mut attributes, "dimensions", Some(dimensions.clone()));
        }

        resources.push(ResourceSummary {
            id: format!(
                "metric/{namespace}{METRIC_SEPARATOR}{metric_name}{METRIC_SEPARATOR}{dimensions}"
            ),
            name: metric_name.to_owned(),
            kind: "metric".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    load_metric_alarms(&client, &mut resources).await;
    load_dashboards(&client, &mut resources).await;

    Ok(managed_inventory(
        "cloudwatch",
        "CloudWatch",
        tabs(),
        resources,
        vec!["alarm_history_requires_explicit_query".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "query_metric_data" => {
            let (namespace, metric_name, dimensions) = selected_metric(request)?;
            let period = optional_payload_i32(request, "period")
                .unwrap_or(60)
                .clamp(1, 86_400);
            let window_seconds = optional_payload_i32(request, "window_seconds")
                .unwrap_or(3_600)
                .clamp(period, 604_800);
            let now = OffsetDateTime::now_utc().unix_timestamp();
            let output = client(config)
                .get_metric_data()
                .start_time(DateTime::from_secs(now - i64::from(window_seconds)))
                .end_time(DateTime::from_secs(now))
                .metric_data_queries(metric_data_query(
                    namespace,
                    metric_name,
                    dimensions,
                    period,
                ))
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cloudwatch", "query_metric_data", err)
                })?;
            let result = output.metric_data_results().first();
            let datapoints = result.map_or(0, |result| result.values().len());
            let latest = result
                .and_then(|result| result.values().first())
                .map_or_else(
                    || "no datapoints returned".to_owned(),
                    |value| format!("{value:.4}"),
                );

            Ok(ActionResult {
                changed: false,
                message: format!(
                    "Queried `{metric_name}` over {window_seconds} second(s): {datapoints} datapoint(s), latest {latest}."
                ),
                resource_id: request.resource_id.clone(),
            })
        }
        "create_alarm" => {
            let (namespace, metric_name, dimensions) = selected_metric(request)?;
            let alarm_name = require_name_payload(request, "alarm_name")?;
            let threshold = require_payload_f64(request, "threshold")?;
            let period = optional_payload_i32(request, "period")
                .unwrap_or(60)
                .clamp(1, 86_400);
            let evaluation_periods = optional_payload_i32(request, "evaluation_periods")
                .unwrap_or(1)
                .clamp(1, 100);
            let mut builder = client(config)
                .put_metric_alarm()
                .alarm_name(&alarm_name)
                .alarm_description(format!(
                    "Managed by Floci UI for {namespace}/{metric_name}."
                ))
                .namespace(namespace)
                .metric_name(metric_name)
                .statistic(Statistic::Average)
                .period(period)
                .evaluation_periods(evaluation_periods)
                .threshold(threshold)
                .comparison_operator(ComparisonOperator::GreaterThanThreshold);

            for dimension in dimensions {
                builder = builder.dimensions(dimension);
            }

            builder.send().await.map_err(|err| {
                ServiceManagementError::client_error("cloudwatch", "create_alarm", err)
            })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created CloudWatch alarm `{alarm_name}`."),
                resource_id: Some(format!("alarm/{alarm_name}")),
            })
        }
        "delete_alarm" => {
            let alarm_name = require_resource_id(request, "alarm/")?;
            require_typed_confirmation(request)?;
            client(config)
                .delete_alarms()
                .alarm_names(alarm_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cloudwatch", "delete_alarm", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted CloudWatch alarm `{alarm_name}`."),
                resource_id: Some(format!("alarm/{alarm_name}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

fn client(config: &AppConfig) -> aws_sdk_cloudwatch::Client {
    let sdk_config = aws_sdk_cloudwatch::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_cloudwatch::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_cloudwatch::Client::from_conf(sdk_config)
}

async fn load_metric_alarms(
    client: &aws_sdk_cloudwatch::Client,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.describe_alarms().send().await else {
        return;
    };

    for alarm in output.metric_alarms() {
        let Some(alarm_name) = alarm.alarm_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "metric_name", alarm.metric_name());
        insert_attr(&mut attributes, "namespace", alarm.namespace());
        insert_attr(&mut attributes, "threshold", alarm.threshold());
        insert_attr(
            &mut attributes,
            "comparison_operator",
            alarm
                .comparison_operator()
                .map(|operator| operator.as_str()),
        );
        insert_attr(
            &mut attributes,
            "dimension_count",
            Some(alarm.dimensions().len()),
        );
        if let Some(reason) = alarm.state_reason() {
            insert_attr(
                &mut attributes,
                "state_reason",
                Some(trim_preview(reason, 160)),
            );
        }

        resources.push(ResourceSummary {
            id: format!("alarm/{alarm_name}"),
            name: alarm_name.to_owned(),
            kind: "alarm".to_owned(),
            status: alarm
                .state_value()
                .map_or_else(|| "unknown".to_owned(), |state| state.as_str().to_owned()),
            created_at: format_timestamp(alarm.alarm_configuration_updated_timestamp()),
            updated_at: format_timestamp(alarm.state_updated_timestamp()),
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_dashboards(
    client: &aws_sdk_cloudwatch::Client,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.list_dashboards().send().await else {
        return;
    };

    for dashboard in output.dashboard_entries() {
        let Some(dashboard_name) = dashboard.dashboard_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "dashboard_arn", dashboard.dashboard_arn());
        insert_attr(&mut attributes, "size_bytes", dashboard.size());

        resources.push(ResourceSummary {
            id: format!("dashboard/{dashboard_name}"),
            name: dashboard_name.to_owned(),
            kind: "dashboard".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: format_timestamp(dashboard.last_modified()),
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

fn metric_data_query(
    namespace: &str,
    metric_name: &str,
    dimensions: Vec<Dimension>,
    period: i32,
) -> MetricDataQuery {
    let mut metric = Metric::builder()
        .namespace(namespace)
        .metric_name(metric_name);
    for dimension in dimensions {
        metric = metric.dimensions(dimension);
    }
    let metric_stat = MetricStat::builder()
        .metric(metric.build())
        .period(period)
        .stat("Average")
        .build();

    MetricDataQuery::builder()
        .id("metric_1")
        .metric_stat(metric_stat)
        .return_data(true)
        .build()
}

fn selected_metric(
    request: &ServiceActionRequest,
) -> Result<(&str, &str, Vec<Dimension>), ServiceManagementError> {
    let selected = require_resource_id(request, "metric/")?;
    let mut parts = selected.splitn(3, METRIC_SEPARATOR);
    let namespace = parts.next().unwrap_or_default();
    let metric_name = parts.next().unwrap_or_default();
    let dimensions = parts.next().unwrap_or_default();

    if namespace.is_empty() || metric_name.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            "Select a CloudWatch metric before running this action.",
        ));
    }

    Ok((namespace, metric_name, parse_dimensions(dimensions)))
}

fn dimension_key(dimensions: &[Dimension]) -> String {
    dimensions
        .iter()
        .filter_map(|dimension| Some((dimension.name()?, dimension.value()?)))
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_dimensions(value: &str) -> Vec<Dimension> {
    value
        .split(',')
        .filter_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            if name.trim().is_empty() || value.trim().is_empty() {
                return None;
            }

            Some(
                Dimension::builder()
                    .name(name.trim())
                    .value(value.trim())
                    .build(),
            )
        })
        .collect()
}

fn require_payload_f64(
    request: &ServiceActionRequest,
    field_name: &str,
) -> Result<f64, ServiceManagementError> {
    let value = require_payload_text(request, field_name)?;
    value.parse::<f64>().map_err(|err| {
        ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            format!("`{field_name}` must be a number: {err}"),
        )
    })
}

fn trim_preview(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }

    let mut preview = value.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "metrics".to_owned(),
            label: "Metrics".to_owned(),
            kinds: vec!["metric".to_owned()],
            empty_message: "No CloudWatch metrics found.".to_owned(),
        },
        ResourceTab {
            key: "alarms".to_owned(),
            label: "Alarms".to_owned(),
            kinds: vec!["alarm".to_owned()],
            empty_message: "No CloudWatch alarms found.".to_owned(),
        },
        ResourceTab {
            key: "dashboards".to_owned(),
            label: "Dashboards".to_owned(),
            kinds: vec!["dashboard".to_owned()],
            empty_message: "No CloudWatch dashboards found.".to_owned(),
        },
        ResourceTab {
            key: "queries".to_owned(),
            label: "Queries".to_owned(),
            kinds: vec!["query-result".to_owned()],
            empty_message: "Metric query results are loaded through explicit actions.".to_owned(),
        },
    ]
}
