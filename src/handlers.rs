use crate::{
    config::{self, Config},
    error::AppError,
    models::{DownloadRequest, DownloadResponse, DownloadStatus, FormatRequest, VideoInfo},
    AppState, DownloadState,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::{wrappers::LinesStream, StreamExt};
use walkdir::WalkDir;

static YTDLP_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[download\]\s+(?P<progress>[\d\.]+)%\s+of\s+~?\s*(?P<size>[\d\.\w/]+)(?:\s+at\s+(?P<speed>[\d\.\w/]+))?\s+ETA\s+(?P<eta>[\d:]+)").unwrap()
});


// ===================================================================
//                          CONFIG HANDLERS
// ===================================================================

/// # GET /config - Returns the current application configuration.
pub async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let config = state.config.read().unwrap().clone();
    Ok((StatusCode::OK, Json(config)))
}

/// # POST /config - Updates the configuration and saves it to disk.
pub async fn update_config(
    State(state): State<AppState>,
    Json(payload): Json<Config>,
) -> Result<impl IntoResponse, AppError> {
    *state.config.write().unwrap() = payload.clone();
    config::save_config(&payload).await?;
    tracing::info!("Configuration updated and saved.");
    Ok((StatusCode::OK, Json(payload)))
}

// ===================================================================
//                          FORMATS HANDLER
// ===================================================================

/// # GET /formats - Fetches available formats for a given video URL.
pub async fn list_formats(Query(params): Query<FormatRequest>) -> Result<impl IntoResponse, AppError> {
    if params.url.is_empty() {
        return Err(AppError::BadRequest("URL parameter cannot be empty".to_string()));
    }
    tracing::info!("Fetching formats for URL: {}", params.url);

    let output = Command::new("yt-dlp").arg("--dump-json").arg(&params.url).output().await?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        tracing::error!("yt-dlp failed: {}", error_message);
        return Err(AppError::YtDlp(error_message));
    }

    let info: VideoInfo = serde_json::from_slice(&output.stdout)?;
    tracing::info!("Successfully fetched {} formats for '{}'", info.formats.len(), info.title);
    Ok((StatusCode::OK, Json(info)))
}

// ===================================================================
//                          DOWNLOAD HANDLERS
// ===================================================================

/// # POST /download - Spawns a background download process.
pub async fn start_download(
    State(state): State<AppState>,
    Json(payload): Json<DownloadRequest>,
) -> Result<impl IntoResponse, AppError> {
    let download_key = payload.url.clone();

    // Determine the final output template. Use the request's template if it exists,
    // otherwise, build one from the global config.
    let output_template = payload.output_template.clone().unwrap_or_else(|| {
        let config = state.config.read().unwrap();
        let download_dir = PathBuf::from(&config.download_directory);
        download_dir.join("%(title)s [%(id)s].%(ext)s").to_string_lossy().to_string()
    });

    // Ensure the base download directory from config exists.
    let base_downloads_path = get_download_dir_from_state(&state);
    tokio::fs::create_dir_all(&base_downloads_path).await?;

    // Check for existing downloads and set initial status.
    {
        // CORRECTED: Access state.downloads, not state.
        let mut map = state.downloads.lock().unwrap();
        if matches!(map.get(&download_key), Some(s) if s.status == "downloading" || s.status == "starting") {
            return Err(AppError::BadRequest("A download for this URL is already in progress.".to_string()));
        }
        map.insert(download_key.clone(), DownloadStatus { status: "starting".to_string(), ..Default::default() });
    }

    // Spawn the actual download logic in a separate, non-blocking task.
    tokio::spawn(run_download_task(
        state.downloads.clone(),
        download_key.clone(),
        payload,
        output_template,
    ));

    Ok((StatusCode::ACCEPTED, Json(DownloadResponse {
        message: "Download started successfully".to_string(),
        download_key,
    })))
}

