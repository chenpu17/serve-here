use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;
use serde::{Deserialize, Serialize};
use sysinfo::System;

const MAX_RECENT_REQUESTS: usize = 1000;
const MAX_RESOURCE_HISTORY: usize = 60;
const MAX_TRACKED_ROUTE_KEYS: usize = 2048;
const MAX_TRACKED_CLIENT_KEYS: usize = 1024;

#[derive(Clone)]
pub struct MetricsStore {
    inner: Arc<Mutex<RequestMetrics>>,
    resources: Arc<Mutex<ResourceMonitor>>,
    started_at: Instant,
    started_at_system: SystemTime,
    pid: u32,
    root_dir: Arc<str>,
    host: Arc<str>,
    port: u16,
}

#[derive(Default)]
struct RequestMetrics {
    total_requests: u64,
    total_bytes_sent: u64,
    last_request_at: Option<SystemTime>,
    route_counts: HashMap<String, u64>,
    client_counts: HashMap<String, u64>,
    method_counts: HashMap<String, u64>,
    status_counts: HashMap<u16, u64>,
    recent_requests: VecDeque<RequestRecord>,
    route_count_overflowed: bool,
    client_count_overflowed: bool,
    other_route_requests: u64,
    other_client_requests: u64,
}

struct ResourceMonitor {
    system: System,
    history: VecDeque<ResourceHistoryPoint>,
}

