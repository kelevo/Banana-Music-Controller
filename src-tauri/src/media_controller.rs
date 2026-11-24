use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Mutex as StdMutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionMediaProperties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
#[cfg(target_os = "windows")]
use windows::Storage::Streams::DataReader;

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn keybd_event(b_vk: u8, b_scan: u8, dw_flags: u32, dw_extra_info: usize);
    fn RoInitialize(init_type: i32) -> i32;
    fn RoUninitialize();
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MediaSourceApp {
    Spotify,
    Chrome,
    YouTube,
    Udemy,
    Safari,
    AppleMusic,
    MediaPlayer,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub title: String,
    pub artist: String,
    pub status: PlaybackStatus,
    pub source_app: MediaSourceApp,
    pub thumbnail_base64: Option<String>,
    pub accent_color: String,
    pub position_seconds: u32,
    pub duration_seconds: u32,
}

impl Default for MediaState {
    fn default() -> Self {
        Self {
            title: "Sin reproducción activa".to_string(),
            artist: "Windows Media".to_string(),
            status: PlaybackStatus::Stopped,
            source_app: MediaSourceApp::Unknown,
            thumbnail_base64: None,
            accent_color: "#1DB954".to_string(),
            position_seconds: 0,
            duration_seconds: 0,
        }
    }
}

#[derive(Clone)]
struct CachedTrackInfo {
    track_key: String,
    thumbnail_base64: Option<String>,
    accent_color: String,
}

static CURRENT_MEDIA_STATE: StdMutex<Option<MediaState>> = StdMutex::new(None);
static LAST_EMITTED_STATE: StdMutex<Option<MediaState>> = StdMutex::new(None);
static TRACK_CACHE: StdMutex<Option<CachedTrackInfo>> = StdMutex::new(None);
static RETRY_TRACK: StdMutex<Option<(String, u32)>> = StdMutex::new(None);
static IS_QUERYING: AtomicBool = AtomicBool::new(false);
static GLOBAL_APP_HANDLE: StdMutex<Option<AppHandle>> = StdMutex::new(None);

pub fn run_gsmtc_query_cli() {
    #[cfg(target_os = "windows")]
    {
        let (tx, rx) = channel::<MediaState>();

        std::thread::spawn(move || {
            const RO_INIT_MULTITHREADED: i32 = 1;
            unsafe {
                let _ = RoInitialize(RO_INIT_MULTITHREADED);
            }

            let state = read_session_snapshot();

            unsafe {
                RoUninitialize();
            }

            let _ = tx.send(state);
        });

        // Timeout generoso de 3.0 segundos para la lectura WinRT (evita dropouts con carátulas pesadas)
        let result_state = rx.recv_timeout(Duration::from_millis(3000)).unwrap_or_default();

        if let Ok(json) = serde_json::to_string(&result_state) {
            println!("{}", json);
        }
        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("{}", serde_json::to_string(&MediaState::default()).unwrap());
        std::process::exit(0);
    }
}

pub fn run_gsmtc_seek_cli(seconds: f64) {
    perform_gsmtc_seek(seconds);
    std::process::exit(0);
}

fn perform_gsmtc_seek(seconds: f64) {
    #[cfg(target_os = "windows")]
    {
        const RO_INIT_MULTITHREADED: i32 = 1;
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }

        if let Ok(mgr) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|op| op.join())
        {
            if let Ok(session) = mgr.GetCurrentSession() {
                let ticks = (seconds * 10_000_000.0) as i64;
                let _ = session.TryChangePlaybackPositionAsync(ticks).and_then(|op| op.join());
            }
        }

        unsafe {
            RoUninitialize();
        }
    }
}

