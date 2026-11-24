# Banana Music Controller 🍌🎵

[![macOS](https://img.shields.io/badge/macOS-12%2B%20Compatible-000000?logo=apple&logoColor=white)](https://apple.com/macos)
[![Windows 11](https://img.shields.io/badge/Windows%2011-Compatible-0078D4?logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Widget multimedia flotante, minimalista y siempre visible (*Always-on-Top*) compatible de forma nativa con **macOS** (12.0+) y **Windows 11**. Sincroniza y controla la reproducción multimedia global de tu sistema (Spotify, Apple Music, Google Chrome, YouTube, SoundCloud, VLC, etc.) con una estética adaptativa inspirada en píldora, carátula circular con punch elástico al ritmo del bombo y un espectro bilateral de ondas de audio reactivo en tiempo real estilo After Effects.

---

## ✨ Características Principales

- 🎵 **Sincronización Global Multimedia:**
  - **macOS:** Detección de reproductores nativos (Spotify, Apple Music) y pestañas multimedia de Google Chrome / Safari con descarga automática de miniaturas en HD para YouTube y SoundCloud.
  - **Windows 11:** Integración profunda con GSMTC (*Global System Media Transport Controls*) vía WinRT.
- 🎨 **Paleta Dinámica Adaptativa:** Extracción en tiempo real de 4 colores armónicos de la carátula para generar el degradado de fondo y las ondas.
- 🟡 **Carátula Circular con Efecto Punch:** Medallón circular centrado que pulsa elásticamente en GPU al compás de las frecuencias bajas (*bass*).
- 🌊 **Visualizador de Espectro Bilateral Reactivo:** Espectro de ondas de 3 capas concéntricas ceñidas con simetría en espejo exacta inspirado en After Effects:
  - Alas laterales superiores que reaccionan con potencia al ritmo de los bajos y convergen con un pellizco suave en el centro superior.
  - Amortiguación armónica lateral e inferior para mantener una silueta limpia y orgánica.
  - En **macOS**: Captura y filtrado IIR de frecuencias bajas (20-150 Hz) en tiempo real mediante **CoreAudio**.
  - En **Windows**: Captura por hardware mediante **WASAPI Loopback**.
- 📌 **Ventana Flotante Inteligente:**
  - Sin bordes nativos ni marco (`decorations: false`, `transparent: true`).
  - Arrastre libre con clic izquierdo hacia cualquier posición de tu pantalla.
  - **Snap rápido:** Doble clic sobre el widget para enviarlo a la esquina inferior derecha del monitor activo, respetando el Dock y la barra de menú en macOS, o la barra de tareas en Windows.
  - Menú contextual nativo con clic derecho para cerrar o alinear.
- 🔕 **Modo Tray-Only (Sin Saturar tu Espacio de Trabajo):**
  - **macOS:** Configurado como accesorio del sistema (`LSUIElement` / `ActivationPolicy::Accessory`), por lo que **no aparece en el Dock**, habitando únicamente en la barra de menús superior.
  - **Windows 11:** Estilo `WS_EX_TOOLWINDOW` para que **no aparezca en la barra de tareas**, gestionándose exclusivamente desde la bandeja del sistema (*System Tray*).
- ⚡ **Rendimiento Ultraligero:** Construido con Tauri v2 y Rust. Consume menos de 35 MB de RAM y menos de 0.1% de CPU.
- 🛡️ **Cero Interrupciones de Audio:** Arquitectura desacoplada en hilos dedicados para evitar cortes o micro-stuttering en el sonido.

---

## 🚀 Instalación y Descarga

Puedes descargar el instalador oficial o el ejecutable directo desde la sección de **[Releases](https://github.com/kelevo/Banana-Music-Controller/releases)**:

### macOS (Apple Silicon & Intel)
1. **Instalador Oficial DMG (`banana-music-controller_0.1.0_aarch64.dmg`):**  
   Abre la imagen de disco y arrastra la aplicación a tu carpeta de **Aplicaciones**.
2. **Aplicación directa (`banana-music-controller.app`):**  
   Ejecutable directo empaquetado para macOS 12.0+.

### Windows 11
1. **Instalador Oficial (`banana-music-controller_0.1.0_x64-setup.exe`):**  
   Asistente de instalación de Windows con accesos directos en el Menú Inicio y Escritorio.
2. **Versión Portable (`banana-music-controller.exe`):**  
   Binario único ejecutable sin necesidad de instalación.

---

## 🎮 Controles y Gestos

| Acción | Gesto / Control |
|---|---|
| **Mover widget** | Clic izquierdo sostenido y arrastrar (fuera de botones) |
| **Alinear a la esquina** | Doble clic sobre el widget |
| **Menú contextual** | Clic derecho sobre el widget (Cerrar / Alinear) |
| **Controles de música** | Play / Pause, Canción anterior, Canción siguiente |
| **Buscar posición (Seek)** | Clic o arrastre en la barra de progreso |
| **System Tray / Menú Bar** | Clic izquierdo alterna visibilidad / Clic derecho abre menú de control |

---

## 🛠️ Desarrollo Local

### Requisitos Previos

- [Node.js](https://nodejs.org/) (versión 18 o superior)
- [Rust](https://www.rust-lang.org/tools/install) (versión 1.77+)
- **En macOS:** Xcode Command Line Tools (`xcode-select --install`)
- **En Windows:** [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### Instrucciones de Configuración

1. **Clonar el repositorio:**
   ```bash
   git clone https://github.com/kelevo/Banana-Music-Controller.git
   cd Banana-Music-Controller
   ```

2. **Instalar dependencias de Node:**
   ```bash
   npm install
   ```

3. **Iniciar en entorno de desarrollo:**
   ```bash
   npm run tauri dev
   ```

4. **Compilar para producción (Instaladores NSIS / DMG):**
   ```bash
   npm run tauri build
   ```
   - **En macOS:** El instalador se generará en `src-tauri/target/release/bundle/dmg/`.
   - **En Windows:** El instalador se generará en `src-tauri/target/release/bundle/nsis/`.

---

## 🏗️ Arquitectura del Proyecto

```text
Banana-Music-Controller/
├── public/                         # Favicon e iconos web públicos
├── src/                            # Frontend en React 19 + TypeScript
│   ├── assets/                     # Iconos SVG y logos oficiales de la marca
│   ├── components/                 # AudioWidget, AudioWaveSpectrum, ArtworkSection...
│   ├── hooks/                      # useAudioProgress, useMarquee...
│   ├── styles/                     # Estilos visuales del widget y animaciones CSS
│   └── utils/                      # Extractor cromático de paletas de carátulas
├── src-tauri/                      # Backend en Rust (Tauri v2)
│   ├── icons/                      # Variantes nativas de iconos (.icns, .ico, .png, tray)
│   ├── src/
│   │   ├── audio_visualizer.rs     # Captura de audio loopback (CoreAudio en macOS / WASAPI en Windows)
│   │   ├── media_controller.rs     # Coordinador multimedia desacoplado
│   │   ├── media_controller_macos.rs # Integración con Spotify, Chrome y Apple Music en macOS
│   │   ├── window_manager.rs       # Snap a la esquina adaptativo y menús nativos
│   │   └── lib.rs                  # Setup de Tauri, System Tray y manejadores IPC
│   ├── Info.plist                  # Configuración macOS (LSUIElement para modo sin Dock)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
└── README.md
```

---

## 👤 Créditos y Autoría

- **Desarrollador Principal:** [Patrick J. Hernandez](https://github.com/kelevo)
- **Diseño & Concepto:** Banana Music Controller Team

---

## 📄 Licencia

Este proyecto es de código abierto bajo la licencia [MIT](LICENSE).
