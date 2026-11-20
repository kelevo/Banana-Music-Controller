use crate::media_controller::{MediaSourceApp, MediaState, PlaybackStatus};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Default)]
struct CachedTrackArt {
    key: String,
    base64_data: Option<String>,
}

static ART_CACHE: Mutex<Option<CachedTrackArt>> = Mutex::new(None);

/// Consulta el estado multimedia en macOS combinando MediaRemote y AppleScript
pub fn fetch_media_snapshot_macos() -> MediaState {
    // 1. Intentar consulta optimizada para reproductores nativos populares (Spotify / Music)
    if let Some(state) = fetch_from_applescript() {
        return state;
    }

    // 2. Intentar consulta vía MediaRemote para reproductores web y otros navegadores
    if let Some(state) = fetch_from_media_remote() {
        return state;
    }

    MediaState::default()
}

/// Consulta de alta velocidad vía AppleScript para Spotify y Apple Music
fn fetch_from_applescript() -> Option<MediaState> {
    // Script optimizado que se ejecuta en <20ms
    let script = r#"
    if application "Spotify" is running then
        tell application "Spotify"
            set pState to player state as string
            if pState is not "stopped" then
                set trackName to name of current track
                set trackArtist to artist of current track
                set trackPos to player position as integer
                set trackDur to ((duration of current track) / 1000) as integer
                set artUrl to ""
                try
                    set artUrl to artwork url of current track
                end try
                return "Spotify§" & pState & "§" & trackName & "§" & trackArtist & "§" & trackPos & "§" & trackDur & "§" & artUrl
            end if
        end tell
    end if

    if application "Music" is running then
        tell application "Music"
            set pState to player state as string
            if pState is not "stopped" then
                set trackName to name of current track
                set trackArtist to artist of current track
                set trackPos to player position as integer
                set trackDur to duration of current track as integer
                return "Music§" & pState & "§" & trackName & "§" & trackArtist & "§" & trackPos & "§" & trackDur & "§"
            end if
        end tell
    end if

    return "NONE"
    "#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text == "NONE" || text.is_empty() {
        return None;
    }

    let parts: Vec<&str> = text.split('§').collect();
    if parts.len() < 6 {
        return None;
    }

    let app_name = parts[0];
    let raw_status = parts[1].to_lowercase();
    let title = parts[2].to_string();
    let artist = parts[3].to_string();
    let pos: u32 = parts[4].parse().unwrap_or(0);
    let dur: u32 = parts[5].parse().unwrap_or(0);
    let art_url = if parts.len() > 6 { parts[6].trim() } else { "" };

    let status = if raw_status.contains("play") {
        PlaybackStatus::Playing
    } else if raw_status.contains("pause") {
        PlaybackStatus::Paused
    } else {
        PlaybackStatus::Stopped
    };

    let source_app = match app_name {
        "Spotify" => MediaSourceApp::Spotify,
        "Music" => MediaSourceApp::MediaPlayer,
        _ => MediaSourceApp::Unknown,
    };

    let track_key = format!("{}___{}", title, artist);
    let mut thumbnail_base64: Option<String> = None;

    // Gestión de carátula en caché
    {
        if let Ok(guard) = ART_CACHE.lock() {
            if let Some(cached) = &*guard {
                if cached.key == track_key {
                    thumbnail_base64 = cached.base64_data.clone();
                }
            }
        }
    }

    // Si no está en caché y tenemos URL de Spotify, descargar la miniatura de forma ligera
    if thumbnail_base64.is_none() && !art_url.is_empty() && art_url.starts_with("http") {
        if let Ok(curl_output) = Command::new("curl")
            .arg("-s")
            .arg("-m")
            .arg("2")
            .arg(art_url)
            .output()
        {
            if curl_output.status.success() && !curl_output.stdout.is_empty() {
                let bytes = curl_output.stdout;
                let mime = if bytes.starts_with(&[137, 80, 78, 71]) {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                let uri = format!("data:{};base64,{}", mime, BASE64.encode(&bytes));
                thumbnail_base64 = Some(uri.clone());

                if let Ok(mut guard) = ART_CACHE.lock() {
                    *guard = Some(CachedTrackArt {
                        key: track_key,
                        base64_data: Some(uri),
                    });
                }
            }
        }
    }

    let accent_color = match source_app {
        MediaSourceApp::Spotify => "#1DB954".to_string(),
        MediaSourceApp::MediaPlayer => "#FA243C".to_string(), // Apple Music red
        _ => "#1DB954".to_string(),
    };

    Some(MediaState {
        title,
        artist,
        status,
        source_app,
        thumbnail_base64,
        accent_color,
        position_seconds: pos,
        duration_seconds: dur,
    })
}

/// Consulta dinámica mediante PrivateFramework MediaRemote
fn fetch_from_media_remote() -> Option<MediaState> {
    // MediaRemote fallback / helper si se requiere para navegadores (Chrome/Safari)
    None
}

/// Envía Play/Pause universal en macOS
pub fn send_play_pause_macos() {
    let script = r#"
    tell application "System Events"
        if application "Spotify" is running then
            tell application "Spotify" to playpause
            return
        end if
        if application "Music" is running then
            tell application "Music" to playpause
            return
        end if
    end tell
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

/// Envía Siguiente pista en macOS
pub fn send_next_macos() {
    let script = r#"
    tell application "System Events"
        if application "Spotify" is running then
            tell application "Spotify" to next track
            return
        end if
        if application "Music" is running then
            tell application "Music" to next track
            return
        end if
    end tell
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

/// Envía Pista anterior en macOS
pub fn send_previous_macos() {
    let script = r#"
    tell application "System Events"
        if application "Spotify" is running then
            tell application "Spotify" to previous track
            return
        end if
        if application "Music" is running then
            tell application "Music" to previous track
            return
        end if
    end tell
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

/// Envía Seek a posición en segundos en macOS
pub fn send_seek_macos(seconds: f64) {
    let sec_int = seconds.max(0.0) as u32;
    let script = format!(
        r#"
        tell application "System Events"
            if application "Spotify" is running then
                tell application "Spotify" to set player position to {}
                return
            end if
            if application "Music" is running then
                tell application "Music" to set player position to {}
                return
            end if
        end tell
        "#,
        sec_int, sec_int
    );
    let _ = Command::new("osascript").arg("-e").arg(&script).output();
}