#[cfg(target_os = "windows")]
fn send_media_key(vk: u8) {
    unsafe {
        const KEYEVENTF_KEYUP: u32 = 0x0002;
        keybd_event(vk, 0, 0, 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[tauri::command]
pub fn get_media_state() -> Result<MediaState, String> {
    if let Ok(guard) = CURRENT_MEDIA_STATE.lock() {
        if let Some(state) = &*guard {
            return Ok(state.clone());
        }
    }
    Ok(MediaState::default())
}

#[tauri::command]
pub fn media_play_pause() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    send_media_key(0xB3); // VK_MEDIA_PLAY_PAUSE

    #[cfg(target_os = "macos")]
    crate::media_controller_macos::send_play_pause_macos();

    trigger_decoupled_query_delayed(200);
    Ok(())
}

#[tauri::command]
pub fn media_next() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    send_media_key(0xB0); // VK_MEDIA_NEXT_TRACK

    #[cfg(target_os = "macos")]
    crate::media_controller_macos::send_next_macos();

    trigger_decoupled_query_delayed(220);
    Ok(())
}

#[tauri::command]
pub fn media_previous() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    send_media_key(0xB1); // VK_MEDIA_PREV_TRACK

    #[cfg(target_os = "macos")]
    crate::media_controller_macos::send_previous_macos();

    trigger_decoupled_query_delayed(220);
    Ok(())
}

#[tauri::command]
pub fn media_seek(position_seconds: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    crate::media_controller_macos::send_seek_macos(position_seconds);

    #[cfg(target_os = "windows")]
    std::thread::spawn(move || {
        let current_exe = std::env::current_exe();
        if let Ok(exe) = current_exe {
            #[cfg(target_os = "windows")]
            {
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                let _ = std::process::Command::new(exe)
                    .arg("--seek-gsmtc")
                    .arg(position_seconds.to_string())
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
            }
        } else {
            perform_gsmtc_seek(position_seconds);
        }
    });

    trigger_decoupled_query_delayed(200);
    Ok(())
}

fn trigger_decoupled_query_delayed(delay_ms: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(delay_ms));
        if let Ok(guard) = GLOBAL_APP_HANDLE.lock() {
            if let Some(app) = &*guard {
                trigger_decoupled_query(app.clone());
            }
        }
    });
}

pub fn trigger_decoupled_query(app_handle: AppHandle) {
    if IS_QUERYING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // Ya hay una consulta en curso, evitar ejecuciones concurrentes
        return;
    }

    std::thread::spawn(move || {
        let state = fetch_media_state_decoupled();

        // Actualizar estado global
        if let Ok(mut guard) = CURRENT_MEDIA_STATE.lock() {
            *guard = Some(state.clone());
        }

        // Deduplicación / Diff estricto de estado para no emitir ráfagas IPC
        let mut should_emit = false;
        if let Ok(mut guard) = LAST_EMITTED_STATE.lock() {
            if let Some(last) = &*guard {
                if state.title != last.title
                    || state.artist != last.artist
                    || state.status != last.status
                    || state.source_app != last.source_app
                    || state.thumbnail_base64 != last.thumbnail_base64
                    || state.accent_color != last.accent_color
                    || state.duration_seconds != last.duration_seconds
                    || (state.position_seconds as i64 - last.position_seconds as i64).abs() >= 2
                {
                    should_emit = true;
                    *guard = Some(state.clone());
                }
            } else {
                should_emit = true;
                *guard = Some(state.clone());
            }
        }

        if should_emit {
            let _ = app_handle.emit("media-state-changed", &state);
        }

        // Reintento progresivo de carátula para transiciones naturales en Spotify / navegadores:
        // Se ejecuta independientemente de `should_emit` para no interrumpir el ciclo si la carátula aún no está lista
        let track_key = format!("{}___{}", state.title, state.artist);
        let is_valid_playing_track = !state.title.trim().is_empty()
            && state.title != "Sin reproducción activa"
            && state.status == PlaybackStatus::Playing;

        if is_valid_playing_track && state.thumbnail_base64.is_none() {
            let mut retry_guard = RETRY_TRACK.lock().unwrap();
            let count = match &*retry_guard {
                Some((key, c)) if key == &track_key => *c,
                _ => 0,
            };

            if count < 5 {
                *retry_guard = Some((track_key.clone(), count + 1));
                let delay_ms = match count {
                    0 => 350,
                    1 => 750,
                    2 => 1400,
                    3 => 2200,
                    _ => 3200,
                };
                trigger_decoupled_query_delayed(delay_ms);
            }
        } else if state.thumbnail_base64.is_some() {
            if let Ok(mut retry_guard) = RETRY_TRACK.lock() {
                *retry_guard = None;
            }
        }

        IS_QUERYING.store(false, Ordering::SeqCst);
    });
}

