/**
 * Utilidad de alto rendimiento para extraer la paleta cromática de 4 colores
 * (Fondo, Capa Exterior, Capa Intermedia, Capa Interior)
 * a partir de una carátula decodificada mediante un canvas HTML5 fuera de pantalla.
 */

export interface FourColorPalette {
  backgroundColor: string; // Color 1: Degradado del fondo del widget
  waveOuter: string;       // Color 2: Capa 1 exterior (la más grande)
  waveMiddle: string;      // Color 3: Capa 2 intermedia
  waveInner: string;       // Color 4: Capa 3 interior (la más pequeña)
}

export interface ColorPalette {
  primary: string;
  secondary: string;
  glow: string;
}

export const DEFAULT_FOUR_PALETTE: FourColorPalette = {
  backgroundColor: "#21753B",
  waveOuter: "#10B981",
  waveMiddle: "#06B6D4",
  waveInner: "#064E3B",
};

export function getFallbackFourPalette(
  sourceApp: string,
  accentColor?: string
): FourColorPalette {
  switch (sourceApp) {
    case "Spotify":
      return {
        backgroundColor: "#1DB954",
        waveOuter: "#1ED760",
        waveMiddle: "#00D2B4",
        waveInner: "#0D5C3A",
      };
    case "YouTube":
      return {
        backgroundColor: "#CC0000",
        waveOuter: "#FF0055",
        waveMiddle: "#FF5500",
        waveInner: "#7A001E",
      };
    case "Chrome":
      return {
        backgroundColor: "#1A73E8",
        waveOuter: "#4285F4",
        waveMiddle: "#EA4335",
        waveInner: "#FBBC05",
      };
    case "Udemy":
      return {
        backgroundColor: "#A435F0",
        waveOuter: "#EC4899",
        waveMiddle: "#8B5CF6",
        waveInner: "#581C87",
      };
    case "MediaPlayer":
      return {
        backgroundColor: "#2563EB",
        waveOuter: "#38BDF8",
        waveMiddle: "#6366F1",
        waveInner: "#1E3A8A",
      };
    default: {
      const base = accentColor || "#21753B";
      return {
        backgroundColor: base,
        waveOuter: "#10B981",
        waveMiddle: "#06B6D4",
        waveInner: "#064E3B",
      };
    }
  }
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      case b:
        h = (r - g) / d + 4;
        break;
    }
    h /= 6;
  }
  return [h * 360, s, l];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  h = ((h % 360) + 360) % 360;
  h /= 360;
  let r: number, g: number, b: number;

  if (s === 0) {
    r = g = b = l;
  } else {
    const hue2rgb = (p: number, q: number, t: number) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    r = hue2rgb(p, q, h + 1 / 3);
    g = hue2rgb(p, q, h);
    b = hue2rgb(p, q, h - 1 / 3);
  }
  return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
}

const toHex = (n: number) => Math.min(255, Math.max(0, n)).toString(16).padStart(2, "0").toUpperCase();

interface ScoredPixel {
  r: number;
  g: number;
  b: number;
  h: number;
  s: number;
  l: number;
  score: number;
}

