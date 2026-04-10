use std::fs;
use std::path::{Path, PathBuf};

const PID_DIR_NAME: &str = ".serve-here";
const LOG_DIR_NAME: &str = "logs";

fn home_dir() -> PathBuf {
    dirs_home().expect("Could not determine home directory")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn pid_dir() -> PathBuf {
    home_dir().join(PID_DIR_NAME)
}

fn log_dir() -> PathBuf {
    pid_dir().join(LOG_DIR_NAME)
}

fn pid_file(port: u16) -> PathBuf {
    pid_dir().join(format!("serve-here-{}.pid", port))
}

fn log_file(port: u16) -> PathBuf {
    log_dir().join(format!("serve-here-{}.log", port))
}

fn ready_file(port: u16) -> PathBuf {
    pid_dir().join(format!("serve-here-{}.ready", port))
}

fn ensure_directories() {
    let _ = fs::create_dir_all(pid_dir());
    let _ = fs::create_dir_all(log_dir());
}

struct PidInfo {
    pid: u32,
    root_dir: String,
}

fn read_pid_file(port: u16) -> Option<PidInfo> {
    let path = pid_file(port);
    let content = fs::read_to_string(&path).ok()?;
    let mut lines = content.trim().splitn(2, '\n');
    let pid: u32 = lines.next()?.trim().parse().ok()?;
    let root_dir = lines.next().unwrap_or("").to_string();
    Some(PidInfo { pid, root_dir })
}

fn write_pid_file(port: u16, pid: u32, root_dir: &str) {
    let path = pid_file(port);
    let _ = fs::write(&path, format!("{}\n{}", pid, root_dir));
}

fn remove_pid_file(port: u16) {
    let path = pid_file(port);
    let _ = fs::remove_file(path);
}

#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    unsafe {
        if libc::kill(pid as i32, 0) == 0 {
            true
        } else {
            // ESRCH = no such process; EPERM = process exists but no permission
            std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
        }
    }
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    false
}

#[cfg(unix)]
pub fn start_daemon(root_dir: &Path, port: u16, host: &str) -> ! {
    use std::os::unix::process::CommandExt;
    use std::process::{exit, Command};

    ensure_directories();

    let root_dir = match fs::canonicalize(root_dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: directory \"{}\" is not accessible: {}", root_dir.display(), e);
            exit(1);
        }
    };

    // Check if already running
    if let Some(info) = read_pid_file(port) {
        if is_process_running(info.pid) {
            eprintln!("Error: Server already running on port {} (PID: {})", port, info.pid);
            eprintln!("Serving: {}", info.root_dir);
            exit(1);
        } else {
            remove_pid_file(port);
        }
    }

    let log_path = log_file(port);
    let ready_path = ready_file(port);
    let _ = fs::remove_file(&ready_path);
    let log_file = fs::File::options().append(true).create(true).open(&log_path).unwrap();

    // Spawn child process with --daemon-child flag
    let current_exe = std::env::current_exe().unwrap();
    let mut child = Command::new(current_exe);
    child
        .arg("--daemon-child")
        .arg("-d")
        .arg(&root_dir)
        .arg("-p")
        .arg(port.to_string())
        .arg("-H")
        .arg(host)
        .env("SERVE_HERE_READY_FILE", &ready_path)
        .stdin(std::process::Stdio::null())
        .stdout(log_file.try_clone().unwrap())
        .stderr(log_file);

    // Detach from parent (create new session)
    unsafe {
        child.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    let mut child = match child.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Failed to start daemon: {}", e);
            exit(1);
        }
    };

    let child_pid = child.id();
    write_pid_file(port, child_pid, &root_dir.to_string_lossy());

    let mut attempts = 0;
    while attempts < 50 {
        if ready_path.exists() {
            let _ = fs::remove_file(&ready_path);
            println!("Server started in background (PID: {})", child_pid);
            println!("Serving: {}", root_dir.display());
            println!("Listening on port: {}", port);
            println!("Log file: {}", log_path.display());
            println!("\nTo stop: serve-here --stop -p {}", port);
            exit(0);
        }

        if let Ok(Some(_)) = child.try_wait() {
            let _ = fs::remove_file(&ready_path);
            remove_pid_file(port);
            eprintln!("Error: Failed to start daemon. Check log file: {}", log_path.display());
            exit(1);
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        attempts += 1;
    }

    let _ = fs::remove_file(&ready_path);
    remove_pid_file(port);
    eprintln!(
        "Error: Timed out waiting for daemon readiness. Check log file: {}",
        log_path.display()
    );
    let _ = child.kill();
    exit(1);

}