fn fetch_media_state_decoupled() -> MediaState {
    #[cfg(target_os = "windows")]
    {
        let (tx, rx) = channel::<MediaState>();
        std::thread::spawn(move || {
            const RO_INIT_MULTITHREADED: i32 = 1;
            unsafe {
                let _ = RoInitialize(RO_INIT_MULTITHREADED);
            }
            let s = read_session_snapshot();
            unsafe {
                RoUninitialize();
            }
            let _ = tx.send(s);
        });

        let mut state = rx.recv_timeout(Duration::from_millis(2500)).unwrap_or_default();

        // Si la lectura actual no devolvió carátula pero la pista sigue siendo la misma,
        // preservar la carátula y color anteriores para evitar parpadeos
        if state.thumbnail_base64.is_none() {
            if let Ok(guard) = CURRENT_MEDIA_STATE.lock() {
                if let Some(prev) = &*guard {
                    if prev.title == state.title
                        && prev.artist == state.artist
                        && prev.thumbnail_base64.is_some()
                    {
                        state.thumbnail_base64 = prev.thumbnail_base64.clone();
                        state.accent_color = prev.accent_color.clone();
                    }
                }
            }
        }

        state
    }

    #[cfg(target_os = "macos")]
    {
        crate::media_controller_macos::fetch_media_snapshot_macos()
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    MediaState::default()
}

pub fn start_media_sync_loop(app_handle: AppHandle) {
    if let Ok(mut guard) = GLOBAL_APP_HANDLE.lock() {
        *guard = Some(app_handle.clone());
    }

    // Consulta inicial inmediata
    trigger_decoupled_query(app_handle.clone());

    // Temporizador de sincronización regular cada 1.5s para detectar transiciones ágilmente
    std::thread::Builder::new()
        .name("gsmtc-decoupled-timer".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(1500));
                trigger_decoupled_query(app_handle.clone());
            }
        })
        .expect("failed to spawn gsmtc-decoupled-timer");
}

