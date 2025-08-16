use axum::{
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
// --- FIX 1: Only import Daemonize on Unix platforms ---
#[cfg(unix)]
use daemonize::Daemonize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, RwLock};
use sysinfo::{Pid, System};
use tower_http::cors::{Any, CorsLayer};

use crate::config::{Config, load_config};
use crate::models::DownloadStatus;

// --- Modules ---
pub mod config;
pub mod error;
pub mod handlers;
pub mod models;

// --- State Type Aliases ---
pub type DownloadState = Arc<Mutex<HashMap<String, DownloadStatus>>>;
pub type ConfigState = Arc<RwLock<Config>>;

#[derive(Clone)]
pub struct AppState {
    pub downloads: DownloadState,
    pub config: ConfigState,
}

// --- Command-Line Argument Parsing ---
#[derive(Parser, Debug)]
#[command(author, version, about = "A backend API for yt-dlp.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manages the server process.
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },
}

#[derive(Subcommand, Debug)]
enum ServerAction {
    /// Start the server as a background process.
    Start,
    /// Stop the background server process.
    Stop,
    /// Restart the background server process.
    Restart,
    /// Run the server in the foreground.
    Run,
    /// Check the status of the background server process.
    Status,
}

// --- Main Application Logic ---
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Server { action } => match action {
            ServerAction::Start => start_server()?,
            ServerAction::Stop => stop_server()?,
            ServerAction::Restart => {
                stop_server()?;
                std::thread::sleep(std::time::Duration::from_secs(1));
                start_server()?;
            }
            ServerAction::Run => run_server().await?,
            ServerAction::Status => check_status()?,
        },
    }

    Ok(())
}

// --- Server Action Functions ---

/// The core function that runs the Axum web server.
async fn run_server() -> anyhow::Result<()> {
    // ... This function remains unchanged ...
    tracing_subscriber::fmt::init();
    let config = load_config().await?;
    let state = AppState {
        downloads: Arc::new(Mutex::new(HashMap::new())),
        config: Arc::new(RwLock::new(config)),
    };
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port_str = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port_str);
    let app = Router::new()
        .route("/formats", get(handlers::list_formats))
        .route("/download", post(handlers::start_download))
        .route("/status", get(handlers::get_status))
        .route("/files", get(handlers::list_files))
        .route("/files/*path", get(handlers::get_file))
        .route("/config", get(handlers::get_config).post(handlers::update_config))
        .layer(CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(Any))
        .with_state(state);
    tracing::info!("Starting server in foreground, listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Starts the server as a background process using platform-specific logic.
fn start_server() -> anyhow::Result<()> {
    if is_running()? {
        println!("Server is already running.");
        return Ok(());
    }

    let pid_file = get_pid_path()?;
    let myself = env::current_exe()?;
    println!("Starting server in the background...");

    // --- FIX 2: Use #[cfg(unix)] for the Unix-specific daemonization code ---
    #[cfg(unix)]
    {
        // This code will only be compiled for Linux, macOS, etc.
        let daemonize = Daemonize::new().pid_file(&pid_file);
        match daemonize.start() {
            Ok(_) => {
                // This code runs in the detached background process
                // We re-launch the executable with the `server run` command
                Command::new(&myself).arg("server").arg("run").spawn()?;
            }
            Err(e) => eprintln!("Error, failed to daemonize: {}", e),
        }
    }

    // --- Use #[cfg(windows)] for the Windows-specific process spawning code ---
    #[cfg(windows)]
    {
        // This code will only be compiled for Windows
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        let child = Command::new(&myself)
            .arg("server")
            .arg("run")
            .creation_flags(CREATE_NO_WINDOW) // Prevents a console window from appearing
            .spawn()?;
            
        // Save the PID to the file
        fs::write(&pid_file, child.id().to_string())?;
    }

    println!("Server started successfully. PID file at: {}", pid_file.display());
    Ok(())
}

/// Stops the background server process.
fn stop_server() -> anyhow::Result<()> {
    // ... This function remains unchanged ...
    let pid_file = get_pid_path()?;
    if !pid_file.exists() {
        println!("Server is not running (no PID file).");
        return Ok(());
    }
    let pid_str = fs::read_to_string(&pid_file)?;
    let pid: u32 = pid_str.trim().parse()?;
    let s = System::new_all();
    if let Some(process) = s.process(Pid::from_u32(pid)) {
        println!("Stopping server process with PID: {}", pid);
        process.kill();
    } else {
        println!("Process with PID {} not found. It may have already stopped.", pid);
    }
    fs::remove_file(&pid_file)?;
    println!("Server stopped.");
    Ok(())
}

/// Checks if the server process is running.
fn check_status() -> anyhow::Result<()> {
    // ... This function remains unchanged ...
    if is_running()? {
        let pid_str = fs::read_to_string(get_pid_path()?)?;
        println!("Server is running with PID: {}", pid_str.trim());
    } else {
        println!("Server is not running.");
    }
    Ok(())
}


// --- Helper Functions ---

/// Gets the path for the server's PID file.
fn get_pid_path() -> anyhow::Result<PathBuf> {
    // ... This function remains unchanged ...
    let project_dirs = directories::ProjectDirs::from("com", "YourOrg", "YT-DLP-API")
        .ok_or_else(|| anyhow::anyhow!("Could not find a valid project directory"))?;
    let data_dir = project_dirs.data_local_dir();
    fs::create_dir_all(data_dir)?;
    Ok(data_dir.join("server.pid"))
}

/// Checks if the server is running by checking the PID file and the process list.
fn is_running() -> anyhow::Result<bool> {
    // ... This function remains unchanged ...
    let pid_file = get_pid_path()?;
    if !pid_file.exists() {
        return Ok(false);
    }
    let pid_str = fs::read_to_string(pid_file)?;
    let pid: u32 = pid_str.trim().parse()?;
    let s = System::new_all();
    Ok(s.process(Pid::from_u32(pid)).is_some())
}