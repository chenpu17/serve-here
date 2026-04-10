mod daemon;
mod error;
mod format;
mod handler;
mod listing;
mod metrics;
mod network;
mod server;
mod stats;

use std::path::PathBuf;

use clap::Parser;

const DEFAULT_PORT: u16 = 8080;

fn validate_port(s: &str) -> Result<u16, String> {
    let port: u16 = s.parse().map_err(|_| format!("Invalid port: {}", s))?;
    if port == 0 {
        return Err("Port must be 1-65535".to_string());
    }
    Ok(port)
}

#[derive(Parser, Debug)]
#[command(
    name = "serve-here",
    version,
    about = "Serve any local directory over HTTP"
)]
struct Cli {
    /// Directory to serve (defaults to current working directory)
    directory: Option<PathBuf>,

    /// Directory to serve
    #[arg(short = 'd', long = "dir")]
    dir: Option<PathBuf>,

    /// Port to listen on (default: 8080)
    #[arg(short = 'p', long = "port", value_parser = validate_port)]
    port: Option<u16>,

    /// Hostname or IP to bind (default: 0.0.0.0)
    #[arg(short = 'H', long = "host", default_value = "0.0.0.0")]
    host: String,

    /// Run as a background daemon (Unix only)
    #[arg(short = 'D', long = "daemon")]
    daemon: bool,

    /// [internal] Run as daemon child process
    #[arg(long = "daemon-child", hide = true)]
    daemon_child: bool,

    /// Stop a running daemon (use with -p to specify port)
    #[arg(long = "stop")]
    stop: bool,

    /// Show status of running daemon(s)
    #[arg(long = "status")]
    status: bool,
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    let cli = Cli::parse();

    // Handle daemon management commands
    if cli.stop {
        daemon::stop_daemon(cli.port.unwrap_or(DEFAULT_PORT));
        return;
    }

    if cli.status {
        daemon::show_status(cli.port);
        return;
    }

    // Resolve directory
    let root_dir = cli
        .dir
        .or(cli.directory)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let root_dir = std::path::Path::new(&root_dir);
    if !root_dir.is_dir() {
        eprintln!(
            "Error: directory \"{}\" does not exist or is not accessible.",
            root_dir.display()
        );
        std::process::exit(1);
    }

    let root_dir = std::fs::canonicalize(root_dir).unwrap_or_else(|e| {
        eprintln!(
            "Error: directory \"{}\" is not accessible: {}",
            root_dir.display(),
            e
        );
        std::process::exit(1);
    });

    let port = cli.port.unwrap_or(DEFAULT_PORT);
    let host = cli.host;

    // Daemon child mode
    if cli.daemon_child {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            daemon::run_daemon_child(root_dir, port, &host).await;
        });
        return;
    }

    // Daemon mode - spawn child process (never returns)
    if cli.daemon {
        daemon::start_daemon(&root_dir, port, &host);
    }

    // Foreground mode
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match server::run_server(root_dir, &host, port).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Failed to start server: {}", e);
                std::process::exit(1);
            }
        }
    });
}