#[cfg(target_os = "windows")]
fn select_best_windows_session(
    session_manager: &GlobalSystemMediaTransportControlsSessionManager,
) -> Option<GlobalSystemMediaTransportControlsSession> {
    if let Ok(sessions) = session_manager.GetSessions() {
        let count = sessions.Size().unwrap_or(0);
        if count > 0 {
            let mut playing_spotify = None;
            let mut playing_music = None;
            let mut playing_browser = None;
            let mut playing_other = None;

            let mut paused_spotify = None;
            let mut paused_music = None;
            let mut paused_browser = None;
            let mut paused_other = None;

            for i in 0..count {
                if let Ok(s) = sessions.GetAt(i) {
                    let app_id = s
                        .SourceAppUserModelId()
                        .map(|id| id.to_string().to_lowercase())
                        .unwrap_or_default();

                    let is_playing = s
                        .GetPlaybackInfo()
                        .and_then(|info| info.PlaybackStatus())
                        .map(|st| {
                            st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                        })
                        .unwrap_or(false);

                    let is_spotify = app_id.contains("spotify");
                    let is_music = app_id.contains("applemusic") || app_id.contains("itunes");
                    let is_browser = app_id.contains("chrome")
                        || app_id.contains("msedge")
                        || app_id.contains("edge")
                        || app_id.contains("firefox")
                        || app_id.contains("brave");

                    if is_playing {
                        if is_spotify && playing_spotify.is_none() {
                            playing_spotify = Some(s);
                        } else if is_music && playing_music.is_none() {
                            playing_music = Some(s);
                        } else if is_browser && playing_browser.is_none() {
                            playing_browser = Some(s);
                        } else if playing_other.is_none() {
                            playing_other = Some(s);
                        }
                    } else {
                        if is_spotify && paused_spotify.is_none() {
                            paused_spotify = Some(s);
                        } else if is_music && paused_music.is_none() {
                            paused_music = Some(s);
                        } else if is_browser && paused_browser.is_none() {
                            paused_browser = Some(s);
                        } else if paused_other.is_none() {
                            paused_other = Some(s);
                        }
                    }
                }
            }

            // 1. Spotify reproduciendo (Prioridad máxima)
            if let Some(s) = playing_spotify {
                return Some(s);
            }
            // 2. Apple Music reproduciendo
            if let Some(s) = playing_music {
                return Some(s);
            }
            // 3. Navegadores web reproduciendo
            if let Some(s) = playing_browser {
                return Some(s);
            }
            // 4. Otros reproductores activos
            if let Some(s) = playing_other {
                return Some(s);
            }

            // Si ninguno está reproduciendo, mantener prioridad entre sesiones pausadas:
            if let Some(s) = paused_spotify {
                return Some(s);
            }
            if let Some(s) = paused_music {
                return Some(s);
            }
            if let Some(s) = paused_browser {
                return Some(s);
            }
            if let Some(s) = paused_other {
                return Some(s);
            }
        }
    }

    session_manager.GetCurrentSession().ok()
}

