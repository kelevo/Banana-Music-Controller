import React, { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { FourColorPalette } from "../utils/colorExtractor";
import { AudioBands } from "../types/media";

interface AudioWaveSpectrumProps {
  isPlaying: boolean;
  palette: FourColorPalette;
}

const CANVAS_SIZE = 120;
const BASE_RADIUS = 26; // Radio del contorno de la carátula circular (52px / 2)
const POINTS_COUNT = 80; // 80 puntos para un trazado suave de las alas y el pellizco central

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

      // Ritmo de tiempo suave para respiración sutil en silencio (sin rotación)
      timeRef.current += 0.035;
      const t = timeRef.current;
      const subtleBreath = bass > 0.03 ? 0.0 : 0.15 * Math.sin(t * 2.2);

      // =========================================================================
      // ESPECTRO DE ONDAS INSPIRADO EN AFTER EFFECTS (SIMETRÍA BILATERAL LÍQUIDA):
      // - 2 Alas laterales simétricas que se abren y bombean con el bajo.
      // - En la parte superior, convergen suavemente y se desvanecen en el centro.
      // - En la parte inferior, forman una base redondeada limpia.
      // - 3 Capas concéntricas estrechas y ceñidas en escalón uniforme.
      // =========================================================================

      // Función generadora de la silueta de alas laterales estilo After Effects
      // con simetría bilateral matemática exacta (espejo perfecto izquierda/derecha)
      const calculateWingRadius = (
        angle: number,
        baseOffset: number,
        layerScale: number
      ): number => {
        // phi va de 0 (centro superior) a PI (centro inferior) simétricamente en ambos lados
        // Dado que -sin(angle) es idéntico para puntos simétricos respecto al eje Y vertical,
        // garantizamos un espejo perfecto izquierda/derecha.
        const phi = Math.acos(Math.max(-1, Math.min(1, -Math.sin(angle))));

        // 1. Ala superior prominente con pico centrado en ~48° (0.84 rad)
        const wingCenter = 0.84;
        const wingWidth = 0.40;
        let wing = Math.exp(-Math.pow((phi - wingCenter) / wingWidth, 2));

        // 2. Pérdida y pellizco en el centro superior (tiende a 0 cuando phi -> 0)
        wing *= Math.tanh(phi / 0.28);

        // 3. Amortiguación lateral e inferior (muy poco visible, tal como en el video de referencia)
        const bottomSubtle =
          phi > Math.PI / 2 ? 0.08 * Math.pow(Math.sin(phi - Math.PI / 2), 2) : 0.0;

        const shapeProfile = wing + bottomSubtle;

        // Amplitud ultra-reactiva que expande las alas al ritmo del bajo
        const maxAmp = (2.0 * subtleBreath + 18.0 * bass) * layerScale * currentAmp;
        const delta = shapeProfile * maxAmp;

        return BASE_RADIUS + baseOffset + delta;
      };

      // -------------------------------------------------------------------------
      // CAPA 1: EXTERIOR (Color 2: waveOuter) - Capa envolvente más externa
      // -------------------------------------------------------------------------
      const pointsLayer1: { x: number; y: number }[] = [];
      const baseOffset1 = 3.0;
      for (let i = 0; i < POINTS_COUNT; i++) {
        const angle = (i / POINTS_COUNT) * (Math.PI * 2);
        const r = calculateWingRadius(angle, baseOffset1, 1.0);
        pointsLayer1.push({
          x: center + Math.cos(angle) * r,
          y: center + Math.sin(angle) * r,
        });
      }

      ctx.save();
      ctx.fillStyle = palette.waveOuter;
      ctx.globalAlpha = Math.min(1.0, 0.92 * currentAmp);
      ctx.shadowColor = palette.waveOuter;
      ctx.shadowBlur = (6 + 12 * bass) * currentAmp;
      drawSmoothSpline(ctx, pointsLayer1);
      ctx.fill();
      ctx.restore();

      // -------------------------------------------------------------------------
      // CAPA 2: INTERMEDIA (Color 3: waveMiddle) - Ceñida a la Capa 1 (~1.2px)
      // -------------------------------------------------------------------------
      const pointsLayer2: { x: number; y: number }[] = [];
      const baseOffset2 = 2.0;
      for (let i = 0; i < POINTS_COUNT; i++) {
        const angle = (i / POINTS_COUNT) * (Math.PI * 2);
        const r = calculateWingRadius(angle, baseOffset2, 0.76);
        pointsLayer2.push({
          x: center + Math.cos(angle) * r,
          y: center + Math.sin(angle) * r,
        });
      }

      ctx.save();
      ctx.fillStyle = palette.waveMiddle;
      ctx.globalAlpha = Math.min(1.0, 0.96 * currentAmp);
      ctx.shadowColor = palette.waveMiddle;
      ctx.shadowBlur = (4 + 8 * bass) * currentAmp;
      drawSmoothSpline(ctx, pointsLayer2);
      ctx.fill();
      ctx.restore();

      // -------------------------------------------------------------------------
      // CAPA 3: INTERIOR (Color 4: waveInner) - Ceñida a la Capa 2 (~1.0px)
      // -------------------------------------------------------------------------
      const pointsLayer3: { x: number; y: number }[] = [];
      const baseOffset3 = 0.9;
      for (let i = 0; i < POINTS_COUNT; i++) {
        const angle = (i / POINTS_COUNT) * (Math.PI * 2);
        const r = calculateWingRadius(angle, baseOffset3, 0.52);
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
