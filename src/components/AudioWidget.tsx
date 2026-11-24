import React, { useState, useEffect, MouseEvent as ReactMouseEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

import { MediaState } from "../types/media";
import { ArtworkSection } from "./ArtworkSection";
import { TrackInfoMarquee } from "./TrackInfoMarquee";
import { PlaybackControls } from "./PlaybackControls";
import { ProgressBar } from "./ProgressBar";
import { useAudioProgress } from "../hooks/useAudioProgress";

interface AudioWidgetProps {
  media: MediaState;
  onLocalProgressTick: (newPos: number) => void;
  onSeek: (newPos: number) => void;
  onTogglePlayPause: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onContextMenu: (e: ReactMouseEvent<HTMLDivElement>) => void;
  onColorExtracted?: (color: string) => void;
}

function hexToRgba(hex: string, alpha: number): string {
  let clean = hex.replace("#", "");
  if (clean.length === 3) {
    clean = clean
      .split("")
      .map((c) => c + c)
      .join("");
  }
  const num = parseInt(clean, 16);
  if (isNaN(num)) return `rgba(33, 117, 59, ${alpha})`;
  const r = (num >> 16) & 255;
  const g = (num >> 8) & 255;
  const b = num & 255;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export const AudioWidget: React.FC<AudioWidgetProps> = ({
  media,
  onLocalProgressTick,
  onSeek,
  onTogglePlayPause,
  onPrevious,
  onNext,
  onContextMenu,
  onColorExtracted,
}) => {
  const [extractedColor, setExtractedColor] = useState<string | null>(null);

  // Resetear el color extraído si cambia el título o artista
  useEffect(() => {
    setExtractedColor(null);
  }, [media.title, media.artist]);

  // Avance suave de tiempo en el frontend sin sobrecargar el puente IPC ni enviar Seek a Windows
  useAudioProgress({
    status: media.status,
    positionSeconds: media.positionSeconds,
    durationSeconds: media.durationSeconds,
    onUpdatePosition: onLocalProgressTick,
  });

  // Arrastre de Ventana (Solo botón izquierdo y fuera de elementos interactivos)
  const handleMouseDown = async (e: ReactMouseEvent<HTMLDivElement>) => {
    if (e.button === 0) {
      const target = e.target as HTMLElement;
      if (!target.closest(".interactive")) {
        try {
          await getCurrentWindow().startDragging();
        } catch (err) {
          console.error("Error al arrastrar ventana:", err);
        }
      }
    }
  };

  // Doble Clic para Snap Inferior Derecho en monitor activo
  const handleDoubleClick = async (e: ReactMouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    if (!target.closest(".interactive")) {
      try {
        await invoke("snap_to_bottom_right");
      } catch (err) {
        console.error("Error en snap_to_bottom_right:", err);
      }
    }
  };

  const activeColor = extractedColor || media.accentColor || "#21753B";

  const dynamicStyles = {
    "--accent-color": activeColor,
    "--accent-glow": hexToRgba(activeColor, 0.45),
  } as React.CSSProperties;

  const hasActiveMedia = Boolean(
    media.title || media.status === "Playing" || media.status === "Paused"
  );

  return (
    <div
      className="audio-widget-card"
      style={dynamicStyles}
      onMouseDown={handleMouseDown}
      onDoubleClick={handleDoubleClick}
      onContextMenu={onContextMenu}
    >
      {/* Columna Izquierda: Carátula (52x52) con Wave Spectrum de audio */}
      <div className="widget-col-left">
        <ArtworkSection
          thumbnailBase64={media.thumbnailBase64}
          sourceApp={media.sourceApp}
          accentColor={activeColor}
          title={media.title}
          isPlaying={media.status === "Playing"}
          onColorExtracted={(color) => {
            setExtractedColor(color);
            if (onColorExtracted) {
              onColorExtracted(color);
            }
          }}
        />
      </div>

      {/* Columna Derecha: Contenido (Fila superior: Título/Artista + Controles; Fila inferior: Tiempos y Progreso) */}
      <div className="widget-content-section">
        <div className="widget-top-row">
          <TrackInfoMarquee title={media.title} artist={media.artist} />
          <PlaybackControls
            status={media.status}
            disabled={!hasActiveMedia}
            onPrevious={onPrevious}
            onTogglePlayPause={onTogglePlayPause}
            onNext={onNext}
          />
        </div>

        <div className="widget-bottom-row">
          <ProgressBar
            positionSeconds={media.positionSeconds}
            durationSeconds={media.durationSeconds}
            onSeek={onSeek}
          />
        </div>
      </div>
    </div>
  );
};
