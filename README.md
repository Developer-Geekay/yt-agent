# Rust Media Server for yt-dlp

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)![Axum](https://img.shields.io/badge/axum-v0.7-blue?style=for-the-badge)

A powerful, local-first backend server written in Rust to provide a web API for the `yt-dlp` command-line tool. This server is designed to be the backend for a web interface (e.g., built with Angular, React, or Vue) for downloading and managing media files.

## ✨ Features

-   **List Available Formats**: Fetch all available video, audio, and combined formats for a given media URL.
-   **Background Downloads**: Initiate downloads that run as background processes, allowing the API to respond instantly.
-   **Real-time Progress Tracking**: A status endpoint provides live updates on download progress, speed, and ETA.
-   **File Management**: List all downloaded files and serve them for download through the API.
-   **Flexible Download Paths**:
    -   Organize downloads into subdirectories within a primary `downloads` folder.
    -   Optionally specify **absolute paths** to save files anywhere on your system.
-   **Secure by Default**: Includes path traversal protection to prevent writing files outside the intended directories (unless an absolute path is explicitly provided).

## 🏗️ Architecture

This project is a native Rust application that runs a web server using the **Axum** framework on top of **Tokio**. It does not use WASM.

-   **Frontend (Not Included)**: A web application (e.g., Angular 19) that interacts with this backend.
-   **Backend (This Project)**: A Rust Axum server that listens for HTTP requests.
-   **Core Tools**: The backend calls the `yt-dlp` and `ffmpeg` command-line tools installed on the host machine to perform the actual downloading and file processing.

## 📋 Prerequisites

Before you begin, ensure you have the following installed on your system and available in your system's PATH.

1.  **Rust Toolchain**: [Install via rustup](https://www.rust-lang.org/tools/install).
2.  **yt-dlp**: [Installation instructions](https://github.com/yt-dlp/yt-dlp#installation).
3.  **FFmpeg**: [Installation instructions](https://ffmpeg.org/download.html). `yt-dlp` requires `ffmpeg` to merge separate video and audio formats (like DASH streams used by YouTube).

## 🚀 Getting Started

### 1. Installation

Clone the repository and build the project. Building in release mode is recommended for performance.

```bash
git clone <your-repo-url>
cd <your-repo-name>
cargo build --release
```

### 2. Running the Server

You can run the server in development mode or by executing the compiled binary.

**Development Mode (with live logging):**
```bash
cargo run
```

**Production Mode (from compiled binary):**
```bash
./target/release/media-server
```

By default, the server will be listening on `http://127.0.0.1:8080`.

### 3. Configuration

The server can be configured with environment variables:

-   `HOST`: The IP address to bind to. (Default: `127.0.0.1`)
-   `PORT`: The port to listen on. (Default: `8080`)

**Example (run on port 3000 and make it accessible on your local network):**
```bash
HOST=0.0.0.0 PORT=3000 cargo run
```

## 📖 API Documentation

### `GET /formats`

Fetches all available download formats for a given media URL.

-   **Query Parameters**:
    -   `url` (string, required): The URL of the video to inspect.
-   **Example Request**:
    ```bash
    curl "http://localhost:8080/formats?url=https://www.youtube.com/watch?v=aqz-KE-bpKQ"
    ```
-   **Success Response (`200 OK`)**:
    ```json
    {
      "title": "Big Buck Bunny 60fps 4K - Official Blender Foundation Short Film",
      "thumbnail": "https://i.ytimg.com/vi/aqz-KE-bpKQ/maxresdefault.jpg",
      "formats": [
        {
          "format_id": "137",
          "ext": "mp4",
          "resolution": "1920x1080",
          "vcodec": "avc1.640028",
          "acodec": "none",
          "filesize": 105873725,
          "tbr": 1450.778
        },
        {
          "format_id": "140",
          "ext": "m4a",
          "resolution": "audio only",
          "vcodec": "none",
          "acodec": "mp4a.40.2",
          "filesize": 1599318,
          "tbr": 129.982
        }
      ]
    }
    ```

### `POST /download`

Starts a new download in the background.

-   **JSON Body**:
    -   `url` (string, required): The URL of the video to download.
    -   `format_id` (string, required): The format ID to download. To combine video and audio, use a `+` (e.g., `"137+140"`).
    -   `output_path` (string, optional): A relative or absolute path for the download folder.
-   **Example Request (Relative Path)**:
    ```bash
    # Downloads to ./downloads/cartoons/
    curl -X POST http://localhost:8080/download \
    -H "Content-Type: application/json" \
    -d '{
      "url": "https://www.youtube.com/watch?v=aqz-KE-bpKQ",
      "format_id": "137+140",
      "output_path": "cartoons"
    }'
    ```
-   **Example Request (Absolute Path)**:
    ```bash
    # On macOS/Linux, downloads to the user's Desktop
    curl -X POST http://localhost:8080/download \
    -H "Content-Type: application/json" \
    -d '{
      "url": "https://www.youtube.com/watch?v=aqz-KE-bpKQ",
      "format_id": "137+140",
      "output_path": "/Users/your-user/Desktop"
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

Retrieves the status of all current and historical downloads for the current server session.

-   **Example Request**:
    ```bash
    curl http://localhost:8080/status
    ```
-   **Success Response (`200 OK`)**:
    ```json
    {
      "https://www.youtube.com/watch?v=aqz-KE-bpKQ": {
        "status": "downloading",
        "progress": 42.1,
        "eta": "00:00:35",
        "speed": "2.51MiB/s",
        "error": null
      }
    }
    ```

### `GET /files`

Lists all files located within the default `./downloads` directory (and its subdirectories).

-   **Example Request**:
    ```bash
    curl http://localhost:8080/files
    ```
-   **Success Response (`200 OK`)**:
    ```json
    [
      "cartoons/Big Buck Bunny 60fps 4K - Official Blender Foundation Short Film - aqz-KE-bpKQ.mp4",
      "other/Some Other Video - fOa8vS532gA.webm"
    ]
    ```

### `GET /files/:path`

Serves a specific file for download from the default `./downloads` directory.

-   **Path Parameter**:
    -   `:path` (string, required): The URL-encoded relative path of the file to download (as returned by `GET /files`).
-   **Example Request**:
    ```bash
    # Note: Spaces and other special characters in the filename must be URL-encoded.
    curl http://localhost:8080/files/cartoons/Big%20Buck%20Bunny...mp4 -o my_local_file.mp4
    ```

## ⚠️ Security Considerations

-   **Local Use Only**: This server is designed for personal, local use. Do not expose it directly to the internet.
-   **Absolute Path Risk**: The feature allowing absolute download paths (`output_path`) grants the API client permission to write files **anywhere** on the host filesystem that the server process can access. This is a significant security risk in a multi-user or public environment.
-   **Sandboxed File Serving**: The `GET /files` and `GET /files/:path` endpoints are sandboxed and will **only** ever list or serve files from within the application's `./downloads` directory, regardless of where other files have been downloaded. This prevents accidental exposure of sensitive system files.



### Prerequisites

1.  The Axum server is running on `localhost:8080`.
2.  You have a command-line tool like [`jq`](https://jqlang.github.io/jq/) installed to pretty-print the JSON responses, which makes them easier to read.
3.  You will need to replace placeholder values like `VIDEO_URL`, `PLAYLIST_URL`, and `FORMAT_ID` with actual data.

---

### 1. `GET /formats` - Getting Available Formats

This endpoint is used to discover the `format_id` for a video, which is required for the download endpoint.

```bash
# Replace VIDEO_URL with a real video URL
VIDEO_URL="https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# The -G flag tells curl to use GET and --data-urlencode handles special characters
curl -G --data-urlencode "url=${VIDEO_URL}" http://localhost:8080/formats | jq```

---

### 2. `POST /download` - Starting a Download

This is the main endpoint with all the new functionality.

#### Scenario 1: Basic Video Download

This is the simplest download, equivalent to the original functionality but using the new `output_template`.

```bash
# Downloads the specified format to the default location "downloads/Video Title [id].ext"
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "VIDEO_URL",
    "format_id": "22"
}' \
http://localhost:8080/download | jq
```

#### Scenario 2: Audio Extraction

Extract audio from a video, specifying the format and quality.

```bash
# Extracts audio to an MP3 file with the highest VBR quality (0)
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "VIDEO_URL",
    "format_id": "140",
    "extract_audio": true,
    "audio_format": "mp3",
    "audio_quality": "0"
}' \
http://localhost:8080/download | jq
```

#### Scenario 3: Video Conversion (Remuxing)

Download and change the video container to MKV without re-encoding.

```bash
# Remuxes the downloaded video into an MKV container
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "VIDEO_URL",
    "format_id": "137+140",
    "remux_video": "mkv"
}' \
http://localhost:8080/download | jq
```

#### Scenario 4: Custom Output & Metadata

Use a custom file path template and save additional metadata files.

```bash
# Saves the video to "downloads/UploaderName/Video Title.mp4"
# Also saves the .info.json and thumbnail image in the same directory.
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "VIDEO_URL",
    "format_id": "22",
    "output_template": "downloads/%(uploader)s/%(title)s.%(ext)s",
    "write_info_json": true,
    "write_thumbnail": true
}' \
http://localhost:8080/download | jq
```

#### Scenario 5: Playlist Filtering

Download specific videos from a playlist.

```bash
# Downloads only the 1st, 5th, 6th, and 7th videos from the playlist
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "PLAYLIST_URL",
    "format_id": "22",
    "playlist_items": "1,5-7"
}' \
http://localhost:8080/download | jq
```

#### Scenario 6: Advanced Filtering

Download only videos that meet specific criteria, like duration or size.

```bash
# Downloads videos longer than 5 minutes (300s) with more than 10,000 views,
# and aborts if any file is larger than 100MB.
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "PLAYLIST_URL",
    "format_id": "best",
    "match_filter": "duration > 300 & view_count > 10000",
    "max_filesize": "100M"
}' \
http://localhost:8080/download | jq
```

#### Scenario 7: SponsorBlock Integration

Automatically remove sponsored segments and intros from the final video file.

```bash
# Downloads a video and removes all SponsorBlock categories except for the intro
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "SPONSOR_VIDEO_URL",
    "format_id": "bestvideo+bestaudio",
    "sponsorblock_remove": "all,-intro"
}' \
http://localhost:8080/download | jq
```

#### Scenario 8: The "Kitchen Sink" - Combining Multiple Options

A complex example showing how features can be combined.

```bash
# - Extracts audio to a high-quality FLAC file
# - Saves it to a custom path: "downloads/audio_archive/Uploader/Track Title.flac"
# - Embeds the thumbnail as album art
# - Skips sponsored segments
# - Restricts the filename to simple ASCII characters
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "MUSIC_VIDEO_URL",
    "format_id": "bestaudio",
    "extract_audio": true,
    "audio_format": "flac",
    "output_template": "downloads/audio_archive/%(uploader)s/%(track)s.%(ext)s",
    "embed_thumbnail": true,
    "sponsorblock_remove": "sponsor",
    "restrict_filenames": true
}' \
http://localhost:8080/download | jq
```

---

### 3. `GET /status` - Monitoring Downloads

Check the status of all downloads that have been initiated.

```bash
# Returns a JSON object with the status of all downloads keyed by their URL
curl http://localhost:8080/status | jq
```

---

### 4. `GET /files` and `GET /files/:path` - Managing Files

#### List All Downloaded Files

```bash
# Returns a JSON array of all file paths relative to the 'downloads' directory
curl http://localhost:8080/files | jq
```

#### Download a Specific File

Retrieve a file that was previously downloaded by the service.

```bash
# Example: Downloading a file that was saved in a subdirectory
# Note: The path is the part AFTER the base 'downloads' directory.
# The -o flag tells curl to save the output to a local file.
curl -o "My_Local_Video.mp4" http://localhost:8080/files/SomeUploader/Some%20Video%20Title.mp4

# If you saved an audio file from the "Kitchen Sink" example:
curl -o "local_song.flac" http://localhost:8080/files/audio_archive/ArtistName/Track%20Title.flac
```

### 1. Configuration Management (`/config`)

These new endpoints control the application's behavior, like the default download location.

#### Get the Current Configuration

Check the current settings. On the first run, this will show the default download directory for your operating system.

```bash
curl http://localhost:8080/config | jq
```

**Example Output (on Linux):**
```json
{
  "download_directory": "/home/your_user/Downloads"
}
```

#### Update the Configuration

Change the default download directory. This change is saved to a file and takes effect immediately.

```bash
curl -X POST -H "Content-Type: application/json" \
-d '{
    "download_directory": "/media/storage/my_videos"
}' \
http://localhost:8080/config | jq
```

---

### 2. Getting Video Information (`/formats`)

Before downloading, you usually need to see what formats are available to get a `format_id`.

```bash
# Replace with a real video URL
VIDEO_URL="https://www.youtube.com/watch?v=dQw4w9WgXcQ"

curl -G --data-urlencode "url=${VIDEO_URL}" http://localhost:8080/formats | jq
```

---

### 3. Starting a Download (`/download`)

This is the most powerful endpoint.

#### Scenario 1: Basic Download (Using New Configurable Default)

This is the key new behavior. By **not** providing an `output_template`, the video will be saved to the directory specified in your configuration.

```bash
# The video will be saved in "/media/storage/my_videos/Video Title [id].ext"
# (or whatever you set in /config)
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "VIDEO_URL",
    "format_id": "22"
}' \
http://localhost:8080/download | jq
```

#### Scenario 2: Download with Custom Path (Overriding the Default)

Provide an `output_template` to save the file to a specific location, ignoring the default from the config.

```bash
# Saves the file to a specific subdirectory with a custom name format.
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "VIDEO_URL",
    "format_id": "22",
    "output_template": "downloads/specific_project/%(uploader)s - %(title)s.%(ext)s"
}' \
http://localhost:8080/download | jq
```

#### Scenario 3: Audio Extraction

Download and convert a video to an MP3 file, saving it to the default download directory.

```bash
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "MUSIC_VIDEO_URL",
    "format_id": "bestaudio",
    "extract_audio": true,
    "audio_format": "mp3",
    "audio_quality": "0"
}' \
http://localhost:8080/download | jq
```

#### Scenario 4: Playlist and Filtering

Download only specific items from a playlist that match certain criteria.

```bash
# Downloads items 1 through 5 from a playlist, but only if their
# duration is less than 10 minutes (600 seconds).
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "PLAYLIST_URL",
    "format_id": "best",
    "playlist_items": "1-5",
    "match_filter": "duration < 600"
}' \
http://localhost:8080/download | jq
```

#### Scenario 5: SponsorBlock and Metadata

Remove sponsored segments and also save the video's `.info.json` file.

```bash
curl -X POST -H "Content-Type: application/json" \
-d '{
    "url": "VIDEO_URL_WITH_SPONSORS",
    "format_id": "bestvideo+bestaudio",
    "sponsorblock_remove": "sponsor,selfpromo",
    "write_info_json": true
}' \
http://localhost:8080/download | jq
```

---

### 4. Monitoring and Managing Files

#### Check Download Status (`/status`)

Get a real-time progress report of all active and completed downloads.

```bash
curl http://localhost:8080/status | jq```

#### List All Downloaded Files (`/files`)

This will list all files inside the directory you configured via the `/config` endpoint.

```bash
# If your download_directory is "/media/storage/my_videos", this lists its contents.
curl http://localhost:8080/files | jq
```

#### Download a Specific File (`/files/:path`)

Retrieve a file from the server. The path is relative to your configured `download_directory`.

```bash
# Suppose a file is at "/media/storage/my_videos/My Cool Video.mp4".
# The curl command uses the path relative to that base directory.
# The `-o` flag saves the output to a local file.
curl -o "local_copy.mp4" "http://localhost:8080/files/My Cool Video.mp4"
```