#[cfg(not(unix))]
pub fn start_daemon(_root_dir: &Path, _port: u16, _host: &str) -> ! {
    eprintln!("Error: Daemon mode is not supported on Windows.");
    std::process::exit(1);
}

/// Run as daemon child process - start server and manage PID file cleanup
pub async fn run_daemon_child(root_dir: PathBuf, port: u16, host: &str) {
    let root_dir_str = root_dir.to_string_lossy().to_string();

    // Write PID file with our PID
    write_pid_file(port, std::process::id(), &root_dir_str);

    let server_future = crate::server::run_server(root_dir, host, port);

    tokio::pin!(server_future);

    // Wait for either the server to finish or a shutdown signal
    let port_for_cleanup = port;
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        tokio::select! {
            result = &mut server_future => {
                if let Err(e) = result {
                    eprintln!("Server error: {}", e);
                }
                remove_pid_file(port_for_cleanup);
            }
            _ = async {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {},
                    _ = sigterm.recv() => {},
                }
            } => {
                remove_pid_file(port_for_cleanup);
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::select! {
            result = &mut server_future => {
                if let Err(e) = result {
                    eprintln!("Server error: {}", e);
                }
                remove_pid_file(port_for_cleanup);
            }
            _ = tokio::signal::ctrl_c() => {
                remove_pid_file(port_for_cleanup);
            }
        }
    }
}

pub fn stop_daemon(port: u16) {
    let info = match read_pid_file(port) {
        Some(i) => i,
        None => {
            eprintln!("No server running on port {}", port);
            std::process::exit(1);
        }
    };

    if !is_process_running(info.pid) {
        println!("Server (PID: {}) is not running, cleaning up...", info.pid);
        remove_pid_file(port);
        std::process::exit(0);
    }

    #[cfg(unix)]
    {
        unsafe {
            libc::kill(info.pid as i32, libc::SIGTERM);
        }
        println!("Stopping server (PID: {})...", info.pid);

        let mut attempts = 0;
        while attempts < 10 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !is_process_running(info.pid) {
                remove_pid_file(port);
                println!("Server stopped.");
                return;
            }
            attempts += 1;
        }

        eprintln!("Server did not stop gracefully, force killing...");
        unsafe {
            libc::kill(info.pid as i32, libc::SIGKILL);
        }
        remove_pid_file(port);
    }

    #[cfg(not(unix))]
    {
        let _ = info;
        eprintln!("Error: Stop daemon is not supported on Windows.");
        std::process::exit(1);
    }
}

pub fn show_status(port: Option<u16>) {
    if let Some(port) = port {
        match read_pid_file(port) {
            None => println!("No server running on port {}", port),
            Some(info) => {
                let running = is_process_running(info.pid);
                println!("Port {}:", port);
                println!("  PID: {}", info.pid);
                println!("  Status: {}", if running { "running" } else { "stopped" });
                println!("  Directory: {}", info.root_dir);
                println!("  Log: {}", log_file(port).display());

                if !running {
                    remove_pid_file(port);
                }
            }
        }
    } else {
        ensure_directories();
        let entries = match fs::read_dir(pid_dir()) {
            Ok(e) => e,
            Err(_) => {
                println!("No servers running.");
                return;
            }
        };

        let pid_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".pid"))
            .collect();

        if pid_files.is_empty() {
            println!("No servers running.");
            return;
        }

        println!("Running servers:\n");
        for entry in pid_files {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(p) = name
                .strip_prefix("serve-here-")
                .and_then(|s| s.strip_suffix(".pid"))
                .and_then(|s| s.parse::<u16>().ok())
            {
                if let Some(info) = read_pid_file(p) {
                    let running = is_process_running(info.pid);
                    println!("Port {}:", p);
                    println!("  PID: {}", info.pid);
                    println!("  Status: {}", if running { "running" } else { "stopped" });
                    println!("  Directory: {}", info.root_dir);
                    println!();

                    if !running {
                        remove_pid_file(p);
                    }
                }
            }
        }
    }
}
