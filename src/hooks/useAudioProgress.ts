import { useEffect, useRef } from "react";
import { PlaybackStatus } from "../types/media";

interface UseAudioProgressProps {
  status: PlaybackStatus;
  positionSeconds: number;
  durationSeconds: number;
  onUpdatePosition: (newPosition: number) => void;
}

export function useAudioProgress({
  status,
  positionSeconds,
  durationSeconds,
  onUpdatePosition,
}: UseAudioProgressProps) {
  const currentPosRef = useRef(positionSeconds);
  const durationRef = useRef(durationSeconds);
  const onUpdateRef = useRef(onUpdatePosition);

  useEffect(() => {
    currentPosRef.current = positionSeconds;
  }, [positionSeconds]);

  useEffect(() => {
    durationRef.current = durationSeconds;
  }, [durationSeconds]);

  useEffect(() => {
    onUpdateRef.current = onUpdatePosition;
  }, [onUpdatePosition]);

  useEffect(() => {
    if (status !== "Playing") return;

    const interval = setInterval(() => {
      const nextPos = Math.min(durationRef.current, currentPosRef.current + 1);
      currentPosRef.current = nextPos;
      onUpdateRef.current(nextPos);
    }, 1000);

    return () => clearInterval(interval);
  }, [status]);
}
