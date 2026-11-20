# Banana Music Controller 🍌🎵

[![macOS](https://img.shields.io/badge/macOS-12%2B%20Compatible-000000?logo=apple&logoColor=white)](https://apple.com/macos)
[![Windows 11](https://img.shields.io/badge/Windows%2011-Compatible-0078D4?logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Widget multimedia flotante, minimalista y siempre visible (*Always-on-Top*) compatible de forma nativa con **macOS** (12.0+) y **Windows 11**. Sincroniza y controla la reproducción multimedia global de tu sistema (Spotify, Apple Music, Google Chrome, YouTube, Udemy, VLC, etc.) con una estética adaptativa inspirada en píldora que extrae los colores dominantes de la carátula en reproducción y un espectro concéntrico de ondas de audio reactivo en tiempo real.

---

## ✨ Características Principales

- 🎵 **Sincronización Global Multimedia:**
  - **macOS:** Detección de reproductores nativos (Spotify, Apple Music) y pestañas multimedia de Google Chrome / Safari con descarga automática de miniaturas en HD para YouTube.
  - **Windows 11:** Integración profunda con GSMTC (*Global System Media Transport Controls*) vía WinRT.
- 🎨 **Paleta Dinámica Adaptativa:** Extracción en tiempo real de 4 colores armónicos de la carátula para generar el degradado de fondo y las ondas.
- 🌊 **Visualizador de Espectro Reactivo al Audio:** Espectro concéntrico de ondas de 3 capas situado detrás de la carátula:
  - En **macOS**: Captura de audio en tiempo real mediante CoreAudio analizando las frecuencias bajas (*bass*).
  - En **Windows**: WASAPI Loopback nativo a nivel de sistema.
- 📌 **Ventana Flotante Inteligente:**
  - Sin bordes nativos ni marco (`decorations: false`, `transparent: true`).
  - Arrastre libre con clic izquierdo hacia cualquier posición de tu pantalla.
  - **Snap rápido:** Doble clic sobre el widget para enviarlo a la esquina inferior derecha del monitor activo, respetando el Dock y la barra de menú en macOS, o la barra de tareas en Windows.
  - Menú contextual nativo con clic derecho para cerrar o alinear.
- 🔔 **Integración con System Tray (Bandeja del Sistema / Menú Bar):** Icono circular oficial en la barra de menús de macOS o en el área de notificación de Windows con menú para mostrar, ocultar y cerrar la aplicación.
- ⚡ **Rendimiento Ultraligero:** Construido con Tauri v2 y Rust. Consume menos de 35 MB de RAM y menos de 0.1% de CPU.
- 🛡️ **Cero Interrupciones de Audio:** Arquitectura desacoplada en hilos dedicados con recuperación progresiva de carátulas para evitar cortes o micro-stuttering en el sonido.

---

## 🚀 Instalación y Descarga

Puedes descargar el instalador oficial o el ejecutable directo desde la sección de **[Releases](https://github.com/tu-usuario/banana-music-controller/releases)**:

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
| **System Tray** | Clic izquierdo alterna visibilidad / Clic derecho abre menú de control |

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
   git clone https://github.com/tu-usuario/banana-music-controller.git
   cd banana-music-controller
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
banana-music-controller/
├── public/                         # Favicon e iconos web públicos
├── src/                            # Frontend en React 19 + TypeScript
│   ├── assets/                     # Iconos SVG y logos oficiales de la marca
│   ├── components/                 # AudioWidget, AudioWaveSpectrum, ProgressBar, Marquee...
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
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
└── README.md
```

---

## 💡 Metodología de Desarrollo

Este proyecto fue concebido y desarrollado inicialmente utilizando la metodología **Spec-Driven Development (SDD)** orquestada con **Antigravity**, garantizando una arquitectura desacoplada, estabilidad de audio garantizada y fidelidad visual con el diseño original.

> 🤝 **Contribuciones:** Las contribuciones de la comunidad son bienvenidas. Para aportar código, mejoras o correcciones de bugs, **no es necesario seguir estrictamente esta metodología**; siéntete libre de abrir un *Issue* o enviar un *Pull Request* siguiendo las buenas prácticas convencionales de Git.

---

## 👤 Créditos y Autoría

- **Desarrollador Principal:** [Patrick J. Hernandez](https://github.com/tu-usuario)
- **Diseño & Concepto:** Banana Music Controller Team

---

## 📄 Licencia

Este proyecto es de código abierto bajo la licencia [MIT](LICENSE).
