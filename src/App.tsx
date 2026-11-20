import { useState, useEffect, MouseEvent as ReactMouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AudioWidget } from "./components/AudioWidget";
import { ContextMenu } from "./components/ContextMenu";
import { MediaState } from "./types/media";

import "./styles/widget.css";
import "./styles/marquee.css";

const defaultMediaState: MediaState = {
  title: "Sin reproducción activa",
  artist: "Windows Media",
  status: "Stopped",
  sourceApp: "Unknown",
  thumbnailBase64: null,
  accentColor: "#1DB954",
  positionSeconds: 0,
  durationSeconds: 0,
};

export default function App() {
  const [media, setMedia] = useState<MediaState>(defaultMediaState);
  const [contextMenu, setContextMenu] = useState<{
    visible: boolean;
    x: number;
    y: number;
  }>({
    visible: false,
    x: 0,
    y: 0,
  });

  useEffect(() => {
    // 1. Cargar estado inicial desde Windows 11 GSMTC
    invoke<MediaState>("get_media_state")
      .then((state) => {
        if (state) {
          setMedia(state);
        }
      })
      .catch((err) => console.warn("Error leyendo estado GSMTC inicial:", err));

    // 2. Suscribirse a eventos de cambio en tiempo real emitidos por Rust
    const unlistenPromise = listen<MediaState>("media-state-changed", (event) => {
      if (event.payload) {
        setMedia((prev) => {
          const next = event.payload;
          // Si la pista es idéntica pero el nuevo estado momentáneamente no trae carátula,
          // preservamos la carátula y el color previo para evitar parpadeos
          if (
            next.title === prev.title &&
            next.artist === prev.artist &&
            !next.thumbnailBase64 &&
            prev.thumbnailBase64
          ) {
            return {
              ...next,
              thumbnailBase64: prev.thumbnailBase64,
              accentColor: prev.accentColor,
            };
          }
          return next;
        });
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Avance suave puramente local de la posición (UI-only, sin tocar GSMTC)
  const handleLocalProgressTick = (newPosition: number) => {
    setMedia((prev) => ({
      ...prev,
      positionSeconds: newPosition,
    }));
  };

  // Seek manual del usuario (solo al soltar la barra de progreso)
  const handleManualSeek = async (newPosition: number) => {
    setMedia((prev) => ({
      ...prev,
      positionSeconds: newPosition,
    }));
    try {
      await invoke("media_seek", { positionSeconds: newPosition });
    } catch (err) {
      console.error("Error en media_seek:", err);
    }
  };

  const handleTogglePlayPause = async () => {
    try {
      await invoke("media_play_pause");
    } catch (err) {
      console.error("Error en media_play_pause:", err);
    }
  };

  const handlePrevious = async () => {
    try {
      await invoke("media_previous");
    } catch (err) {
      console.error("Error en media_previous:", err);
    }
  };

  const handleNext = async () => {
    try {
      await invoke("media_next");
    } catch (err) {
      console.error("Error en media_next:", err);
    }
  };

  const handleContextMenu = async (e: ReactMouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    try {
      await invoke("show_widget_context_menu");
    } catch {
      setContextMenu({
        visible: true,
        x: Math.min(e.clientX, 330 - 140),
        y: Math.min(e.clientY, 86 - 40),
      });
    }
  };

  return (
    <>
      {/* Widget Reproductor Principal (Siempre dinámico, estilo píldora de Figma) */}
      <AudioWidget
        media={media}
        onLocalProgressTick={handleLocalProgressTick}
        onSeek={handleManualSeek}
        onTogglePlayPause={handleTogglePlayPause}
        onPrevious={handlePrevious}
        onNext={handleNext}
        onContextMenu={handleContextMenu}
        onColorExtracted={(color) => {
          setMedia((prev) => ({
            ...prev,
            accentColor: color,
          }));
        }}
      />

      {/* Menú Contextual (Única opción: Cerrar) */}
      {contextMenu.visible && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu((prev) => ({ ...prev, visible: false }))}
        />
      )}
    </>
  );
}
