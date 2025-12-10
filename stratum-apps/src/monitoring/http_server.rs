//! HTTP server for exposing monitoring data using Axum

use super::{
    client::{ClientsMonitoring, ClientsSummary},
    prometheus_metrics::PrometheusMetrics,
    server::{ServerMonitoring, ServerSummary},
    sv1::Sv1ClientsMonitoring,
    GlobalInfo,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};
use serde::Deserialize;
use std::{
    future::Future,
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tracing::info;

/// Shared state for all HTTP handlers
#[derive(Clone)]
struct ServerState {
    server_monitoring: Option<Arc<dyn ServerMonitoring + Send + Sync + 'static>>,
    clients_monitoring: Option<Arc<dyn ClientsMonitoring + Send + Sync + 'static>>,
    sv1_monitoring: Option<Arc<dyn Sv1ClientsMonitoring + Send + Sync + 'static>>,
    start_time: u64,
    metrics: PrometheusMetrics,
}

const DEFAULT_LIMIT: usize = 25;
const MAX_LIMIT: usize = 100;

#[derive(Deserialize)]
struct Pagination {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: Option<usize>,
}

impl Pagination {
    fn effective_limit(&self) -> usize {
        self.limit
            .map(|l| l.min(MAX_LIMIT))
            .unwrap_or(DEFAULT_LIMIT)
    }
}

fn paginate<T: Clone>(items: &[T], params: &Pagination) -> (usize, Vec<T>) {
    let total = items.len();
    let limit = params.effective_limit();
    let offset = params.offset.min(total);
    let sliced = items
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    (total, sliced)
}

/// HTTP server that exposes monitoring data as JSON
pub struct MonitoringServer {
    bind_address: SocketAddr,
    state: ServerState,
}

impl MonitoringServer {
    /// Create a new monitoring server
    ///
    /// Returns a server that exposes monitoring data via HTTP JSON API. Chain with
    /// `with_sv1_monitoring()` for SV1 support, then call `run()` to start.
    pub fn new(
        bind_address: SocketAddr,
        server_monitoring: Option<Arc<dyn ServerMonitoring + Send + Sync + 'static>>,
        clients_monitoring: Option<Arc<dyn ClientsMonitoring + Send + Sync + 'static>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Only register metrics for available monitoring types
        let metrics = PrometheusMetrics::new(
            server_monitoring.is_some(),
            clients_monitoring.is_some(),
            false, // SV1 metrics added later via with_sv1_monitoring
        )?;

        Ok(Self {
            bind_address,
            state: ServerState {
                server_monitoring,
                clients_monitoring,
                sv1_monitoring: None,
                start_time,
                metrics,
            },
        })
    }

    /// Add SV1 client monitoring (optional, for Translator Proxy only)
    pub fn with_sv1_monitoring(
        mut self,
        sv1_monitoring: Arc<dyn Sv1ClientsMonitoring + Send + Sync + 'static>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        self.state.sv1_monitoring = Some(sv1_monitoring);

        // Re-create metrics with SV1 enabled
        self.state.metrics = PrometheusMetrics::new(
            self.state.server_monitoring.is_some(),
            self.state.clients_monitoring.is_some(),
            true, // Enable SV1 metrics
        )?;

        Ok(self)
    }

    /// Run the monitoring server until the shutdown signal completes
    ///
    /// Starts an HTTP server that exposes monitoring data as JSON.
    /// The server shuts down gracefully when `shutdown_signal` completes.
    ///
    /// Automatically exposes Prometheus metrics at `/metrics` including:
    /// - SV2 channel metrics (counts, hashrates, shares)
    /// - SV1 client metrics (for Translator)
    /// - HTTP request metrics (optional, via custom middleware)
    pub async fn run(
        self,
        shutdown_signal: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting monitoring server on {}", self.bind_address);

        // Versioned JSON API under /api/v1 plus a Prometheus /metrics endpoint.
        let api_v1 = Router::new()
            .route("/", get(handle_root))
            .route("/health", get(handle_health))
            .route("/global", get(handle_global))
            // Server endpoint (upstream connection)
            .route("/server", get(handle_server))
            // Clients endpoints (downstream connections)
            .route("/clients", get(handle_clients))
            .route("/clients/{client_id}", get(handle_client_by_id))
            // SV1 clients (Translator only)
            .route("/sv1/clients", get(handle_sv1_clients));

        let app = Router::new()
            .nest("/api/v1", api_v1)
            // Prometheus exporters conventionally expose /metrics at the root.
            .route("/metrics", get(handle_prometheus_metrics))
            .with_state(self.state);

        let listener = TcpListener::bind(self.bind_address).await?;

        info!(
            "Prometheus metrics available at http://{}/metrics",
            self.bind_address
        );

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_signal.await;
                info!("Monitoring server received shutdown signal, stopping...");
            })
            .await?;

        info!("Monitoring server stopped");
        Ok(())
    }
}

