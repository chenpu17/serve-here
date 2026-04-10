use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::middleware;
use axum::routing::{any, get};
use axum::Router;
use tokio::net::TcpListener;

use crate::handler;
use crate::metrics::MetricsStore;
use crate::network;

#[derive(Clone)]
pub struct AppState {
    pub root_dir: Arc<PathBuf>,
    pub host: Arc<str>,
    pub port: u16,
    pub dashboard_path: Arc<str>,
    pub dashboard_data_path: Arc<str>,
    pub metrics: MetricsStore,
}

pub fn create_app_state(root_dir: PathBuf, host: &str, port: u16) -> AppState {
    let root_dir = root_dir
        .canonicalize()
        .unwrap_or(root_dir);
    let (dashboard_path, dashboard_data_path) = dashboard_route_paths(&root_dir);

    AppState {
        metrics: MetricsStore::new(&root_dir, host, port),
        root_dir: Arc::new(root_dir),
        host: Arc::from(host.to_string()),
        port,
        dashboard_path: Arc::from(dashboard_path),
        dashboard_data_path: Arc::from(dashboard_data_path),
    }
}

pub fn build_app(state: AppState) -> Router {
    let dashboard_path = state.dashboard_path.to_string();
    let dashboard_data_path = state.dashboard_data_path.to_string();

    Router::new()
        .route(&dashboard_path, get(handler::handle_stats_page))
        .route(&dashboard_data_path, get(handler::handle_stats_data))
        .fallback(any(handler::handle_request))
        .layer(middleware::from_fn_with_state(state.clone(), crate::metrics::track_request))
        .with_state(state)
}

pub async fn run_server(root_dir: PathBuf, host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let state = create_app_state(root_dir, host, port);
    let app = build_app(state.clone());

    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    let listening_port = listener.local_addr()?.port();

    if let Ok(ready_file) = std::env::var("SERVE_HERE_READY_FILE") {
        if let Err(error) = std::fs::write(&ready_file, "ready") {
            tracing::error!("Failed to write daemon readiness file {}: {}", ready_file, error);
        }
    }

    println!("  Serving {}", state.root_dir.display());
    println!("  Listening on:");
    network::print_listening_addresses(host, listening_port);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    }

    tracing::info!("Shutting down...");
}