#[cfg(target_os = "windows")]
fn read_session_snapshot() -> MediaState {
    let session_manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .and_then(|op| op.join())
    {
        Ok(mgr) => mgr,
        Err(_) => return MediaState::default(),
    };

    let session = match select_best_windows_session(&session_manager) {
        Some(s) => s,
        None => return MediaState::default(),
    };

    let mut state = MediaState::default();

    // 1. Identificar Source App
    if let Ok(app_id) = session.SourceAppUserModelId() {
        let app_str = app_id.to_string().to_lowercase();
        state.source_app = if app_str.contains("spotify") {
            MediaSourceApp::Spotify
        } else if app_str.contains("applemusic") || app_str.contains("itunes") {
            MediaSourceApp::AppleMusic
        } else if app_str.contains("chrome")
            || app_str.contains("msedge")
            || app_str.contains("edge")
            || app_str.contains("firefox")
            || app_str.contains("brave")
        {
            MediaSourceApp::Chrome
        } else if app_str.contains("youtube") {
            MediaSourceApp::YouTube
        } else if app_str.contains("udemy") {
            MediaSourceApp::Udemy
        } else if app_str.contains("zune")
            || app_str.contains("mediaplayer")
            || app_str.contains("vlc")
        {
            MediaSourceApp::MediaPlayer
        } else {
            MediaSourceApp::Unknown
        };
    }

    // 2. Playback Status
    if let Ok(playback_info) = session.GetPlaybackInfo() {
        if let Ok(status) = playback_info.PlaybackStatus() {
            state.status = match status {
                GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => {
                    PlaybackStatus::Playing
                }
                GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => {
                    PlaybackStatus::Paused
                }
                GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => {
                    PlaybackStatus::Stopped
                }
                GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => {
                    PlaybackStatus::Closed
                }
                _ => PlaybackStatus::Stopped,
            };
        }
    }

    // 3. Media Properties (Título, Artista, Carátula)
    if let Ok(media_props) = session
        .TryGetMediaPropertiesAsync()
        .and_then(|op| op.join())
    {
        if let Ok(t) = media_props.Title() {
            let title_str = t.to_string();
            if !title_str.trim().is_empty() {
                state.title = title_str;
            }
        }
        if let Ok(a) = media_props.Artist() {
            let artist_str = a.to_string();
            if !artist_str.trim().is_empty() {
                state.artist = artist_str;
            }
        }

        // Detección contextual de YouTube / Udemy si la app base es Chrome
        if state.source_app == MediaSourceApp::Chrome {
            let lower_title = state.title.to_lowercase();
            let lower_artist = state.artist.to_lowercase();
            if lower_title.contains("youtube") || lower_artist.contains("youtube") {
                state.source_app = MediaSourceApp::YouTube;
            } else if lower_title.contains("udemy") || lower_artist.contains("udemy") {
                state.source_app = MediaSourceApp::Udemy;
            }
        }

        let track_key = format!("{}___{}", state.title, state.artist);

        // Consultar caché para no volver a abrir streams de carátula
        let cached = {
            let guard = TRACK_CACHE.lock().unwrap();
            guard.clone()
        };

        if let Some(cached_info) = cached {
            if cached_info.track_key == track_key && !track_key.trim().is_empty() {
                if cached_info.thumbnail_base64.is_none() {
                    // Si antes fue None (ej. Spotify aún estaba descargando la carátula al cambiar de pista),
                    // reintentamos obtenerla ahora:
                    let (thumb_opt, color) = fetch_thumbnail_and_color(&media_props, &state.source_app);
                    if thumb_opt.is_some() {
                        state.thumbnail_base64 = thumb_opt.clone();
                        state.accent_color = color.clone();
                        let mut guard = TRACK_CACHE.lock().unwrap();
                        *guard = Some(CachedTrackInfo {
                            track_key,
                            thumbnail_base64: thumb_opt,
                            accent_color: color,
                        });
                    } else {
                        state.thumbnail_base64 = None;
                        state.accent_color = cached_info.accent_color;
                    }
                } else {
                    state.thumbnail_base64 = cached_info.thumbnail_base64;
                    state.accent_color = cached_info.accent_color;
                }
            } else if !track_key.trim().is_empty() && track_key != "Sin reproducción activa___Windows Media" {
                let (thumb_opt, color) = fetch_thumbnail_and_color(&media_props, &state.source_app);
                state.thumbnail_base64 = thumb_opt.clone();
                state.accent_color = color.clone();

                let mut guard = TRACK_CACHE.lock().unwrap();
                *guard = Some(CachedTrackInfo {
                    track_key,
                    thumbnail_base64: thumb_opt,
                    accent_color: color,
                });
            }
        } else if !track_key.trim().is_empty() && track_key != "Sin reproducción activa___Windows Media" {
            let (thumb_opt, color) = fetch_thumbnail_and_color(&media_props, &state.source_app);
            state.thumbnail_base64 = thumb_opt.clone();
            state.accent_color = color.clone();

            let mut guard = TRACK_CACHE.lock().unwrap();
            *guard = Some(CachedTrackInfo {
                track_key,
                thumbnail_base64: thumb_opt,
                accent_color: color,
            });
        }
    }

    // 4. Timeline
    if let Ok(timeline) = session.GetTimelineProperties() {
        if let Ok(pos) = timeline.Position() {
            state.position_seconds = (pos.Duration / 10_000_000).max(0) as u32;
        }
        if let Ok(end) = timeline.EndTime() {
            state.duration_seconds = (end.Duration / 10_000_000).max(0) as u32;
        }
    }

    if state.thumbnail_base64.is_none() {
        if let Ok(guard) = CURRENT_MEDIA_STATE.lock() {
            if let Some(prev) = &*guard {
                if prev.title == state.title && prev.artist == state.artist && prev.thumbnail_base64.is_some() {
                    state.thumbnail_base64 = prev.thumbnail_base64.clone();
                    state.accent_color = prev.accent_color.clone();
                }
            }
        }
    }

    if state.accent_color.is_empty() {
        state.accent_color = get_fallback_color(&state.source_app);
    }

    state
}

