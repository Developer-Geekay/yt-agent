use serde::{Deserialize, Serialize};

// === API Request/Response Models ===

/// The query parameters for a `GET /formats` request.
#[derive(Deserialize, Debug)]
pub struct FormatRequest {
    pub url: String,
}

/// Represents the top-level JSON output from `yt-dlp --dump-json`.
#[derive(Serialize, Deserialize, Debug)]
pub struct VideoInfo {
    pub title: String,
    pub formats: Vec<Format>,
    pub thumbnail: Option<String>,
}

/// Represents a single format available for download.
#[derive(Serialize, Deserialize, Debug)]
pub struct Format {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    #[serde(default)]
    pub vcodec: String,
    #[serde(default)]
    pub acodec: String,
    #[serde(default)]
    pub filesize: Option<u64>,
    #[serde(default)]
    pub tbr: Option<f64>, // Total Bitrate in KBit/s
}

// === Download & Status Models ===

/// The JSON body for a `POST /download` request with extended functionality.
#[derive(Deserialize, Debug)]
pub struct DownloadRequest {
    // === Core Fields ===
    pub url: String,
    pub format_id: String,

    // === Filesystem & Metadata Fields ===
    /// Output template for the filename, e.g., "downloads/%(uploader)s/%(title)s.%(ext)s"
    /// Replaces the old `output_path`.
    pub output_template: Option<String>,
    #[serde(default)]
    pub write_info_json: bool,
    #[serde(default)]
    pub write_thumbnail: bool,
    #[serde(default)]
    pub restrict_filenames: bool,

    // === Filtering Fields ===
    /// e.g., "1-3,7"
    pub playlist_items: Option<String>,
    /// e.g., "duration > 600 & like_count > 1000"
    pub match_filter: Option<String>,
    /// e.g., "50M" or "1G"
    pub max_filesize: Option<String>,

    // === Post-Processing Fields ===
    /// If true, triggers audio extraction.
    #[serde(default)]
    pub extract_audio: bool,
    /// e.g., "mp3", "flac", "wav"
    pub audio_format: Option<String>,
    /// e.g., "0" (best VBR) or "128K"
    pub audio_quality: Option<String>,
    /// e.g., "mkv", "mp4"
    pub remux_video: Option<String>,
    pub embed_thumbnail: Option<bool>,

    // === SponsorBlock Fields ===
    /// e.g., "sponsor,selfpromo" or "all"
    pub sponsorblock_remove: Option<String>,
    /// e.g., "all,-outro"
    pub sponsorblock_mark: Option<String>,
}

/// The response sent after successfully starting a download.
#[derive(Serialize, Debug)]
pub struct DownloadResponse {
    pub message: String,
    pub download_key: String,
}

/// Represents the real-time status of a single download.
/// This will be stored in our shared state.
#[derive(Clone, Serialize, Debug, Default)]
pub struct DownloadStatus {
    pub status: String, // e.g., "starting", "downloading", "completed", "failed"
    pub progress: f64,
    pub eta: String,    // Estimated Time of Arrival
    pub speed: String,
    pub error: Option<String>,
}