#[derive(Clone)]
pub struct RequestEvent {
    pub method: String,
    pub path: String,
    pub route: String,
    pub status: u16,
    pub duration_ms: u64,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub response_bytes: Option<u64>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct StatsSnapshot {
    pub generated_at_ms: u64,
    pub uptime_seconds: u64,
    pub server: ServerSnapshot,
    pub overview: OverviewSnapshot,
    pub resources: ResourceSnapshot,
    pub top_routes: Vec<CountItem>,
    pub top_clients: Vec<CountItem>,
    pub methods: Vec<CountItem>,
    pub statuses: Vec<StatusCountItem>,
    pub recent_requests: Vec<RequestRecord>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ServerSnapshot {
    pub root_dir: String,
    pub host: String,
    pub port: u16,
    pub pid: u32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct OverviewSnapshot {
    pub total_requests: u64,
    pub unique_routes: usize,
    pub unique_routes_capped: bool,
    pub unique_clients: usize,
    pub unique_clients_capped: bool,
    pub total_bytes_sent: u64,
    pub requests_per_minute: f64,
    pub last_request_at_ms: Option<u64>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ResourceSnapshot {
    pub process_cpu_percent: f32,
    pub process_memory_bytes: u64,
    pub process_virtual_memory_bytes: u64,
    pub system_cpu_percent: f32,
    pub system_memory_used_bytes: u64,
    pub system_memory_total_bytes: u64,
    pub system_memory_percent: f32,
    pub load_avg_one: f64,
    pub load_avg_five: f64,
    pub load_avg_fifteen: f64,
    pub history: Vec<ResourceHistoryPoint>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CountItem {
    pub label: String,
    pub count: u64,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct StatusCountItem {
    pub status: u16,
    pub count: u64,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RequestRecord {
    pub timestamp_ms: u64,
    pub method: String,
    pub path: String,
    pub route: String,
    pub status: u16,
    pub duration_ms: u64,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub response_bytes: Option<u64>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ResourceHistoryPoint {
    pub timestamp_ms: u64,
    pub process_cpu_percent: f32,
    pub process_memory_bytes: u64,
    pub system_cpu_percent: f32,
    pub system_memory_percent: f32,
}

impl MetricsStore {
    pub fn new(root_dir: &Path, host: &str, port: u16) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            inner: Arc::new(Mutex::new(RequestMetrics::default())),
            resources: Arc::new(Mutex::new(ResourceMonitor {
                system,
                history: VecDeque::new(),
            })),
            started_at: Instant::now(),
            started_at_system: SystemTime::now(),
            pid: std::process::id(),
            root_dir: Arc::from(root_dir.display().to_string()),
            host: Arc::from(host.to_string()),
            port,
        }
    }

    pub fn record_request(&self, event: RequestEvent) {
        let mut metrics = self.inner.lock().expect("request metrics lock poisoned");
        metrics.total_requests += 1;
        metrics.total_bytes_sent += event.response_bytes.unwrap_or(0);
        metrics.last_request_at = Some(SystemTime::now());
        *metrics.method_counts.entry(event.method.clone()).or_insert(0) += 1;
        *metrics.status_counts.entry(event.status).or_insert(0) += 1;
        if let Some(count) = metrics.route_counts.get_mut(&event.route) {
            *count += 1;
        } else if metrics.route_counts.len() < MAX_TRACKED_ROUTE_KEYS {
            metrics.route_counts.insert(event.route.clone(), 1);
        } else {
            metrics.route_count_overflowed = true;
            metrics.other_route_requests += 1;
        }

        if let Some(client_ip) = &event.client_ip {
            if let Some(count) = metrics.client_counts.get_mut(client_ip) {
                *count += 1;
            } else if metrics.client_counts.len() < MAX_TRACKED_CLIENT_KEYS {
                metrics.client_counts.insert(client_ip.clone(), 1);
            } else {
                metrics.client_count_overflowed = true;
                metrics.other_client_requests += 1;
            }
        }

        metrics.recent_requests.push_front(RequestRecord {
            timestamp_ms: unix_ms(SystemTime::now()),
            method: event.method,
            path: event.path,
            route: event.route,
            status: event.status,
            duration_ms: event.duration_ms,
            client_ip: event.client_ip,
            user_agent: event.user_agent,
            response_bytes: event.response_bytes,
        });

        while metrics.recent_requests.len() > MAX_RECENT_REQUESTS {
            metrics.recent_requests.pop_back();
        }
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        let resource_snapshot = {
            let mut resources = self.resources.lock().expect("resource monitor lock poisoned");
            resources.snapshot(self.pid)
        };

        let metrics = self.inner.lock().expect("request metrics lock poisoned");
        let uptime_seconds = self.started_at.elapsed().as_secs();
        let uptime_minutes = (self.started_at.elapsed().as_secs_f64() / 60.0).max(1.0 / 60.0);

        StatsSnapshot {
            generated_at_ms: unix_ms(SystemTime::now()),
            uptime_seconds,
            server: ServerSnapshot {
                root_dir: self.root_dir.to_string(),
                host: self.host.to_string(),
                port: self.port,
                pid: self.pid,
            },
            overview: OverviewSnapshot {
                total_requests: metrics.total_requests,
                unique_routes: metrics.route_counts.len(),
                unique_routes_capped: metrics.route_count_overflowed,
                unique_clients: metrics.client_counts.len(),
                unique_clients_capped: metrics.client_count_overflowed,
                total_bytes_sent: metrics.total_bytes_sent,
                requests_per_minute: metrics.total_requests as f64 / uptime_minutes,
                last_request_at_ms: metrics.last_request_at.map(unix_ms),
            },
            resources: resource_snapshot,
            top_routes: sorted_count_items(&metrics.route_counts, 10, metrics.other_route_requests, "[other routes]"),
            top_clients: sorted_count_items(&metrics.client_counts, 8, metrics.other_client_requests, "[other clients]"),
            methods: sorted_count_items(&metrics.method_counts, 8, 0, ""),
            statuses: sorted_status_items(&metrics.status_counts),
            recent_requests: metrics.recent_requests.iter().cloned().collect(),
        }
    }

    pub fn root_dir(&self) -> &str {
        &self.root_dir
    }

    pub fn started_at_ms(&self) -> u64 {
        unix_ms(self.started_at_system)
    }
}

impl ResourceMonitor {
    fn snapshot(&mut self, pid: u32) -> ResourceSnapshot {
        self.system.refresh_all();

        let process = sysinfo::Pid::from_u32(pid);
        let process_snapshot = self.system.process(process);
        let process_cpu_percent = process_snapshot.map(|item| item.cpu_usage()).unwrap_or(0.0);
        let process_memory_bytes = process_snapshot.map(|item| item.memory()).unwrap_or(0);
        let process_virtual_memory_bytes = process_snapshot
            .map(|item| item.virtual_memory())
            .unwrap_or(0);
        let system_memory_total_bytes = self.system.total_memory();
        let system_memory_used_bytes = self.system.used_memory();
        let system_memory_percent = if system_memory_total_bytes == 0 {
            0.0
        } else {
            (system_memory_used_bytes as f64 / system_memory_total_bytes as f64 * 100.0) as f32
        };
        let system_cpu_percent = self.system.global_cpu_info().cpu_usage();
        let load_avg = System::load_average();

        self.history.push_back(ResourceHistoryPoint {
            timestamp_ms: unix_ms(SystemTime::now()),
            process_cpu_percent,
            process_memory_bytes,
            system_cpu_percent,
            system_memory_percent,
        });
        while self.history.len() > MAX_RESOURCE_HISTORY {
            self.history.pop_front();
        }

        ResourceSnapshot {
            process_cpu_percent,
            process_memory_bytes,
            process_virtual_memory_bytes,
            system_cpu_percent,
            system_memory_used_bytes,
            system_memory_total_bytes,
            system_memory_percent,
            load_avg_one: load_avg.one,
            load_avg_five: load_avg.five,
            load_avg_fifteen: load_avg.fifteen,
            history: self.history.iter().cloned().collect(),
        }
    }
}

pub async fn track_request(
    State(state): State<crate::server::AppState>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let route = req.uri().path().to_string();
    let path = match req.uri().query() {
        Some(_) => format!("{}?...", req.uri().path()),
        None => req.uri().path().to_string(),
    };
    let client_ip = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|info| info.0.ip().to_string());
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let response = next.run(req).await;
    let status = response.status().as_u16();
    let response_bytes = if method == "HEAD" {
        Some(0)
    } else {
        response
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
    };
    let duration_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    if route != state.dashboard_data_path.as_ref() {
        state.metrics.record_request(RequestEvent {
            method: method.clone(),
            path: path.clone(),
            route,
            status,
            duration_ms,
            client_ip,
            user_agent,
            response_bytes,
        });
    }

    tracing::info!("{} {} {} {}ms", method, path, status, duration_ms);

    response
}

fn sorted_count_items(
    map: &HashMap<String, u64>,
    limit: usize,
    overflow_count: u64,
    overflow_label: &str,
) -> Vec<CountItem> {
    let mut items: Vec<_> = map
        .iter()
        .map(|(label, count)| CountItem {
            label: label.clone(),
            count: *count,
        })
        .collect();

    if overflow_count > 0 {
        items.push(CountItem {
            label: overflow_label.to_string(),
            count: overflow_count,
        });
    }

    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(limit);
    items
}


fn sorted_status_items(map: &HashMap<u16, u64>) -> Vec<StatusCountItem> {
    let mut items: Vec<_> = map
        .iter()
        .map(|(status, count)| StatusCountItem {
            status: *status,
            count: *count,
        })
        .collect();

    items.sort_by(|left, right| left.status.cmp(&right.status));
    items
}

fn unix_ms(value: SystemTime) -> u64 {
    value
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
