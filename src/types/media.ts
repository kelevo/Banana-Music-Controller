export type PlaybackStatus = 'Playing' | 'Paused' | 'Stopped' | 'Closed';

export type MediaSourceApp = 
  | 'Spotify' 
  | 'Chrome' 
  | 'YouTube' 
  | 'Udemy' 
  | 'Safari'
  | 'AppleMusic'
  | 'MediaPlayer' 
  | 'Unknown';

export interface MediaState {
  title: string;
  artist: string;
  status: PlaybackStatus;
  sourceApp: MediaSourceApp;
  thumbnailBase64: string | null; // Imagen de carátula en base64 o null si no existe
  accentColor: string;            // Hexadecimal dominante (ej. "#1DB954"), fallback "#52525B"
  positionSeconds: number;        // Tiempo transcurrido actual
  durationSeconds: number;        // Duración total de la pista
}

export interface AudioBands {
  bass: number;    // 0.0 a 1.0 (sub-bajos y bombo)
  mid: number;     // 0.0 a 1.0 (voces y melodías)
  treble: number;  // 0.0 a 1.0 (agudos y platillos)
}
