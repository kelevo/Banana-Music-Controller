import React, { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { FourColorPalette } from "../utils/colorExtractor";
import { AudioBands } from "../types/media";

interface AudioWaveSpectrumProps {
  isPlaying: boolean;
  palette: FourColorPalette;
}

const CANVAS_SIZE = 120;
const BASE_RADIUS = 25.5; // Coincide con el contorno de la carátula 52x52
const SQUIRCLE_EXPONENT = 3.4; // Exponente superelíptico que emula esquinas redondeadas de 11px
const POINTS_COUNT = 64;

/**
 * Calcula el radio base de una superelipse/squircle para un ángulo dado.
 */
function getSquircleRadius(angle: number, baseRadius: number): number {
  const cos = Math.abs(Math.cos(angle));
  const sin = Math.abs(Math.sin(angle));
  const denominator = Math.pow(
    Math.pow(cos, SQUIRCLE_EXPONENT) + Math.pow(sin, SQUIRCLE_EXPONENT),
    1 / SQUIRCLE_EXPONENT
  );
  return denominator > 0.001 ? baseRadius / denominator : baseRadius;
}

/**
 * Traza una curva cerrada continua suave que pasa por los puntos generados.
 */
function drawSmoothSpline(ctx: CanvasRenderingContext2D, points: { x: number; y: number }[]) {
  const len = points.length;
  if (len < 3) return;

  ctx.beginPath();
  const firstMid = {
    x: (points[0].x + points[1].x) / 2,
    y: (points[0].y + points[1].y) / 2,
  };
  ctx.moveTo(firstMid.x, firstMid.y);

  for (let i = 1; i < len; i++) {
    const nextIdx = (i + 1) % len;
    const midX = (points[i].x + points[nextIdx].x) / 2;
    const midY = (points[i].y + points[nextIdx].y) / 2;
    ctx.quadraticCurveTo(points[i].x, points[i].y, midX, midY);
  }

  ctx.quadraticCurveTo(points[0].x, points[0].y, firstMid.x, firstMid.y);
  ctx.closePath();
}

export const AudioWaveSpectrum: React.FC<AudioWaveSpectrumProps> = ({
  isPlaying,
  palette,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameIdRef = useRef<number | null>(null);
  const amplitudeRef = useRef<number>(isPlaying ? 1.0 : 0.0);
  const timeRef = useRef<number>(0);

  // Bandas reales de audio recibidas de WASAPI Loopback en Rust
  const bandsRef = useRef<AudioBands>({ bass: 0, mid: 0, treble: 0 });
  const smoothedBassRef = useRef<number>(0);

  // Suscripción al stream de análisis espectral WASAPI emitido por Rust (~35 Hz)
  useEffect(() => {
    const unlistenPromise = listen<AudioBands>("audio-bands-update", (event) => {
      if (event.payload) {
        bandsRef.current = event.payload;
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d", { alpha: true });
    if (!ctx) return;

    // Soporte para densidad de píxeles en pantallas de alta resolución (Retina / 4K)
    const dpr = window.devicePixelRatio || 1;
    canvas.width = CANVAS_SIZE * dpr;
    canvas.height = CANVAS_SIZE * dpr;
    ctx.scale(dpr, dpr);

    const center = CANVAS_SIZE / 2;

    const render = () => {
      const targetAmp = isPlaying ? 1.0 : 0.0;
      // Interpolación suave de amplitud (bloom al reproducir, contracción al pausar)
      amplitudeRef.current += (targetAmp - amplitudeRef.current) * 0.12;
      const currentAmp = amplitudeRef.current;

      // Si está en pausa y la amplitud ya se replegó por completo, limpiar y pausar loop
      if (!isPlaying && currentAmp < 0.005) {
        amplitudeRef.current = 0;
        ctx.clearRect(0, 0, CANVAS_SIZE, CANVAS_SIZE);
        animFrameIdRef.current = null;
        return;
      }

      ctx.clearRect(0, 0, CANVAS_SIZE, CANVAS_SIZE);

      // Suavizado del bajo (bass) en tiempo real
      const targetBass = isPlaying ? bandsRef.current.bass : 0;
      smoothedBassRef.current += (targetBass - smoothedBassRef.current) * 0.28;
      const bass = smoothedBassRef.current;

      // Ritmo de tiempo lento solo para respiración sutil en silencio (sin rotación)
      timeRef.current += 0.035;
      const t = timeRef.current;

      // Respiración sutil en momentos de silencio absoluto
      const subtleBreath = bass > 0.03 ? 0.0 : 0.18 * Math.sin(t * 2.2);

      // =========================================================================
      // DIBUJO ESTRATIFICADO (ORIENTACIÓN FIJA - SIN ROTACIÓN):
      // Capa 1 (Exterior) -> Capa 2 (Medio) -> Capa 3 (Interior)
      // Cada capa mantiene sus lóbulos en posición fija y pulsa al ritmo del bajo.
      // Intensidad calibrada a 2/3 del valor previo (reducción de un tercio).
      // =========================================================================

      // -------------------------------------------------------------------------
      // CAPA 1: EXTERIOR (Color 2: waveOuter) - La más grande, pulsa con el bajo
      // -------------------------------------------------------------------------
      const pointsLayer1: { x: number; y: number }[] = [];
      const baseOffset1 = 4.8;
      const waveAmp1 = (3.0 * subtleBreath + 20.0 * bass) * currentAmp;

      for (let i = 0; i < POINTS_COUNT; i++) {
        const angle = (i / POINTS_COUNT) * (Math.PI * 2);
        const baseR = getSquircleRadius(angle, BASE_RADIUS + baseOffset1);
        // Sin rotación en el ángulo: la onda se mantiene fija en su orientación
        const h1 = Math.sin(3 * angle);
        const h2 = Math.cos(2 * angle);
        const delta = (h1 * 0.65 + h2 * 0.35) * waveAmp1;
        const r = Math.max(BASE_RADIUS, baseR + delta);
        pointsLayer1.push({
          x: center + Math.cos(angle) * r,
          y: center + Math.sin(angle) * r,
        });
      }

      ctx.save();
      ctx.fillStyle = palette.waveOuter;
      ctx.globalAlpha = Math.min(1.0, 0.92 * currentAmp);
      ctx.shadowColor = palette.waveOuter;
      ctx.shadowBlur = (8 + 16 * bass) * currentAmp;
      drawSmoothSpline(ctx, pointsLayer1);
      ctx.fill();
      ctx.restore();

      // -------------------------------------------------------------------------
      // CAPA 2: INTERMEDIA (Color 3: waveMiddle) - Tamaño medio, sobre Capa 1
      // -------------------------------------------------------------------------
      const pointsLayer2: { x: number; y: number }[] = [];
      const baseOffset2 = 2.8;
      const waveAmp2 = (2.0 * subtleBreath + 13.0 * bass) * currentAmp;

      for (let i = 0; i < POINTS_COUNT; i++) {
        const angle = (i / POINTS_COUNT) * (Math.PI * 2);
        const baseR = getSquircleRadius(angle, BASE_RADIUS + baseOffset2);
        const h1 = Math.sin(3 * angle + 0.85);
        const h2 = Math.cos(2 * angle + 0.4);
        const delta = (h1 * 0.65 + h2 * 0.35) * waveAmp2;
        const r = Math.max(BASE_RADIUS, baseR + delta);
        pointsLayer2.push({
          x: center + Math.cos(angle) * r,
          y: center + Math.sin(angle) * r,
        });
      }

      ctx.save();
      ctx.fillStyle = palette.waveMiddle;
      ctx.globalAlpha = Math.min(1.0, 0.96 * currentAmp);
      ctx.shadowColor = palette.waveMiddle;
      ctx.shadowBlur = (5 + 10 * bass) * currentAmp;
      drawSmoothSpline(ctx, pointsLayer2);
      ctx.fill();
      ctx.restore();

      // -------------------------------------------------------------------------
      // CAPA 3: INTERIOR (Color 4: waveInner) - La más pequeña, sobre Capa 2
      // -------------------------------------------------------------------------
      const pointsLayer3: { x: number; y: number }[] = [];
      const baseOffset3 = 1.0;
      const waveAmp3 = (1.0 * subtleBreath + 7.0 * bass) * currentAmp;

      for (let i = 0; i < POINTS_COUNT; i++) {
        const angle = (i / POINTS_COUNT) * (Math.PI * 2);
        const baseR = getSquircleRadius(angle, BASE_RADIUS + baseOffset3);
        const h1 = Math.sin(3 * angle + 1.7);
        const h2 = Math.cos(2 * angle + 0.8);
        const delta = (h1 * 0.65 + h2 * 0.35) * waveAmp3;
        const r = Math.max(BASE_RADIUS, baseR + delta);
        pointsLayer3.push({
          x: center + Math.cos(angle) * r,
          y: center + Math.sin(angle) * r,
        });
      }

      ctx.save();
      ctx.fillStyle = palette.waveInner;
      ctx.globalAlpha = Math.min(1.0, 1.0 * currentAmp);
      drawSmoothSpline(ctx, pointsLayer3);
      ctx.fill();
      ctx.restore();

      animFrameIdRef.current = requestAnimationFrame(render);
    };

    // Iniciar renderizado
    animFrameIdRef.current = requestAnimationFrame(render);

    return () => {
      if (animFrameIdRef.current !== null) {
        cancelAnimationFrame(animFrameIdRef.current);
        animFrameIdRef.current = null;
      }
    };
  }, [isPlaying, palette]);

  return (
    <canvas
      ref={canvasRef}
      className="artwork-spectrum-canvas"
      style={{
        width: `${CANVAS_SIZE}px`,
        height: `${CANVAS_SIZE}px`,
      }}
    />
  );
};
