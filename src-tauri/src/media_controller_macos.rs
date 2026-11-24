use crate::media_controller::{MediaSourceApp, MediaState, PlaybackStatus};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::process::Command;
use std::sync::Mutex;

#[derive(Default)]
struct CachedTrackArt {
    key: String,
    base64_data: Option<String>,
}

static ART_CACHE: Mutex<Option<CachedTrackArt>> = Mutex::new(None);
static LAST_DETECTED_APP: Mutex<MediaSourceApp> = Mutex::new(MediaSourceApp::Unknown);
static LAST_ACTIVE_BROWSER: Mutex<String> = Mutex::new(String::new());

/// Comprueba de forma instantánea y nativa si un proceso está en ejecución
/// para evitar que AppleScript despierte o lance aplicaciones cerradas.
fn is_app_running(process_name: &str) -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg(process_name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Consulta el estado multimedia en macOS con prioridad estricta:
/// 1. Basado en qué app está REPRODUCIENDO (Playing):
///    - Si Spotify reproduce -> toma Spotify.
///    - Si Apple Music reproduce -> toma Apple Music.
///    - Si Safari / Chrome / Edge reproducen -> toma ese navegador.
///    - Si varios reproducen a la vez -> Prioridad absoluta a Spotify (reglas 2 y 3).
/// 2. Si ninguna reproduce activamente, muestra la primera con pista cargada (Paused).
/// 3. Si ninguna app multimedia está abierta, no ejecuta scripts y retorna default.
pub fn fetch_media_snapshot_macos() -> MediaState {
    let spotify_running = is_app_running("Spotify");
    let music_running = is_app_running("Music");
    let safari_running = is_app_running("Safari");
    let chrome_running = is_app_running("Google Chrome");
    let edge_running = is_app_running("Microsoft Edge");

    // Si NINGUNA app multimedia está corriendo, retornar vacío sin ejecutar scripts
    if !spotify_running && !music_running && !safari_running && !chrome_running && !edge_running {
        return MediaState::default();
    }

    let mut playing_candidates: Vec<MediaState> = Vec::new();
    let mut paused_candidates: Vec<MediaState> = Vec::new();

    // 1. Spotify nativo (Prioridad #1 si reproduce)
    if spotify_running {
        if let Some(state) = fetch_from_spotify() {
            if state.status == PlaybackStatus::Playing {
                playing_candidates.push(state);
            } else if state.status == PlaybackStatus::Paused {
                paused_candidates.push(state);
            }
        }
    }

    // 2. Apple Music (Prioridad #2 si reproduce)
    if music_running {
        if let Some(state) = fetch_from_apple_music() {
            if state.status == PlaybackStatus::Playing {
                playing_candidates.push(state);
            } else if state.status == PlaybackStatus::Paused {
                paused_candidates.push(state);
            }
        }
    }

    // 3. Safari
    if safari_running {
        if let Some(state) = fetch_from_safari() {
            if state.status == PlaybackStatus::Playing {
                playing_candidates.push(state);
            } else if state.status == PlaybackStatus::Paused {
                paused_candidates.push(state);
            }
        }
    }

    // 4. Google Chrome
    if chrome_running {
        if let Some(state) = fetch_from_chrome() {
            if state.status == PlaybackStatus::Playing {
                playing_candidates.push(state);
            } else if state.status == PlaybackStatus::Paused {
                paused_candidates.push(state);
            }
        }
    }

    // 5. Microsoft Edge
    if edge_running {
        if let Some(state) = fetch_from_edge() {
            if state.status == PlaybackStatus::Playing {
                playing_candidates.push(state);
            } else if state.status == PlaybackStatus::Paused {
                paused_candidates.push(state);
            }
        }
    }

    // A. Si alguna app está reproduciendo activamente (Playing), tomar la primera por jerarquía
    if let Some(active) = playing_candidates.into_iter().next() {
        if let Ok(mut guard) = LAST_DETECTED_APP.lock() {
            *guard = active.source_app.clone();
        }
        return active;
    }

    // B. Si ninguna reproduce pero hay alguna pausada, tomar la primera con pista cargada
    if let Some(paused) = paused_candidates.into_iter().next() {
        if let Ok(mut guard) = LAST_DETECTED_APP.lock() {
            *guard = paused.source_app.clone();
        }
        return paused;
    }

    MediaState::default()
}

/// Consulta segura para Spotify (solo si el proceso ya existe)
fn fetch_from_spotify() -> Option<MediaState> {
    let script = r#"
    try
        tell application "Spotify"
            set pState to player state as string
            if pState is "playing" or pState is "paused" then
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
    end try
    return "NONE"
    "#;

    parse_applescript_output(script, MediaSourceApp::Spotify, "#1DB954")
}

/// Consulta para Google Chrome buscando pestañas con contenido multimedia (YouTube, SoundCloud, Spotify Web, etc.)
fn fetch_from_chrome() -> Option<MediaState> {
    let script = r#"
    try
        tell application "Google Chrome"
            set pausedCandidate to ""
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    set tt to title of t
                    if u contains "youtube.com" or u contains "youtu.be" or u contains "music.youtube.com" or u contains "soundcloud.com" or u contains "open.spotify.com" or u contains "twitch.tv" or u contains "udemy.com" or u contains "vimeo.com" then
                        set isPlaying to "playing"
                        set curTime to 0
                        set totalDur to 0
                        try
                            set isPlaying to (tell t to execute javascript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                (v && !v.paused) ? 'playing' : 'paused';
                            ")
                            set curTime to (tell t to execute javascript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                v ? Math.floor(v.currentTime) : 0;
                            ") as integer
                            set totalDur to (tell t to execute javascript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                v ? Math.floor(v.duration) : 0;
                            ") as integer
                        end try
                        set itemStr to ("Chrome§" & isPlaying & "§" & tt & "§Web§" & curTime & "§" & totalDur & "§" & u)
                        if isPlaying is "playing" then
                            return itemStr
                        else if pausedCandidate is "" then
                            set pausedCandidate to itemStr
                        end if
                    end if
                end repeat
            end repeat
            if pausedCandidate is not "" then
                return pausedCandidate
            end if
        end tell
    end try
    return "NONE"
    "#;

    let res = parse_browser_media_output(script, MediaSourceApp::Chrome);
    if res.is_some() {
        if let Ok(mut guard) = LAST_ACTIVE_BROWSER.lock() {
            *guard = "chrome".to_string();
        }
    }
    res
}

/// Consulta para Microsoft Edge buscando pestañas multimedia
fn fetch_from_edge() -> Option<MediaState> {
    let script = r#"
    try
        tell application "Microsoft Edge"
            set pausedCandidate to ""
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    set tt to title of t
                    if u contains "youtube.com" or u contains "youtu.be" or u contains "music.youtube.com" or u contains "soundcloud.com" or u contains "open.spotify.com" or u contains "twitch.tv" or u contains "udemy.com" or u contains "vimeo.com" then
                        set isPlaying to "playing"
                        set curTime to 0
                        set totalDur to 0
                        try
                            set isPlaying to (tell t to execute javascript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                (v && !v.paused) ? 'playing' : 'paused';
                            ")
                            set curTime to (tell t to execute javascript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                v ? Math.floor(v.currentTime) : 0;
                            ") as integer
                            set totalDur to (tell t to execute javascript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                v ? Math.floor(v.duration) : 0;
                            ") as integer
                        end try
                        set itemStr to ("Edge§" & isPlaying & "§" & tt & "§Web§" & curTime & "§" & totalDur & "§" & u)
                        if isPlaying is "playing" then
                            return itemStr
                        else if pausedCandidate is "" then
                            set pausedCandidate to itemStr
                        end if
                    end if
                end repeat
            end repeat
            if pausedCandidate is not "" then
                return pausedCandidate
            end if
        end tell
    end try
    return "NONE"
    "#;

    let res = parse_browser_media_output(script, MediaSourceApp::Chrome);
    if res.is_some() {
        if let Ok(mut guard) = LAST_ACTIVE_BROWSER.lock() {
            *guard = "edge".to_string();
        }
    }
    res
}

/// Consulta para Safari buscando pestañas con contenido multimedia (YouTube, Udemy, SoundCloud, Twitch, etc.)
fn fetch_from_safari() -> Option<MediaState> {
    let script = r#"
    try
        tell application "Safari"
            set pausedCandidate to ""
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    set tt to name of t
                    if u contains "youtube.com" or u contains "youtu.be" or u contains "soundcloud.com" or u contains "udemy.com" or u contains "twitch.tv" or u contains "spotify.com" or u contains "vimeo.com" or u contains "music.apple.com" then
                        set isPlaying to "playing"
                        set curTime to 0
                        set totalDur to 0
                        try
                            set isPlaying to (do JavaScript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                (v && !v.paused) ? 'playing' : 'paused';
                            " in t)
                            set curTime to (do JavaScript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                v ? Math.floor(v.currentTime) : 0;
                            " in t) as integer
                            set totalDur to (do JavaScript "
                                var v = document.querySelector('video') || document.querySelector('audio');
                                v ? Math.floor(v.duration) : 0;
                            " in t) as integer
                        end try
                        set itemStr to ("Safari§" & isPlaying & "§" & tt & "§Web§" & curTime & "§" & totalDur & "§" & u)
                        if isPlaying is "playing" then
                            return itemStr
                        else if pausedCandidate is "" then
                            set pausedCandidate to itemStr
                        end if
                    end if
                end repeat
            end repeat
            if pausedCandidate is not "" then
                return pausedCandidate
            end if
        end tell
    end try
    return "NONE"
    "#;

    let res = parse_browser_media_output(script, MediaSourceApp::MediaPlayer);
    if res.is_some() {
        if let Ok(mut guard) = LAST_ACTIVE_BROWSER.lock() {
            *guard = "safari".to_string();
        }
    }
    res
}

/// Consulta para Apple Music defensiva (resistente a radios, streams y pistas sin metadatos completos)
fn fetch_from_apple_music() -> Option<MediaState> {
    let script = r#"
    try
        tell application "Music"
            set pState to (player state as string)
            if pState is "playing" or pState is "paused" then
                set trackName to ""
                set trackArtist to ""
                set trackPos to 0
                set trackDur to 0
                try
                    set trackName to name of current track
                end try
                try
                    set trackArtist to artist of current track
                end try
                try
                    set trackPos to player position as integer
                end try
                try
                    set trackDur to duration of current track as integer
                end try
                if trackName is not "" then
                    return "Music§" & pState & "§" & trackName & "§" & trackArtist & "§" & trackPos & "§" & trackDur & "§"
                end if
            end if
        end tell
    end try
    return "NONE"
    "#;

    parse_applescript_output(script, MediaSourceApp::MediaPlayer, "#FA243C")
}

fn parse_browser_media_output(script: &str, default_app: MediaSourceApp) -> Option<MediaState> {
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
    if parts.len() < 7 {
        return None;
    }

    let raw_status = parts[1].to_lowercase();
    let raw_title = parts[2].to_string();
    let pos: u32 = parts[4].parse().unwrap_or(0);
    let dur: u32 = parts[5].parse().unwrap_or(0);
    let media_url = parts[6].trim().to_string();

    let status = if raw_status.contains("play") {
        PlaybackStatus::Playing
    } else {
        PlaybackStatus::Paused
    };

    // Procesar título y artista de la pestaña (ej: "Queen - Bohemian Rhapsody - YouTube")
    let (mut title, mut artist) = clean_media_title(&raw_title);

    let mut source_app = default_app;
    let mut accent_color = "#EF4444".to_string(); // YouTube red
    let mut thumbnail_url: Option<String> = None;

    if media_url.contains("youtube.com") || media_url.contains("youtu.be") {
        source_app = MediaSourceApp::YouTube;
        accent_color = "#EF4444".to_string();
        if artist == "Desconocido" || artist.is_empty() {
            artist = "YouTube".to_string();
        }

        // Extraer video ID para la miniatura oficial en HD
        if let Some(video_id) = extract_youtube_video_id(&media_url) {
            thumbnail_url = Some(format!("https://img.youtube.com/vi/{}/hqdefault.jpg", video_id));
        }
    } else if media_url.contains("spotify.com") {
        source_app = MediaSourceApp::Spotify;
        accent_color = "#1DB954".to_string();
    } else if media_url.contains("udemy.com") {
        source_app = MediaSourceApp::Udemy;
        accent_color = "#A435F0".to_string();
    } else if media_url.contains("soundcloud.com") {
        accent_color = "#FF5500".to_string();
        if artist.is_empty() {
            artist = "SoundCloud".to_string();
        }
    }

    if title.is_empty() {
        title = "Pestaña multimedia".to_string();
    }

    let track_key = format!("{}___{}", title, artist);
    let mut thumbnail_base64: Option<String> = None;

    // Verificar caché de carátula
    {
        if let Ok(guard) = ART_CACHE.lock() {
            if let Some(cached) = &*guard {
                if cached.key == track_key {
                    thumbnail_base64 = cached.base64_data.clone();
                }
            }
        }
    }

    // Descargar miniatura si es una URL y no estaba en caché
    if thumbnail_base64.is_none() {
        if let Some(thumb_url) = thumbnail_url {
            if let Ok(curl_res) = Command::new("curl")
                .arg("-s")
                .arg("-m")
                .arg("2")
                .arg(&thumb_url)
                .output()
            {
                if curl_res.status.success() && !curl_res.stdout.is_empty() {
                    let bytes = curl_res.stdout;
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
    }

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

fn parse_applescript_output(script: &str, app: MediaSourceApp, color: &str) -> Option<MediaState> {
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

    let raw_status = parts[1].to_lowercase();
    let title = parts[2].to_string();
    let artist = parts[3].to_string();
    let pos: u32 = parts[4].parse().unwrap_or(0);
    let dur: u32 = parts[5].parse().unwrap_or(0);
    let art_url = if parts.len() > 6 { parts[6].trim() } else { "" };

    let status = if raw_status.contains("play") {
        PlaybackStatus::Playing
    } else {
        PlaybackStatus::Paused
    };

    let track_key = format!("{}___{}", title, artist);
    let mut thumbnail_base64: Option<String> = None;

    {
        if let Ok(guard) = ART_CACHE.lock() {
            if let Some(cached) = &*guard {
                if cached.key == track_key {
                    thumbnail_base64 = cached.base64_data.clone();
                }
            }
        }
    }

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

    Some(MediaState {
        title,
        artist,
        status,
        source_app: app,
        thumbnail_base64,
        accent_color: color.to_string(),
        position_seconds: pos,
        duration_seconds: dur,
    })
}

/// Extrae y limpia el Título y el Artista desde el título de la pestaña del navegador
fn clean_media_title(raw: &str) -> (String, String) {
    let mut clean = raw.to_string();

    // Eliminar sufijos comunes de plataformas web
    for suffix in &[
        " - YouTube",
        " - YouTube Music",
        " | SoundCloud",
        " | Spotify",
        " - Twitch",
    ] {
        if let Some(idx) = clean.rfind(suffix) {
            clean.truncate(idx);
        }
    }

    // Eliminar etiquetas de video oficial
    for tag in &[
        "(Official Music Video)",
        "(Official Video)",
        "(Video Oficial)",
        "(Official Audio)",
        "(Audio Oficial)",
        "(Visualizer)",
        "(Lyric Video)",
        "[Official Music Video]",
        "[Official Video]",
    ] {
        clean = clean.replace(tag, "");
    }

    let clean_trimmed = clean.trim();

    // Separar "Artista - Título" si contiene un guion divisor
    if let Some((left, right)) = clean_trimmed.split_once(" - ") {
        (right.trim().to_string(), left.trim().to_string())
    } else {
        (clean_trimmed.to_string(), "Desconocido".to_string())
    }
}

/// Extrae el ID del video de YouTube a partir de una URL
fn extract_youtube_video_id(url: &str) -> Option<String> {
    if let Some(pos) = url.find("v=") {
        let after_v = &url[pos + 2..];
        let id: String = after_v.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
        if !id.is_empty() {
            return Some(id);
        }
    } else if let Some(pos) = url.find("youtu.be/") {
        let after = &url[pos + 9..];
        let id: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
        if !id.is_empty() {
            return Some(id);
        }
    }
    None
}

/// Envía Play/Pause universal e inteligente en macOS (solo a apps en ejecución)
pub fn send_play_pause_macos() {
    let spotify_running = is_app_running("Spotify");
    let music_running = is_app_running("Music");
    let safari_running = is_app_running("Safari");
    let chrome_running = is_app_running("Google Chrome");
    let edge_running = is_app_running("Microsoft Edge");

    // Si NINGUNA app multimedia está corriendo, no hacer nada (evita auto-aperturas)
    if !spotify_running && !music_running && !safari_running && !chrome_running && !edge_running {
        return;
    }

    let app = {
        let guard = LAST_DETECTED_APP.lock().unwrap();
        guard.clone()
    };
    let active_browser = {
        let guard = LAST_ACTIVE_BROWSER.lock().unwrap();
        guard.clone()
    };

    match app {
        MediaSourceApp::Spotify if spotify_running => {
            let script = r#"
            try
                tell application "Spotify" to playpause
            end try
            "#;
            let _ = Command::new("osascript").arg("-e").arg(script).output();
            return;
        }
        MediaSourceApp::MediaPlayer if music_running => {
            let script = r#"
            try
                tell application "Music" to playpause
            end try
            "#;
            let _ = Command::new("osascript").arg("-e").arg(script).output();
            return;
        }
        MediaSourceApp::YouTube | MediaSourceApp::Chrome | MediaSourceApp::Udemy => {
            if active_browser == "safari" && safari_running {
                send_safari_play_pause();
                return;
            } else if active_browser == "edge" && edge_running {
                send_edge_play_pause();
                return;
            } else if chrome_running {
                send_chrome_play_pause();
                return;
            }
        }
        _ => {}
    }

    // Fallback: si no había app detectada específica pero alguna está abierta, enviar a la que esté abierta en orden de prioridad
    if spotify_running {
        let _ = Command::new("osascript").arg("-e").arg(r#"
        try
            tell application "Spotify" to playpause
        end try
        "#).output();
    } else if music_running {
        let _ = Command::new("osascript").arg("-e").arg(r#"
        try
            tell application "Music" to playpause
        end try
        "#).output();
    } else if safari_running {
        send_safari_play_pause();
    } else if chrome_running {
        send_chrome_play_pause();
    } else if edge_running {
        send_edge_play_pause();
    }
}

/// Envía Siguiente pista en macOS (solo a apps en ejecución)
pub fn send_next_macos() {
    let spotify_running = is_app_running("Spotify");
    let music_running = is_app_running("Music");
    let safari_running = is_app_running("Safari");
    let chrome_running = is_app_running("Google Chrome");
    let edge_running = is_app_running("Microsoft Edge");

    if !spotify_running && !music_running && !safari_running && !chrome_running && !edge_running {
        return;
    }

    let app = {
        let guard = LAST_DETECTED_APP.lock().unwrap();
        guard.clone()
    };
    let active_browser = {
        let guard = LAST_ACTIVE_BROWSER.lock().unwrap();
        guard.clone()
    };

    match app {
        MediaSourceApp::Spotify if spotify_running => {
            let script = r#"
            try
                tell application "Spotify" to next track
            end try
            "#;
            let _ = Command::new("osascript").arg("-e").arg(script).output();
            return;
        }
        MediaSourceApp::MediaPlayer if music_running => {
            let script = r#"
            try
                tell application "Music" to next track
            end try
            "#;
            let _ = Command::new("osascript").arg("-e").arg(script).output();
            return;
        }
        MediaSourceApp::YouTube | MediaSourceApp::Chrome | MediaSourceApp::Udemy => {
            if active_browser == "safari" && safari_running {
                send_safari_next();
                return;
            } else if active_browser == "edge" && edge_running {
                send_edge_next();
                return;
            } else if chrome_running {
                send_chrome_next();
                return;
            }
        }
        _ => {}
    }

    if spotify_running {
        let _ = Command::new("osascript").arg("-e").arg(r#"
        try
            tell application "Spotify" to next track
        end try
        "#).output();
    } else if music_running {
        let _ = Command::new("osascript").arg("-e").arg(r#"
        try
            tell application "Music" to next track
        end try
        "#).output();
    } else if safari_running {
        send_safari_next();
    } else if chrome_running {
        send_chrome_next();
    } else if edge_running {
        send_edge_next();
    }
}

/// Envía Pista anterior en macOS (solo a apps en ejecución)
pub fn send_previous_macos() {
    let spotify_running = is_app_running("Spotify");
    let music_running = is_app_running("Music");
    let safari_running = is_app_running("Safari");
    let chrome_running = is_app_running("Google Chrome");
    let edge_running = is_app_running("Microsoft Edge");

    if !spotify_running && !music_running && !safari_running && !chrome_running && !edge_running {
        return;
    }

    let app = {
        let guard = LAST_DETECTED_APP.lock().unwrap();
        guard.clone()
    };
    let active_browser = {
        let guard = LAST_ACTIVE_BROWSER.lock().unwrap();
        guard.clone()
    };

    match app {
        MediaSourceApp::Spotify if spotify_running => {
            let script = r#"
            try
                tell application "Spotify" to previous track
            end try
            "#;
            let _ = Command::new("osascript").arg("-e").arg(script).output();
            return;
        }
        MediaSourceApp::MediaPlayer if music_running => {
            let script = r#"
            try
                tell application "Music" to previous track
            end try
            "#;
            let _ = Command::new("osascript").arg("-e").arg(script).output();
            return;
        }
        MediaSourceApp::YouTube | MediaSourceApp::Chrome | MediaSourceApp::Udemy => {
            if active_browser == "safari" && safari_running {
                send_safari_prev();
                return;
            } else if active_browser == "edge" && edge_running {
                send_edge_prev();
                return;
            } else if chrome_running {
                send_chrome_prev();
                return;
            }
        }
        _ => {}
    }

    if spotify_running {
        let _ = Command::new("osascript").arg("-e").arg(r#"
        try
            tell application "Spotify" to previous track
        end try
        "#).output();
    } else if music_running {
        let _ = Command::new("osascript").arg("-e").arg(r#"
        try
            tell application "Music" to previous track
        end try
        "#).output();
    } else if safari_running {
        send_safari_prev();
    } else if chrome_running {
        send_chrome_prev();
    } else if edge_running {
        send_edge_prev();
    }
}

/// Envía Seek a posición en segundos en macOS (solo a apps en ejecución)
pub fn send_seek_macos(seconds: f64) {
    let sec_int = seconds.max(0.0) as u32;
    let spotify_running = is_app_running("Spotify");
    let music_running = is_app_running("Music");
    let safari_running = is_app_running("Safari");
    let chrome_running = is_app_running("Google Chrome");
    let edge_running = is_app_running("Microsoft Edge");

    if !spotify_running && !music_running && !safari_running && !chrome_running && !edge_running {
        return;
    }

    let app = {
        let guard = LAST_DETECTED_APP.lock().unwrap();
        guard.clone()
    };
    let active_browser = {
        let guard = LAST_ACTIVE_BROWSER.lock().unwrap();
        guard.clone()
    };

    match app {
        MediaSourceApp::Spotify if spotify_running => {
            let script = format!(
                r#"
                try
                    tell application "Spotify" to set player position to {}
                end try
                "#,
                sec_int
            );
            let _ = Command::new("osascript").arg("-e").arg(&script).output();
        }
        MediaSourceApp::MediaPlayer if music_running => {
            let script = format!(
                r#"
                try
                    tell application "Music" to set player position to {}
                end try
                "#,
                sec_int
            );
            let _ = Command::new("osascript").arg("-e").arg(&script).output();
        }
        MediaSourceApp::YouTube | MediaSourceApp::Chrome | MediaSourceApp::Udemy => {
            if active_browser == "safari" && safari_running {
                let script = format!(
                    r#"
                    try
                        tell application "Safari"
                            repeat with w in windows
                                repeat with t in tabs of w
                                    set u to URL of t
                                    if u contains "youtube.com" or u contains "youtu.be" or u contains "soundcloud.com" or u contains "udemy.com" then
                                        try
                                            do JavaScript "
                                                var v = document.querySelector('video') || document.querySelector('audio');
                                                if (v) v.currentTime = {};
                                            " in t
                                        end try
                                        return
                                    end if
                                end repeat
                            end repeat
                        end tell
                    end try
                    "#,
                    sec_int
                );
                let _ = Command::new("osascript").arg("-e").arg(&script).output();
            } else if active_browser == "edge" && edge_running {
                let script = format!(
                    r#"
                    try
                        tell application "Microsoft Edge"
                            repeat with w in windows
                                repeat with t in tabs of w
                                    set u to URL of t
                                    if u contains "youtube.com" or u contains "music.youtube" or u contains "soundcloud.com" or u contains "udemy.com" then
                                        tell t to execute javascript "
                                            var v = document.querySelector('video') || document.querySelector('audio');
                                            if (v) v.currentTime = {};
                                        "
                                        return
                                    end if
                                end repeat
                            end repeat
                        end tell
                    end try
                    "#,
                    sec_int
                );
                let _ = Command::new("osascript").arg("-e").arg(&script).output();
            } else if chrome_running {
                let script = format!(
                    r#"
                    try
                        tell application "Google Chrome"
                            repeat with w in windows
                                repeat with t in tabs of w
                                    set u to URL of t
                                    if u contains "youtube.com" or u contains "music.youtube" or u contains "soundcloud.com" or u contains "udemy.com" then
                                        tell t to execute javascript "
                                            var v = document.querySelector('video') || document.querySelector('audio');
                                            if (v) v.currentTime = {};
                                        "
                                        return
                                    end if
                                end repeat
                            end repeat
                        end tell
                    end try
                    "#,
                    sec_int
                );
                let _ = Command::new("osascript").arg("-e").arg(&script).output();
            }
        }
        _ => {}
    }
}

fn send_safari_play_pause() {
    let script = r#"
    try
        tell application "Safari"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" or u contains "youtu.be" or u contains "soundcloud.com" or u contains "spotify.com" or u contains "udemy.com" or u contains "twitch.tv" or u contains "vimeo.com" then
                        try
                            do JavaScript "
                                var btn = document.querySelector('.ytp-play-button');
                                if (btn) {
                                    btn.click();
                                } else {
                                    var v = document.querySelector('video') || document.querySelector('audio');
                                    if (v) { v.paused ? v.play() : v.pause(); }
                                }
                            " in t
                        end try
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_safari_next() {
    let script = r#"
    try
        tell application "Safari"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" or u contains "youtu.be" then
                        try
                            do JavaScript "
                                var nextBtn = document.querySelector('.ytp-next-button');
                                if (nextBtn) nextBtn.click();
                            " in t
                        end try
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_safari_prev() {
    let script = r#"
    try
        tell application "Safari"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" or u contains "youtu.be" then
                        try
                            do JavaScript "
                                var prevBtn = document.querySelector('.ytp-prev-button');
                                if (prevBtn) {
                                    prevBtn.click();
                                } else {
                                    var v = document.querySelector('video');
                                    if (v) v.currentTime = 0;
                                }
                            " in t
                        end try
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_chrome_play_pause() {
    let script = r#"
    try
        tell application "Google Chrome"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" or u contains "music.youtube" or u contains "soundcloud.com" or u contains "spotify.com" or u contains "udemy.com" then
                        tell t to execute javascript "
                            var btn = document.querySelector('.ytp-play-button');
                            if (btn) {
                                btn.click();
                            } else {
                                var v = document.querySelector('video') || document.querySelector('audio');
                                if (v) { v.paused ? v.play() : v.pause(); }
                            }
                        "
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_chrome_next() {
    let script = r#"
    try
        tell application "Google Chrome"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" then
                        tell t to execute javascript "
                            var nextBtn = document.querySelector('.ytp-next-button');
                            if (nextBtn) nextBtn.click();
                        "
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_chrome_prev() {
    let script = r#"
    try
        tell application "Google Chrome"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" then
                        tell t to execute javascript "
                            var prevBtn = document.querySelector('.ytp-prev-button');
                            if (prevBtn) {
                                prevBtn.click();
                            } else {
                                var v = document.querySelector('video');
                                if (v) v.currentTime = 0;
                            }
                        "
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_edge_play_pause() {
    let script = r#"
    try
        tell application "Microsoft Edge"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" or u contains "music.youtube" or u contains "soundcloud.com" or u contains "spotify.com" or u contains "udemy.com" then
                        tell t to execute javascript "
                            var btn = document.querySelector('.ytp-play-button');
                            if (btn) {
                                btn.click();
                            } else {
                                var v = document.querySelector('video') || document.querySelector('audio');
                                if (v) { v.paused ? v.play() : v.pause(); }
                            }
                        "
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_edge_next() {
    let script = r#"
    try
        tell application "Microsoft Edge"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" then
                        tell t to execute javascript "
                            var nextBtn = document.querySelector('.ytp-next-button');
                            if (nextBtn) nextBtn.click();
                        "
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}

fn send_edge_prev() {
    let script = r#"
    try
        tell application "Microsoft Edge"
            repeat with w in windows
                repeat with t in tabs of w
                    set u to URL of t
                    if u contains "youtube.com" then
                        tell t to execute javascript "
                            var prevBtn = document.querySelector('.ytp-prev-button');
                            if (prevBtn) {
                                prevBtn.click();
                            } else {
                                var v = document.querySelector('video');
                                if (v) v.currentTime = 0;
                            }
                        "
                        return
                    end if
                end repeat
            end repeat
        end tell
    end try
    "#;
    let _ = Command::new("osascript").arg("-e").arg(script).output();
}
