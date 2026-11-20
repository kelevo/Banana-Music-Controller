import React, { useRef, useState, useCallback, MouseEvent as ReactMouseEvent } from "react";

interface ProgressBarProps {
  positionSeconds: number;
  durationSeconds: number;
  onSeek: (newSeconds: number) => void;
}

function formatDuration(totalSeconds: number): string {
  const validSecs = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(validSecs / 3600);
  const minutes = Math.floor((validSecs % 3600) / 60);
  const seconds = validSecs % 60;

  if (hours > 0) {
    return `${hours}:${minutes < 10 ? "0" : ""}${minutes}:${
      seconds < 10 ? "0" : ""
    }${seconds}`;
  }
  return `${minutes}:${seconds < 10 ? "0" : ""}${seconds}`;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  positionSeconds,
  durationSeconds,
  onSeek,
}) => {
  const trackRef = useRef<HTMLDivElement>(null);
  const [isScrubbing, setIsScrubbing] = useState(false);
  const [scrubPosition, setScrubPosition] = useState<number | null>(null);

  const calculateSeekTime = useCallback(
    (clientX: number): number => {
      if (!trackRef.current || durationSeconds <= 0) return 0;
      const rect = trackRef.current.getBoundingClientRect();
      const relativeX = clientX - rect.left;
      const ratio = Math.max(0, Math.min(1, relativeX / rect.width));
      return Math.floor(ratio * durationSeconds);
    },
    [durationSeconds]
  );

  const handleMouseDown = (e: ReactMouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    setIsScrubbing(true);
    const newTime = calculateSeekTime(e.clientX);
    setScrubPosition(newTime);

    const handleMouseMove = (moveEvent: globalThis.MouseEvent) => {
      moveEvent.preventDefault();
      const scrubTime = calculateSeekTime(moveEvent.clientX);
      setScrubPosition(scrubTime);
    };

    const handleMouseUp = (upEvent: globalThis.MouseEvent) => {
      setIsScrubbing(false);
      const finalTime = calculateSeekTime(upEvent.clientX);
      setScrubPosition(null);
      onSeek(finalTime);

      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };

  const displayedPosition =
    isScrubbing && scrubPosition !== null ? scrubPosition : positionSeconds;

  const progressPercent =
    durationSeconds > 0
      ? Math.min(100, Math.max(0, (displayedPosition / durationSeconds) * 100))
      : 0;

  return (
    <div className="progress-bar-wrapper">
      <span className="progress-time time-elapsed">
        {formatDuration(displayedPosition)}
      </span>

      <div
        ref={trackRef}
        className="progress-track interactive"
        title="Adelantar / Atrasar (clic o arrastre)"
        onMouseDown={handleMouseDown}
      >
        <div className="progress-track-line" />
        <div
          className="progress-thumb"
          style={{
            left: `${progressPercent}%`,
            transform: `translate(-${progressPercent}%, -50%)`,
          }}
        />
      </div>

      <span className="progress-time time-total">
        {formatDuration(durationSeconds)}
      </span>
    </div>
  );
};
