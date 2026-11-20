use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioBands {
    pub bass: f32,    // 0.0 a 1.0 (sub-bajos y bombo)
    pub mid: f32,     // 0.0 a 1.0 (voces y melodías)
    pub treble: f32,  // 0.0 a 1.0 (agudos y platillos)
}

impl Default for AudioBands {
    fn default() -> Self {
        Self {
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
        }
    }
}

static IS_VISUALIZER_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start_audio_visualizer(app_handle: AppHandle) {
    if IS_VISUALIZER_RUNNING.swap(true, Ordering::SeqCst) {
        return; // Ya está en ejecución
    }

    #[cfg(target_os = "windows")]
    {
        std::thread::Builder::new()
            .name("wasapi-audio-visualizer".to_string())
            .spawn(move || {
                run_wasapi_loopback_loop(app_handle);
            })
            .expect("Failed to spawn wasapi-audio-visualizer thread");
    }

    #[cfg(target_os = "macos")]
    {
        std::thread::Builder::new()
            .name("macos-audio-visualizer".to_string())
            .spawn(move || {
                run_macos_visualizer_loop(app_handle);
            })
            .expect("Failed to spawn macos-audio-visualizer thread");
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let _ = app_handle;
    }
}

#[cfg(target_os = "macos")]
fn run_macos_visualizer_loop(app_handle: AppHandle) {
    let mut smoothed_bass = 0.0f32;
    let mut smoothed_mid = 0.0f32;
    let mut smoothed_treble = 0.0f32;

    let mut last_emit = Instant::now();
    let emit_interval = Duration::from_millis(28); // ~35 Hz
    let mut phase: f32 = 0.0;

    while IS_VISUALIZER_RUNNING.load(Ordering::SeqCst) {
        // Consultar estado de reproducción para reactividad armónica
        let is_playing = if let Ok(state) = crate::media_controller::get_media_state() {
            state.status == crate::media_controller::PlaybackStatus::Playing
        } else {
            false
        };

        if is_playing {
            phase += 0.14;
            if phase > std::f32::consts::PI * 2.0 {
                phase -= std::f32::consts::PI * 2.0;
            }

            // Pulso dinámico que modula bombo/bajo y frecuencias medias
            let beat = (phase.sin().powi(2) * 0.75 + (phase * 2.0).cos().abs() * 0.25).clamp(0.0, 1.0);
            let raw_bass = (beat * 0.88).min(1.0);
            let raw_mid = (beat * 0.62).min(1.0);
            let raw_treble = ((phase * 3.0).sin().abs() * 0.45).min(1.0);

            let b_rate = if raw_bass > smoothed_bass { 0.85 } else { 0.16 };
            let m_rate = if raw_mid > smoothed_mid { 0.70 } else { 0.18 };
            let t_rate = if raw_treble > smoothed_treble { 0.70 } else { 0.18 };

            smoothed_bass += (raw_bass - smoothed_bass) * b_rate;
            smoothed_mid += (raw_mid - smoothed_mid) * m_rate;
            smoothed_treble += (raw_treble - smoothed_treble) * t_rate;
        } else {
            // Decaimiento natural al pausar o parar
            smoothed_bass *= 0.80;
            smoothed_mid *= 0.80;
            smoothed_treble *= 0.80;
        }

        if last_emit.elapsed() >= emit_interval {
            last_emit = Instant::now();

            let b_val = if smoothed_bass < 0.015 { 0.0 } else { smoothed_bass.min(1.0) };
            let m_val = if smoothed_mid < 0.015 { 0.0 } else { smoothed_mid.min(1.0) };
            let t_val = if smoothed_treble < 0.015 { 0.0 } else { smoothed_treble.min(1.0) };

            let bands = AudioBands {
                bass: (b_val * 100.0).round() / 100.0,
                mid: (m_val * 100.0).round() / 100.0,
                treble: (t_val * 100.0).round() / 100.0,
            };

            let _ = app_handle.emit("audio-bands-update", &bands);
        }

        std::thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(target_os = "windows")]
fn run_wasapi_loopback_loop(app_handle: AppHandle) {
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    unsafe {
        // Inicializar COM en MTA para el hilo dedicado de captura
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let mut smoothed_bass = 0.0f32;
    let mut smoothed_mid = 0.0f32;
    let mut smoothed_treble = 0.0f32;

    let mut last_emit = Instant::now();
    let emit_interval = Duration::from_millis(28); // ~35 Hz para un flujo IPC suave y ligero

    loop {
        let capture_result = (|| -> windows::core::Result<()> {
            unsafe {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

                let device: IMMDevice =
                    enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;

                let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

                let format_ptr = client.GetMixFormat()?;
                let format = *format_ptr;

                let channels = format.nChannels as usize;
                let sample_rate = format.nSamplesPerSec as f32;
                let is_float = format.wFormatTag == 3 // WAVE_FORMAT_IEEE_FLOAT
                    || format.wBitsPerSample == 32;

                // Buffer de 100ms
                client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    1_000_000,
                    0,
                    format_ptr,
                    None,
                )?;

                let capture_client: IAudioCaptureClient = client.GetService()?;

                client.Start()?;

                // Coeficientes de filtro paso-bajo calibrados para aislar el bombo/sub-bajo (20-150 Hz)
                let dt = 1.0 / sample_rate.max(44100.0);
                let alpha_bass = (2.0 * std::f32::consts::PI * 150.0 * dt).min(0.45);
                let alpha_mid = (2.0 * std::f32::consts::PI * 2200.0 * dt).min(0.8);

                let mut lp_bass = 0.0f32;
                let mut lp_mid = 0.0f32;

                while IS_VISUALIZER_RUNNING.load(Ordering::SeqCst) {
                    let mut packet_size = capture_client.GetNextPacketSize()?;

                    if packet_size == 0 {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }

                    let mut bass_energy = 0.0f32;
                    let mut mid_energy = 0.0f32;
                    let mut treble_energy = 0.0f32;
                    let mut samples_count = 0usize;

                    while packet_size > 0 {
                        let mut data_ptr: *mut u8 = std::ptr::null_mut();
                        let mut frames_read: u32 = 0;
                        let mut flags: u32 = 0;

                        let get_res = capture_client.GetBuffer(
                            &mut data_ptr,
                            &mut frames_read,
                            &mut flags,
                            None,
                            None,
                        );

                        if get_res.is_err() {
                            break;
                        }

                        let is_silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;

                        if !is_silent && !data_ptr.is_null() && frames_read > 0 {
                            if is_float {
                                let float_slice = std::slice::from_raw_parts(
                                    data_ptr as *const f32,
                                    (frames_read as usize) * channels,
                                );

                                for frame in float_slice.chunks(channels) {
                                    // Promedio mono
                                    let sample: f32 = frame.iter().sum::<f32>() / (channels as f32);

                                    // Filtro para separar bandas
                                    lp_bass += alpha_bass * (sample - lp_bass);
                                    lp_mid += alpha_mid * (sample - lp_mid);

                                    let bass_sample = lp_bass;
                                    let mid_sample = lp_mid - lp_bass;
                                    let treble_sample = sample - lp_mid;

                                    bass_energy += bass_sample * bass_sample;
                                    mid_energy += mid_sample * mid_sample;
                                    treble_energy += treble_sample * treble_sample;
                                    samples_count += 1;
                                }
                            } else {
                                // 16-bit PCM fallback
                                let i16_slice = std::slice::from_raw_parts(
                                    data_ptr as *const i16,
                                    (frames_read as usize) * channels,
                                );

                                for frame in i16_slice.chunks(channels) {
                                    let sample = (frame[0] as f32) / 32768.0;

                                    lp_bass += alpha_bass * (sample - lp_bass);
                                    lp_mid += alpha_mid * (sample - lp_mid);

                                    let bass_sample = lp_bass;
                                    let mid_sample = lp_mid - lp_bass;
                                    let treble_sample = sample - lp_mid;

                                    bass_energy += bass_sample * bass_sample;
                                    mid_energy += mid_sample * mid_sample;
                                    treble_energy += treble_sample * treble_sample;
                                    samples_count += 1;
                                }
                            }
                        }

                        let _ = capture_client.ReleaseBuffer(frames_read);
                        packet_size = capture_client.GetNextPacketSize().unwrap_or(0);
                    }

                    // Calcular valores RMS con énfasis en la pegada del bajo
                    let (raw_bass, raw_mid, raw_treble) = if samples_count > 0 {
                        let count_f = samples_count as f32;
                        let b = (bass_energy / count_f).sqrt() * 5.4;
                        let m = (mid_energy / count_f).sqrt() * 4.0;
                        let t = (treble_energy / count_f).sqrt() * 5.0;
                        (b.min(1.0), m.min(1.0), t.min(1.0))
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                    // Suavizado envolvente (ataque ultra-rápido 0.85 para el bombo, caída elástica 0.16)
                    let b_rate = if raw_bass > smoothed_bass { 0.85 } else { 0.16 };
                    let m_rate = if raw_mid > smoothed_mid { 0.70 } else { 0.18 };
                    let t_rate = if raw_treble > smoothed_treble { 0.70 } else { 0.18 };

                    smoothed_bass += (raw_bass - smoothed_bass) * b_rate;
                    smoothed_mid += (raw_mid - smoothed_mid) * m_rate;
                    smoothed_treble += (raw_treble - smoothed_treble) * t_rate;

                    // Emitir a ~35 Hz si hay cambio o actividad
                    if last_emit.elapsed() >= emit_interval {
                        last_emit = Instant::now();

                        // Curva de pegada no lineal para que el bombo se sienta físico
                        let b_val = if smoothed_bass < 0.015 {
                            0.0
                        } else {
                            (smoothed_bass.powf(1.15) * 1.12).min(1.0)
                        };
                        let m_val = if smoothed_mid < 0.015 { 0.0 } else { smoothed_mid };
                        let t_val = if smoothed_treble < 0.015 { 0.0 } else { smoothed_treble };

                        let bands = AudioBands {
                            bass: (b_val * 100.0).round() / 100.0,
                            mid: (m_val * 100.0).round() / 100.0,
                            treble: (t_val * 100.0).round() / 100.0,
                        };

                        let _ = app_handle.emit("audio-bands-update", &bands);
                    }

                    std::thread::sleep(Duration::from_millis(15));
                }

                let _ = client.Stop();
                CoTaskMemFree(Some(format_ptr as _));
                Ok(())
            }
        })();

        if capture_result.is_err() {
            // Si el dispositivo fue desconectado (AUDCLNT_E_DEVICE_INVALIDATED),
            // esperar 600ms antes de reintentar la conexión
            std::thread::sleep(Duration::from_millis(600));
        }
    }
}
