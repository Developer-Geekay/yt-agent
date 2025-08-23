# Rust Media Server for yt-dlp

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)![Axum](https://img.shields.io/badge/axum-v0.7-blue?style=for-the-badge)![Clap](https://img.shields.io/badge/clap-v4-yellow?style=for-the-badge)

A powerful, cross-platform backend server written in Rust to provide a web API for the `yt-dlp` command-line tool. This server is designed to be a self-contained, manageable service that acts as the backend for a web interface (e.g., built with Angular, React, or Vue) for downloading and managing media files.

## ✨ Features

-   **Cross-Platform Service**: Manage the server as a background process on Windows, macOS, and Linux with `start`, `stop`, `restart`, and `status` commands.
-   **Comprehensive Downloading**: Exposes the full power of `yt-dlp` through a simple API.
-   **Advanced Post-Processing**:
    -   Extract audio into formats like `mp3`, `flac`, `aac`, etc.
    -   Remux video into different containers (`mkv`, `mp4`).
    -   Embed thumbnails as cover art.
-   **Powerful Filtering**:
    -   Download specific items from a playlist (`1,3-5`).
    -   Filter videos by metadata like `duration`, `like_count`, `filesize`, etc.
-   **SponsorBlock Integration**: Automatically mark or remove sponsored segments, intros, and other annoying sections from videos.
-   **Dynamic Configuration**:
    -   Settings are managed via a `config.toml` file.
    -   An API endpoint (`/config`) allows for live configuration changes without a server restart.
    -   Command-line flags can override settings for ultimate flexibility.
-   **Real-time Progress Tracking**: A status endpoint provides live updates on download progress, speed, and ETA.
-   **File Management**: List all downloaded files and serve them for download through a sandboxed API.

## 📋 Prerequisites

Before you begin, ensure you have the following installed on your system and available in your system's `PATH`.

1.  **Rust Toolchain**: [Install via rustup](https://www.rust-lang.org/tools/install).
2.  **yt-dlp**: [Installation instructions](https://github.com/yt-dlp/yt-dlp#installation).
3.  **FFmpeg**: [Installation instructions](https://ffmpeg.org/download.html). `yt-dlp` requires `ffmpeg` to merge separate video and audio formats (like DASH streams used by YouTube) and for post-processing.

## 🚀 Getting Started

### 1. Installation

Clone the repository and build the project. Building in release mode is highly recommended for performance.

```bash
git clone <your-repo-url>
cd <your-repo-name>
cargo build --release
```

The final executable will be located at `target/release/your-binary-name` (or `.exe` on Windows).

### 2. Configuration

On the first run, the server will automatically create a `config.toml` file in your system's standard configuration directory. This file contains the default settings.

-   **Default Download Directory**: The server smartly detects your OS's default "Downloads" folder (e.g., `/home/user/Downloads`, `C:\Users\user\Downloads`) and sets it as the default. You can change this at any time via the API or by editing the file.

### 3. Managing the Server

The application is a self-contained service manager. Use the `server` subcommand to control it.

**Start the server in the background:**
```bash
./target/release/your-binary-name server start
```

**Check if the server is running:**
```bash
./target/release/your-binary-name server status
```

**Stop the background server:**
```bash
./target/release/your-binary-name server stop
```

**Restart the server:**
```bash
./target/release/your-binary-name server restart
```

**Run in the foreground (for debugging):**
```bash
./target/release/your-binary-name server run
```

### 4. Command-Line Overrides

You can override key settings with command-line flags when running the server. These take precedence over the `config.toml` file.

-   `--host <IP>`: The IP address to bind to (e.g., `0.0.0.0`).
-   `-p, --port <PORT>`: The port to listen on.
-   `-d, --directory <PATH>`: The default directory for downloads.

**Example (run publicly on port 3000 with a custom download directory):**
```bash
./target/release/your-binary-name server run --host 0.0.0.0 --port 3000 --directory /mnt/media
```

## 📖 API Documentation

### `GET /config`

Returns the current application configuration.

-   **Example Request**:
    ```bash
    curl http://localhost:8080/config
    ```
-   **Success Response (`200 OK`)**:
    ```json
    {
      "download_directory": "/home/your_user/Downloads"
    }
    ```

### `POST /config`

Updates the application configuration live and saves it to the `config.toml` file.

-   **Example Request**:
    ```bash
    curl -X POST http://localhost:8080/config \
    -H "Content-Type: application/json" \
    -d '{"download_directory": "/media/new_videos"}'
    ```

### `GET /formats`

Fetches all available download formats for a given media URL.

-   **Query Parameters**:
    -   `url` (string, required): The URL of the video to inspect.
-   **Example Request**:
    ```bash
    curl "http://localhost:8080/formats?url=https://www.youtube.com/watch?v=aqz-KE-bpKQ"
    ```

### `POST /download`

Starts a new download in the background with a rich set of options.

-   **JSON Body**:
    -   `url` (string, required): The URL of the media.
    -   `format_id` (string, required): The format ID. Use `+` to combine video and audio (e.g., `"137+140"`).
    -   `output_template` (string, optional): A `yt-dlp` output template. If omitted, uses the default from the configuration.
    -   `extract_audio` (boolean, optional): If `true`, convert to an audio-only file.
    -   `audio_format` (string, optional): E.g., `mp3`, `flac`, `wav`.
    -   `audio_quality` (string, optional): E.g., `0` (best) or `128K`.
    -   `remux_video` (string, optional): E.g., `mkv`, `mp4`.
    -   `playlist_items` (string, optional): E.g., `"1,3-5"`.
    -   `match_filter` (string, optional): E.g., `"duration > 600 & like_count > 1000"`.
    -   `sponsorblock_remove` (string, optional): E.g., `"sponsor,selfpromo"`.
    -   ...and many more. See `models.rs` for the full list.
-   **Example Request (Audio Extraction)**:
    ```bash
    # Extracts audio to an MP3 file in the configured download directory.
    curl -X POST http://localhost:8080/download \
    -H "Content-Type: application/json" \
    -d '{
      "url": "https://www.youtube.com/watch?v=aqz-KE-bpKQ",
      "format_id": "bestaudio",
      "extract_audio": true,
      "audio_format": "mp3"
    }'
    ```
-   **Success Response (`202 Accepted`)**:
    ```json
    {
      "message": "Download started successfully",
      "download_key": "https://www.youtube.com/watch?v=aqz-KE-bpKQ"
    }
    ```

### `GET /status`

Retrieves the real-time status of all downloads.

-   **Example Request**:
    ```bash
    curl http://localhost:8080/status
    ```

### `GET /files`

Lists all files located within the **configured** download directory.

-   **Example Request**:
    ```bash
    curl http://localhost:8080/files
    ```

### `GET /files/:path`

Serves a specific file for download from the **configured** download directory.

-   **Path Parameter**:
    -   `:path` (string, required): The URL-encoded relative path of the file (as returned by `GET /files`).
-   **Example Request**:
    ```bash
    # Note: Spaces and other special characters must be URL-encoded.
    curl http://localhost:8080/files/Big%20Buck%20Bunny...mp4 -o my_local_file.mp4
    ```

## ⚠️ Security Considerations

-   **Local Use Only**: This server is designed for personal, local use. Do not expose it directly to the internet without a proper authentication layer in front of it.
-   **File System Access**: The server process can write to any directory specified in the `config.toml` or via the `/config` API. Ensure the user running the server has appropriate, limited permissions.
-   **Sandboxed File Serving**: The `GET /files` and `GET /files/:path` endpoints are sandboxed and will **only** ever list or serve files from within the directory specified in your configuration. This prevents accidental exposure of sensitive system files.