export function extractFourColors(img: HTMLImageElement): FourColorPalette {
  try {
    const canvas = document.createElement("canvas");
    canvas.width = 32;
    canvas.height = 32;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    if (!ctx) return DEFAULT_FOUR_PALETTE;

    ctx.drawImage(img, 0, 0, 32, 32);
    const imgData = ctx.getImageData(0, 0, 32, 32).data;

    const scoredPixels: ScoredPixel[] = [];
    let fallbackRgb = { r: 60, g: 60, b: 60 };
    let hasFallback = false;

    for (let i = 0; i < imgData.length; i += 4) {
      const r = imgData[i];
      const g = imgData[i + 1];
      const b = imgData[i + 2];
      const a = imgData[i + 3];

      if (a < 128) continue;

      const brightness = (r + g + b) / 3;
      if (brightness < 18 || brightness > 240) continue;

      if (!hasFallback && brightness >= 35 && brightness <= 200) {
        fallbackRgb = { r, g, b };
        hasFallback = true;
      }

      const [h, s, l] = rgbToHsl(r, g, b);
      if (s < 0.12) continue; // Descartar grises planos

      const targetL = 0.52;
      const lScore = 1 - Math.abs(l - targetL);
      const score = s * 3.0 + lScore * 1.5;

      scoredPixels.push({ r, g, b, h, s, l, score });
    }

    if (scoredPixels.length === 0) {
      const hex = `#${toHex(fallbackRgb.r)}${toHex(fallbackRgb.g)}${toHex(fallbackRgb.b)}`;
      return {
        backgroundColor: hex,
        waveOuter: hex,
        waveMiddle: hex,
        waveInner: hex,
      };
    }

    scoredPixels.sort((a, b) => b.score - a.score);

    // Buscar hasta 4 colores distintos por separación cromática
    const distinct: ScoredPixel[] = [];
    for (const p of scoredPixels) {
      const isDistinct = distinct.every((d) => {
        const hueDiff = Math.abs(p.h - d.h);
        const circularHueDiff = Math.min(hueDiff, 360 - hueDiff);
        const rgbDist = Math.sqrt(
          Math.pow(p.r - d.r, 2) + Math.pow(p.g - d.g, 2) + Math.pow(p.b - d.b, 2)
        );
        return circularHueDiff >= 28 || rgbDist >= 55;
      });

      if (isDistinct) {
        distinct.push(p);
        if (distinct.length === 4) break;
      }
    }

    const basePrimary = distinct[0] || scoredPixels[0];
    const baseH = basePrimary.h;
    const baseS = Math.max(0.65, basePrimary.s);

    // 1. Color de Fondo (Luminosidad moderada/oscura para fundirse con #212121)
    const [bgR, bgG, bgB] = hslToRgb(baseH, baseS * 0.85, 0.32);
    const backgroundColor = `#${toHex(bgR)}${toHex(bgG)}${toHex(bgB)}`;

    // 2. Capa Exterior (Color más brillante, saturado y llamativo)
    let waveOuter: string;
    if (distinct.length >= 2) {
      const p = distinct[0];
      const [r, g, b] = hslToRgb(p.h, Math.min(1, p.s * 1.15), 0.56);
      waveOuter = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    } else {
      const [r, g, b] = hslToRgb(baseH, 0.95, 0.58);
      waveOuter = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    }

    // 3. Capa Intermedia (Color armónico o segundo matiz contrastante)
    let waveMiddle: string;
    if (distinct.length >= 2) {
      const p = distinct[1];
      const [r, g, b] = hslToRgb(p.h, Math.min(1, p.s * 1.1), 0.48);
      waveMiddle = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    } else {
      const [r, g, b] = hslToRgb(baseH + 40, 0.88, 0.48);
      waveMiddle = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    }

    // 4. Capa Interior (Color más cercano a la carátula, tono más profundo o triádico)
    let waveInner: string;
    if (distinct.length >= 3) {
      const p = distinct[2];
      const [r, g, b] = hslToRgb(p.h, p.s, 0.40);
      waveInner = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    } else {
      const [r, g, b] = hslToRgb(baseH - 45, 0.85, 0.38);
      waveInner = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    }

    return {
      backgroundColor,
      waveOuter,
      waveMiddle,
      waveInner,
    };
  } catch (err) {
    console.warn("Error al extraer 4 colores:", err);
    return DEFAULT_FOUR_PALETTE;
  }
}

export function extractPalette(img: HTMLImageElement): ColorPalette {
  const four = extractFourColors(img);
  return {
    primary: four.waveOuter,
    secondary: four.waveMiddle,
    glow: four.waveOuter,
  };
}

export function extractDominantColor(img: HTMLImageElement): string {
  return extractFourColors(img).backgroundColor;
}