async fn handle_root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "SRI Monitoring API",
        "version": "0.1.0",
        "endpoints": {
            "/api/v1": "API root",
            "/api/v1/health": "Health check",
            "/api/v1/global": "Global statistics",
            "/api/v1/server": "Server (upstream) with channels (paginated)",
            "/api/v1/clients": "All clients (paginated)",
            "/api/v1/clients/{id}": "Single client with channels (paginated)",
            "/api/v1/sv1/clients": "SV1 clients (Translator Proxy only, paginated)",
            "/metrics": "Prometheus metrics"
        }
    }))
}

async fn handle_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

async fn handle_global(State(state): State<ServerState>) -> Json<GlobalInfo> {
    let uptime_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        - state.start_time;

    let clients = state
        .clients_monitoring
        .as_ref()
        .map(|m| m.get_clients_summary())
        .unwrap_or_else(|| ClientsSummary {
            total_clients: 0,
            total_channels: 0,
            extended_channels: 0,
            standard_channels: 0,
            total_hashrate: 0.0,
        });

    let server = state
        .server_monitoring
        .as_ref()
        .map(|m| m.get_server_summary())
        .unwrap_or_else(|| ServerSummary {
            total_channels: 0,
            extended_channels: 0,
            standard_channels: 0,
            total_hashrate: 0.0,
        });

    Json(GlobalInfo {
        server,
        clients,
        uptime_secs,
    })
}

async fn handle_server(
    Query(params): Query<Pagination>,
    State(state): State<ServerState>,
) -> Response {
    match &state.server_monitoring {
        Some(monitoring) => {
            let server = monitoring.get_server();

            let (total_extended, extended) = paginate(&server.extended_channels, &params);
            let (total_standard, standard) = paginate(&server.standard_channels, &params);

            Json(serde_json::json!({
                "offset": params.offset,
                "limit": params.effective_limit(),
                "total_extended": total_extended,
                "total_standard": total_standard,
                "extended_channels": extended,
                "standard_channels": standard,
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Server monitoring not available"
            })),
        )
            .into_response(),
    }
}

async fn handle_clients(
    Query(params): Query<Pagination>,
    State(state): State<ServerState>,
) -> Response {
    match &state.clients_monitoring {
        Some(monitoring) => {
            let clients = monitoring.get_clients();
            let (total, page) = paginate(&clients, &params);

            Json(serde_json::json!({
                "offset": params.offset,
                "limit": params.effective_limit(),
                "total": total,
                "items": page
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Clients monitoring not available"
            })),
        )
            .into_response(),
    }
}

async fn handle_client_by_id(
    Path(client_id): Path<usize>,
    Query(params): Query<Pagination>,
    State(state): State<ServerState>,
) -> Response {
    match &state.clients_monitoring {
        Some(monitoring) => {
            let clients = monitoring.get_clients();
            match clients.into_iter().find(|c| c.client_id == client_id) {
                Some(client) => {
                    let (total_extended, extended) = paginate(&client.extended_channels, &params);
                    let (total_standard, standard) = paginate(&client.standard_channels, &params);

                    Json(serde_json::json!({
                        "client_id": client_id,
                        "offset": params.offset,
                        "limit": params.effective_limit(),
                        "total_extended": total_extended,
                        "total_standard": total_standard,
                        "extended_channels": extended,
                        "standard_channels": standard,
                    }))
                    .into_response()
                }
                None => (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": format!("Client {} not found", client_id)
                    })),
                )
                    .into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Clients monitoring not available"
            })),
        )
            .into_response(),
    }
}

async fn handle_sv1_clients(
    Query(params): Query<Pagination>,
    State(state): State<ServerState>,
) -> Response {
    match &state.sv1_monitoring {
        Some(monitoring) => {
            let clients = monitoring.get_sv1_clients();
            let (total, page) = paginate(&clients, &params);

            Json(serde_json::json!({
                "offset": params.offset,
                "limit": params.effective_limit(),
                "total": total,
                "items": page
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "SV1 client monitoring not available"
            })),
        )
            .into_response(),
    }
}