/// The core long-running task for a single download.
/// This function is spawned by `start_download` and runs in the background.
async fn run_download_task(
    downloads_state: DownloadState,
    download_key: String,
    payload: DownloadRequest,
    output_template: String,
) {
    let mut cmd = Command::new("yt-dlp");

    cmd.arg("-f").arg(&payload.format_id)
       .arg("--newline")
       .arg("-o").arg(&output_template);

    // Conditionally add arguments based on the request payload
    if payload.write_info_json { cmd.arg("--write-info-json"); }
    if payload.write_thumbnail { cmd.arg("--write-thumbnail"); }
    if payload.restrict_filenames { cmd.arg("--restrict-filenames"); }
    if let Some(items) = &payload.playlist_items { cmd.arg("--playlist-items").arg(items); }
    if let Some(filter) = &payload.match_filter { cmd.arg("--match-filters").arg(filter); }
    if let Some(size) = &payload.max_filesize { cmd.arg("--max-filesize").arg(size); }
    if payload.extract_audio {
        cmd.arg("--extract-audio");
        if let Some(format) = &payload.audio_format { cmd.arg("--audio-format").arg(format); }
        if let Some(quality) = &payload.audio_quality { cmd.arg("--audio-quality").arg(quality); }
    } else if let Some(format) = &payload.remux_video {
        cmd.arg("--remux-video").arg(format);
    }
    if payload.embed_thumbnail.unwrap_or(false) { cmd.arg("--embed-thumbnail"); }
    if let Some(cats) = &payload.sponsorblock_remove { cmd.arg("--sponsorblock-remove").arg(cats); }
    if let Some(cats) = &payload.sponsorblock_mark { cmd.arg("--sponsorblock-mark").arg(cats); }

    cmd.arg(&payload.url).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            update_status_to_failed(&downloads_state, &download_key, format!("Failed to start yt-dlp process: {}", e));
            return;
        }
    };

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout).lines();
        let mut lines = LinesStream::new(reader);
        while let Some(Ok(line)) = lines.next().await {
            if let Some(caps) = YTDLP_REGEX.captures(&line) {
                let mut map = downloads_state.lock().unwrap();
                if let Some(status) = map.get_mut(&download_key) {
                    status.status = "downloading".to_string();
                    status.progress = caps.name("progress").and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    status.eta = caps.name("eta").map_or_else(String::new, |m| m.as_str().to_string());
                    status.speed = caps.name("speed").map_or_else(String::new, |m| m.as_str().to_string());
                }
            }
        }
    }

    let output = match child.wait_with_output().await {
        Ok(output) => output,
        Err(e) => {
            update_status_to_failed(&downloads_state, &download_key, format!("Download process failed to execute: {}", e));
            return;
        }
    };

    let (final_status_str, final_error) = if output.status.success() {
        ("completed", None)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        tracing::error!("Download failed for {}: {}", download_key, &stderr);
        ("failed", Some(stderr))
    };

    let mut map = downloads_state.lock().unwrap();
    if let Some(status) = map.get_mut(&download_key) {
        status.status = final_status_str.to_string();
        status.error = final_error;
        if status.status == "completed" { status.progress = 100.0; }
    }
}

// ===================================================================
//                          STATUS & FILE HANDLERS
// ===================================================================

/// # GET /status - Returns the status of all downloads.
pub async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    let map = state.downloads.lock().unwrap();
    (StatusCode::OK, Json(map.clone()))
}

/// # GET /files - Lists all downloaded files.
pub async fn list_files(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let mut files = Vec::new();
    let download_dir = get_download_dir_from_state(&state);

    if !download_dir.exists() {
        return Ok(Json(files));
    }

    for entry in WalkDir::new(&download_dir).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(relative_path) = entry.path().strip_prefix(&download_dir) {
                files.push(relative_path.to_string_lossy().to_string());
            }
        }
    }
    Ok(Json(files))
}

/// # GET /files/:path - Serves a single downloaded file.
pub async fn get_file(State(state): State<AppState>, Path(path): Path<String>) -> Result<impl IntoResponse, AppError> {
    let decoded_path = percent_decode_str(&path).decode_utf8_lossy().to_string();
    let download_dir = get_download_dir_from_state(&state);
    let file_path = download_dir.join(&decoded_path);

    let canonical_base = tokio::fs::canonicalize(&download_dir).await?;
    let canonical_file = tokio::fs::canonicalize(&file_path).await.map_err(|_| AppError::NotFound(format!("File '{}' not found.", decoded_path)))?;

    if !canonical_file.starts_with(canonical_base) {
        return Err(AppError::NotFound("File not found (Path Traversal Attempt)".to_string()));
    }

    let file = tokio::fs::File::open(&file_path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    let disposition = format!("attachment; filename=\"{}\"", file_path.file_name().unwrap_or_default().to_string_lossy());
    headers.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&disposition).unwrap());

    Ok((headers, body))
}

// ===================================================================
//                          HELPER FUNCTIONS
// ===================================================================

/// Helper to get the configured download directory path from the shared state.
fn get_download_dir_from_state(state: &AppState) -> PathBuf {
    let config = state.config.read().unwrap();
    PathBuf::from(&config.download_directory)
}

/// Helper to update a download's status to "failed" with a specific message.
fn update_status_to_failed(state: &DownloadState, key: &str, error_message: String) {
    let mut map = state.lock().unwrap();
    if let Some(status) = map.get_mut(key) {
        status.status = "failed".to_string();
        status.error = Some(error_message);
    }
}