#[cfg(target_os = "windows")]
fn fetch_thumbnail_and_color(
    media_props: &GlobalSystemMediaTransportControlsSessionMediaProperties,
    source_app: &MediaSourceApp,
) -> (Option<String>, String) {
    let fallback_color = get_fallback_color(source_app);

    let thumb_ref = match media_props.Thumbnail() {
        Ok(tr) => tr,
        Err(_) => return (None, fallback_color),
    };

    let mut stream_res = thumb_ref.OpenReadAsync().and_then(|op| op.join());
    if stream_res.is_err() {
        // Reintento breve de 60ms por si Spotify está escribiendo la imagen en disco
        std::thread::sleep(Duration::from_millis(60));
        stream_res = thumb_ref.OpenReadAsync().and_then(|op| op.join());
    }

    let stream = match stream_res {
        Ok(s) => s,
        Err(_) => return (None, fallback_color),
    };

    let _ = stream.Seek(0);

    let size = match stream.Size() {
        Ok(sz) if sz > 0 && sz < 15_000_000 => sz,
        _ => return (None, fallback_color),
    };

    let reader = match DataReader::CreateDataReader(&stream) {
        Ok(r) => r,
        Err(_) => return (None, fallback_color),
    };

    let mut buffer = vec![0u8; size as usize];
    if let Ok(load_op) = reader.LoadAsync(size as u32) {
        if let Ok(loaded) = load_op.join() {
            if loaded as usize == size as usize {
                if reader.ReadBytes(&mut buffer).is_err() {
                    return (None, fallback_color);
                }
            } else if loaded > 0 {
                buffer.resize(loaded as usize, 0);
                if reader.ReadBytes(&mut buffer).is_err() {
                    return (None, fallback_color);
                }
            } else {
                return (None, fallback_color);
            }
        } else {
            return (None, fallback_color);
        }
    } else {
        return (None, fallback_color);
    }

    if buffer.is_empty() {
        return (None, fallback_color);
    }

    let mime = if buffer.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
        "image/png"
    } else if buffer.starts_with(&[255, 216]) {
        "image/jpeg"
    } else if buffer.starts_with(b"RIFF") && buffer.len() > 12 && &buffer[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/jpeg"
    };

    let data_uri = format!("data:{};base64,{}", mime, BASE64.encode(&buffer));
    let color = extract_dominant_color(&buffer);

    (Some(data_uri), color)
}

fn get_fallback_color(source_app: &MediaSourceApp) -> String {
    match source_app {
        MediaSourceApp::Spotify => "#1DB954".to_string(),
        MediaSourceApp::Chrome => "#F59E0B".to_string(),
        MediaSourceApp::YouTube => "#EF4444".to_string(),
        MediaSourceApp::Udemy => "#A435F0".to_string(),
        MediaSourceApp::Safari => "#007AFF".to_string(),
        MediaSourceApp::AppleMusic => "#FA243C".to_string(),
        MediaSourceApp::MediaPlayer => "#3B82F6".to_string(),
        MediaSourceApp::Unknown => "#1DB954".to_string(),
    }
}

fn extract_dominant_color(buffer: &[u8]) -> String {
    let mut best_score = 0.0f32;
    let mut best_color = (29u8, 185u8, 84u8);

    let step = (buffer.len() / 80).max(4);
    for i in (0..buffer.len().saturating_sub(3)).step_by(step) {
        let r = buffer[i] as f32;
        let g = buffer[i + 1] as f32;
        let b = buffer[i + 2] as f32;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        if max < 45.0 || min > 225.0 || delta < 35.0 {
            continue;
        }

        let saturation = delta / max;
        let brightness = max / 255.0;
        let score = saturation * 2.0 + brightness;

        if score > best_score {
            best_score = score;
            best_color = (r as u8, g as u8, b as u8);
        }
    }

    format!("#{:02X}{:02X}{:02X}", best_color.0, best_color.1, best_color.2)
}