/// Handler for Prometheus metrics endpoint
///
/// Collects fresh data from all monitoring sources and exports as Prometheus metrics
async fn handle_prometheus_metrics(State(state): State<ServerState>) -> Response {
    let uptime_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        - state.start_time;
    state.metrics.sv2_uptime_seconds.set(uptime_secs as f64);

    // Reset per-channel metrics before repopulating
    if let Some(ref metric) = state.metrics.sv2_client_channel_hashrate {
        metric.reset();
    }
    if let Some(ref metric) = state.metrics.sv2_client_shares_accepted_total {
        metric.reset();
    }
    if let Some(ref metric) = state.metrics.sv2_client_channel_shares_per_minute {
        metric.reset();
    }
    if let Some(ref metric) = state.metrics.sv2_server_channel_hashrate {
        metric.reset();
    }
    if let Some(ref metric) = state.metrics.sv2_server_shares_accepted_total {
        metric.reset();
    }

    // Collect server metrics
    if let Some(monitoring) = &state.server_monitoring {
        let summary = monitoring.get_server_summary();
        if let Some(ref metric) = state.metrics.sv2_server_channels_total {
            metric.set(summary.total_channels as f64);
        }
        if let Some(ref metric) = state.metrics.sv2_server_channels_extended {
            metric.set(summary.extended_channels as f64);
        }
        if let Some(ref metric) = state.metrics.sv2_server_channels_standard {
            metric.set(summary.standard_channels as f64);
        }
        if let Some(ref metric) = state.metrics.sv2_server_hashrate_total {
            metric.set(summary.total_hashrate as f64);
        }

        let server = monitoring.get_server();
        for channel in &server.extended_channels {
            let channel_id = channel.channel_id.to_string();
            let user = &channel.user_identity;

            if let Some(ref metric) = state.metrics.sv2_server_shares_accepted_total {
                metric
                    .with_label_values(&[&channel_id, user])
                    .set(channel.shares_accepted as f64);
            }
            if let Some(ref metric) = state.metrics.sv2_server_channel_hashrate {
                metric
                    .with_label_values(&[&channel_id, user])
                    .set(channel.nominal_hashrate as f64);
            }
        }

        for channel in &server.standard_channels {
            let channel_id = channel.channel_id.to_string();
            let user = &channel.user_identity;

            if let Some(ref metric) = state.metrics.sv2_server_shares_accepted_total {
                metric
                    .with_label_values(&[&channel_id, user])
                    .set(channel.shares_accepted as f64);
            }
            if let Some(ref metric) = state.metrics.sv2_server_channel_hashrate {
                metric
                    .with_label_values(&[&channel_id, user])
                    .set(channel.nominal_hashrate as f64);
            }
        }
    }

    // Collect clients metrics
    if let Some(monitoring) = &state.clients_monitoring {
        let summary = monitoring.get_clients_summary();
        if let Some(ref metric) = state.metrics.sv2_clients_total {
            metric.set(summary.total_clients as f64);
        }
        if let Some(ref metric) = state.metrics.sv2_client_channels_total {
            metric.set(summary.total_channels as f64);
        }
        if let Some(ref metric) = state.metrics.sv2_client_channels_extended {
            metric.set(summary.extended_channels as f64);
        }
        if let Some(ref metric) = state.metrics.sv2_client_channels_standard {
            metric.set(summary.standard_channels as f64);
        }
        if let Some(ref metric) = state.metrics.sv2_client_hashrate_total {
            metric.set(summary.total_hashrate as f64);
        }

        let clients = monitoring.get_clients();
        for client in &clients {
            let client_id = client.client_id.to_string();

            for channel in &client.extended_channels {
                let channel_id = channel.channel_id.to_string();
                let user = &channel.user_identity;

                if let Some(ref metric) = state.metrics.sv2_client_shares_accepted_total {
                    metric
                        .with_label_values(&[&client_id, &channel_id, user])
                        .set(channel.shares_accepted as f64);
                }
                if let Some(ref metric) = state.metrics.sv2_client_channel_hashrate {
                    metric
                        .with_label_values(&[&client_id, &channel_id, user])
                        .set(channel.nominal_hashrate as f64);
                }
                if let Some(ref metric) = state.metrics.sv2_client_channel_shares_per_minute {
                    metric
                        .with_label_values(&[&client_id, &channel_id, user])
                        .set(channel.shares_per_minute as f64);
                }
            }

            for channel in &client.standard_channels {
                let channel_id = channel.channel_id.to_string();
                let user = &channel.user_identity;

                if let Some(ref metric) = state.metrics.sv2_client_shares_accepted_total {
                    metric
                        .with_label_values(&[&client_id, &channel_id, user])
                        .set(channel.shares_accepted as f64);
                }
                if let Some(ref metric) = state.metrics.sv2_client_channel_hashrate {
                    metric
                        .with_label_values(&[&client_id, &channel_id, user])
                        .set(channel.nominal_hashrate as f64);
                }
                if let Some(ref metric) = state.metrics.sv2_client_channel_shares_per_minute {
                    metric
                        .with_label_values(&[&client_id, &channel_id, user])
                        .set(channel.shares_per_minute as f64);
                }
            }
        }
    }

    // Collect SV1 client metrics
    if let Some(monitoring) = &state.sv1_monitoring {
        let summary = monitoring.get_sv1_clients_summary();
        if let Some(ref metric) = state.metrics.sv1_clients_total {
            metric.set(summary.total_clients as f64);
        }
        if let Some(ref metric) = state.metrics.sv1_hashrate_total {
            metric.set(summary.total_hashrate as f64);
        }
    }

    // Encode and return metrics
    let encoder = TextEncoder::new();
    let metric_families = state.metrics.registry.gather();
    let mut buffer = Vec::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => match String::from_utf8(buffer) {
            Ok(metrics_text) => (StatusCode::OK, metrics_text).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("UTF-8 error: {}", e)})),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Encoding error: {}", e)})),
        )
            .into_response(),
    }
}
