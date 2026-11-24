import React from "react";
import { PlaybackStatus } from "../types/media";

import iconAnterior from "../assets/icons/icono_anterior.svg";
import iconPlay from "../assets/icons/icono_play.svg";
import iconPause from "../assets/icons/icono_pause.svg";
import iconSiguiente from "../assets/icons/icono_siguiente.svg";

interface PlaybackControlsProps {
  status: PlaybackStatus;
  disabled?: boolean;
  onPrevious: () => void;
  onTogglePlayPause: () => void;
  onNext: () => void;
}

export const PlaybackControls: React.FC<PlaybackControlsProps> = ({
  status,
  disabled = false,
  onPrevious,
  onTogglePlayPause,
  onNext,
}) => {
  return (
    <div className="playback-controls-group">
      <button
        type="button"
        disabled={disabled}
        className="control-action-btn btn-prev interactive"
        title="Canción anterior"
        onClick={(e) => {
          e.stopPropagation();
          if (!disabled) onPrevious();
        }}
      >
        <img
          src={iconAnterior}
          alt="Anterior"
          width={22}
          height={22}
          draggable={false}
        />
      </button>

      <button
        type="button"
        disabled={disabled}
        className="control-action-btn btn-play-pause interactive"
        title={status === "Playing" ? "Pausar" : "Reproducir"}
        onClick={(e) => {
          e.stopPropagation();
          if (!disabled) onTogglePlayPause();
        }}
      >
        <img
          src={status === "Playing" ? iconPause : iconPlay}
          alt={status === "Playing" ? "Pausar" : "Reproducir"}
          width={32}
          height={32}
          draggable={false}
        />
      </button>

      <button
        type="button"
        disabled={disabled}
        className="control-action-btn btn-next interactive"
        title="Siguiente canción"
        onClick={(e) => {
          e.stopPropagation();
          if (!disabled) onNext();
        }}
      >
        <img
          src={iconSiguiente}
          alt="Siguiente"
          width={22}
          height={22}
          draggable={false}
        />
      </button>
    </div>
  );
};