fn dashboard_route_paths(root_dir: &Path) -> (String, String) {
    if root_dir.join("stats").exists() {
        (
            "/.serve-here/stats".to_string(),
            "/.serve-here/stats/data".to_string(),
        )
    } else {
        ("/stats".to_string(), "/stats/data".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::net::SocketAddr;
    use std::path::Path;

    use reqwest::redirect::Policy;
    use tempfile::TempDir;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    use crate::metrics::StatsSnapshot;

    use super::{build_app, create_app_state};

    struct TestServer {
        _root: TempDir,
        base_url: String,
        client: reqwest::Client,
        handle: JoinHandle<()>,
    }

    impl TestServer {
        async fn spawn() -> Self {
            Self::spawn_with(|_| {}).await
        }

        async fn spawn_with<F>(setup: F) -> Self
        where
            F: FnOnce(&Path),
        {
            let root = TempDir::new().expect("failed to create tempdir");
            seed_fixture(root.path());
            setup(root.path());

            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("failed to bind test listener");
            let port = listener.local_addr().expect("missing local addr").port();
            let app = build_app(create_app_state(
                root.path().to_path_buf(),
                "127.0.0.1",
                port,
            ));

            let handle = tokio::spawn(async move {
                let _ = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
                    .await;
            });

            let client = reqwest::Client::builder()
                .redirect(Policy::none())
                .build()
                .expect("failed to build client");

            Self {
                _root: root,
                base_url: format!("http://127.0.0.1:{}", port),
                client,
                handle,
            }
        }

        async fn get(&self, path: &str) -> reqwest::Response {
            self.client
                .get(format!("{}{}", self.base_url, path))
                .send()
                .await
                .expect("request failed")
        }

        async fn head(&self, path: &str) -> reqwest::Response {
            self.client
                .head(format!("{}{}", self.base_url, path))
                .send()
                .await
                .expect("request failed")
        }

        async fn post(&self, path: &str) -> reqwest::Response {
            self.client
                .post(format!("{}{}", self.base_url, path))
                .send()
                .await
                .expect("request failed")
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.handle.abort();
        }
    }

    #[tokio::test]
    async fn serves_directory_listing_stats_and_files() {
        let server = TestServer::spawn().await;

        let root = server.get("/").await;
        assert_eq!(root.status(), reqwest::StatusCode::OK);
        let root_html = root.text().await.expect("failed to read root html");
        assert!(root_html.contains("Directory explorer"));
        assert!(root_html.contains("Open stats dashboard"));
        assert!(root_html.contains(r#"href="/stats""#));
        assert!(root_html.contains("serve-here-theme"));
        assert!(root_html.contains("打开统计面板"));
        assert!(root_html.contains("hello.txt"));
        assert!(root_html.contains("nested/"));

        let directory_redirect = server.get("/nested").await;
        assert_eq!(directory_redirect.status(), reqwest::StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            directory_redirect
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("/nested/")
        );

        let nested = server.get("/nested/").await;
        assert_eq!(nested.status(), reqwest::StatusCode::OK);
        let nested_html = nested.text().await.expect("failed to read nested html");
        assert!(nested_html.contains("Parent directory"));
        assert!(nested_html.contains(r#"href="/stats""#));
        assert!(nested_html.contains("breadcrumb_root"));
        assert!(nested_html.contains("deep.txt"));

        let file = server.get("/hello.txt").await;
        assert_eq!(file.status(), reqwest::StatusCode::OK);
        assert_eq!(
            file.text().await.expect("failed to read file body"),
            "hello from serve-here\n"
        );

        let head = server.head("/").await;
        assert_eq!(head.status(), reqwest::StatusCode::OK);
        let content_length = head
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .expect("missing content-length");
        assert_ne!(content_length, "0");

        let stats_page = server.get("/stats").await;
        assert_eq!(stats_page.status(), reqwest::StatusCode::OK);
        let stats_html = stats_page.text().await.expect("failed to read stats html");
        assert!(stats_html.contains("Service control deck"));
        assert!(stats_html.contains("/stats/data"));
        assert!(stats_html.contains("serve-here-lang"));
        assert!(stats_html.contains("服务控制台"));

        let stats = server
            .get("/stats/data")
            .await
            .json::<StatsSnapshot>()
            .await
            .expect("failed to decode stats");

        assert_eq!(stats.overview.total_requests, 6);
        assert_eq!(stats.overview.unique_routes, 5);
        assert!(stats.recent_requests.iter().any(|item| item.path == "/stats"));
        assert!(stats.recent_requests.iter().all(|item| item.route != "/stats/data"));
    }

    #[tokio::test]
    async fn caps_recent_request_history_at_one_thousand() {
        let server = TestServer::spawn().await;

        for _ in 0..1005 {
            let response = server.get("/hello.txt").await;
            assert_eq!(response.status(), reqwest::StatusCode::OK);
        }

        let stats = server
            .get("/stats/data")
            .await
            .json::<StatsSnapshot>()
            .await
            .expect("failed to decode stats");

        assert_eq!(stats.overview.total_requests, 1005);
        assert_eq!(stats.recent_requests.len(), 1000);
        assert_eq!(
            stats.top_routes.first().map(|item| item.label.as_str()),
            Some("/hello.txt")
        );
        assert_eq!(stats.top_routes.first().map(|item| item.count), Some(1005));
    }

    #[tokio::test]
    async fn supports_non_ascii_paths_and_rejects_unsupported_methods() {
        let server = TestServer::spawn().await;

        let redirect = server.get("/%E4%B8%AD%E6%96%87").await;
        assert_eq!(redirect.status(), reqwest::StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            redirect
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("/%E4%B8%AD%E6%96%87/")
        );

        let chinese_dir = server.get("/%E4%B8%AD%E6%96%87/").await;
        assert_eq!(chinese_dir.status(), reqwest::StatusCode::OK);
        let chinese_html = chinese_dir
            .text()
            .await
            .expect("failed to read Chinese directory html");
        assert!(chinese_html.contains("readme.txt"));

        let post = server.post("/hello.txt").await;
        assert_eq!(post.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(
            post.headers()
                .get(reqwest::header::ALLOW)
                .and_then(|value| value.to_str().ok()),
            Some("GET, HEAD")
        );
    }

    #[tokio::test]
    async fn preserves_real_stats_paths_when_the_filesystem_uses_that_name() {
        let server = TestServer::spawn_with(|root| {
            fs::write(root.join("stats"), "real stats file\n").expect("failed to write stats file");
        })
        .await;

        let file = server.get("/stats").await;
        assert_eq!(file.status(), reqwest::StatusCode::OK);
        assert_eq!(
            file.text().await.expect("failed to read stats file"),
            "real stats file\n"
        );

        let dashboard = server.get("/.serve-here/stats").await;
        assert_eq!(dashboard.status(), reqwest::StatusCode::OK);
        let dashboard_html = dashboard.text().await.expect("failed to read dashboard html");
        assert!(dashboard_html.contains("Service control deck"));
        assert!(dashboard_html.contains("/.serve-here/stats/data"));

        let root = server.get("/").await;
        assert_eq!(root.status(), reqwest::StatusCode::OK);
        let root_html = root.text().await.expect("failed to read root listing html");
        assert!(root_html.contains(r#"href="/.serve-here/stats""#));

        let data = server
            .get("/.serve-here/stats/data")
            .await
            .json::<StatsSnapshot>()
            .await
            .expect("failed to decode internal stats");
        assert!(data.overview.total_requests >= 2);
    }

    fn seed_fixture(root: &Path) {
        fs::write(root.join("hello.txt"), "hello from serve-here\n").expect("failed to write hello.txt");
        fs::create_dir_all(root.join("nested")).expect("failed to create nested dir");
        fs::write(root.join("nested").join("deep.txt"), "deep file\n").expect("failed to write deep.txt");
        fs::create_dir_all(root.join("中文")).expect("failed to create Chinese dir");
        fs::write(root.join("中文").join("readme.txt"), "unicode path\n")
            .expect("failed to write unicode file");
    }
}
