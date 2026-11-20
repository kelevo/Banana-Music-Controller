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
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::Arc;
    use std::sync::atomic::AtomicU32;

    let bass_accum = Arc::new(AtomicU32::new(0));
    let mid_accum = Arc::new(AtomicU32::new(0));
    let treble_accum = Arc::new(AtomicU32::new(0));
    let count_accum = Arc::new(AtomicU32::new(0));

    let b_acc = bass_accum.clone();
    let m_acc = mid_accum.clone();
    let t_acc = treble_accum.clone();
    let c_acc = count_accum.clone();

    // Captura de audio nativo mediante CoreAudio (cpal)
    let _stream = (|| -> Option<cpal::Stream> {
        let host = cpal::default_host();
        let device = host.default_input_device()?;
        let config = device.default_input_config().ok()?;

        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;
        let dt = 1.0 / sample_rate.max(44100.0);
        let alpha_bass = (2.0 * std::f32::consts::PI * 150.0 * dt).min(0.45);
        let alpha_mid = (2.0 * std::f32::consts::PI * 2200.0 * dt).min(0.8);

        let mut lp_bass = 0.0f32;
        let mut lp_mid = 0.0f32;

        let err_fn = |err| eprintln!("audio stream error: {}", err);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    let mut b_e = 0.0f32;
                    let mut m_e = 0.0f32;
                    let mut t_e = 0.0f32;
                    let mut count = 0u32;

                    for frame in data.chunks(channels) {
                        let sample: f32 = frame.iter().sum::<f32>() / (channels as f32);
                        lp_bass += alpha_bass * (sample - lp_bass);
                        lp_mid += alpha_mid * (sample - lp_mid);

                        let bass_s = lp_bass;
                        let mid_s = lp_mid - lp_bass;
                        let treble_s = sample - lp_mid;

                        b_e += bass_s * bass_s;
                        m_e += mid_s * mid_s;
                        t_e += treble_s * treble_s;
                        count += 1;
                    }

                    if count > 0 {
                        b_acc.fetch_add((b_e * 1000.0) as u32, Ordering::Relaxed);
                        m_acc.fetch_add((m_e * 1000.0) as u32, Ordering::Relaxed);
                        t_acc.fetch_add((t_e * 1000.0) as u32, Ordering::Relaxed);
                        c_acc.fetch_add(count, Ordering::Relaxed);
                    }
                },
                err_fn,
                None,
            ).ok()?,
            _ => return None,
        };

        let _ = stream.play();
        Some(stream)
    })();

    let mut smoothed_bass = 0.0f32;
    let mut smoothed_mid = 0.0f32;
    let mut smoothed_treble = 0.0f32;

    let mut last_emit = Instant::now();
    let emit_interval = Duration::from_millis(28); // ~35 Hz

    while IS_VISUALIZER_RUNNING.load(Ordering::SeqCst) {
        let is_playing = if let Ok(state) = crate::media_controller::get_media_state() {
            state.status == crate::media_controller::PlaybackStatus::Playing
        } else {
            false
        };

        let count = count_accum.swap(0, Ordering::Relaxed);
        let (raw_bass, raw_mid, raw_treble) = if is_playing && count > 0 {
            let b_raw = bass_accum.swap(0, Ordering::Relaxed) as f32 / 1000.0;
            let m_raw = mid_accum.swap(0, Ordering::Relaxed) as f32 / 1000.0;
            let t_raw = treble_accum.swap(0, Ordering::Relaxed) as f32 / 1000.0;

            let count_f = count as f32;
            let b = (b_raw / count_f).sqrt() * 7.5; // Ganancia calibrada para graves de altavoces de Mac
            let m = (m_raw / count_f).sqrt() * 5.0;
            let t = (t_raw / count_f).sqrt() * 4.0;
            (b.min(1.0), m.min(1.0), t.min(1.0))
        } else {
            (0.0, 0.0, 0.0)
        };

        let b_rate = if raw_bass > smoothed_bass { 0.85 } else { 0.16 };
        let m_rate = if raw_mid > smoothed_mid { 0.70 } else { 0.18 };
        let t_rate = if raw_treble > smoothed_treble { 0.70 } else { 0.18 };

        smoothed_bass += (raw_bass - smoothed_bass) * b_rate;
        smoothed_mid += (raw_mid - smoothed_mid) * m_rate;
        smoothed_treble += (raw_treble - smoothed_treble) * t_rate;

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
