use std::path::{Path, PathBuf};
use tokio::process::Command;

pub async fn generate_video_thumbnail(
    video_path: &Path,
    output_path: &Path,
) -> Result<PathBuf, String> {
    let out = output_path.to_path_buf();

    let status = Command::new("ffmpeg")
        .args([
            "-i",
            video_path.to_str().unwrap_or(""),
            "-ss",
            "00:00:01",
            "-vframes",
            "1",
            "-vf",
            "scale=320:-1",
            "-y",
            out.to_str().unwrap_or(""),
        ])
        .output()
        .await
        .map_err(|e| format!("ffmpeg not found or failed to execute: {}", e))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(format!("ffmpeg thumbnail failed: {}", stderr));
    }

    Ok(out)
}

pub async fn get_video_duration(video_path: &Path) -> Result<i32, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            video_path.to_str().unwrap_or(""),
        ])
        .output()
        .await
        .map_err(|e| format!("ffprobe failed: {}", e))?;

    if !output.status.success() {
        return Err("ffprobe returned error".into());
    }

    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str
        .trim()
        .parse::<f64>()
        .map(|d| d as i32)
        .map_err(|e| format!("Duration parse error: {}", e))
}

pub async fn transcode_video(
    input_path: &Path,
    output_path: &Path,
    format: &str,
) -> Result<PathBuf, String> {
    let out = output_path.to_path_buf();

    let codec = match format {
        "mp4" => "libx264",
        "webm" => "libvpx-vp9",
        _ => "libx264",
    };

    let status = Command::new("ffmpeg")
        .args([
            "-i",
            input_path.to_str().unwrap_or(""),
            "-c:v",
            codec,
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
            "-preset",
            "medium",
            "-crf",
            "23",
            "-y",
            out.to_str().unwrap_or(""),
        ])
        .output()
        .await
        .map_err(|e| format!("ffmpeg transcode failed: {}", e))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(format!("ffmpeg transcode error: {}", stderr));
    }

    Ok(out)
}
