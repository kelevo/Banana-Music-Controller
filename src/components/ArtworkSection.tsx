import React, { useState, useEffect } from "react";
import { MediaSourceApp } from "../types/media";
import {
  extractFourColors,
  FourColorPalette,
  getFallbackFourPalette,
} from "../utils/colorExtractor";
import { AudioWaveSpectrum } from "./AudioWaveSpectrum";
import iconChrome from "../assets/icons/icono_chrome.png";

interface ArtworkSectionProps {
  thumbnailBase64: string | null;
  sourceApp: MediaSourceApp;
  accentColor: string;
  title: string;
  isPlaying: boolean;
  onColorExtracted?: (color: string) => void;
  onPaletteExtracted?: (palette: FourColorPalette) => void;
}

export const ArtworkSection: React.FC<ArtworkSectionProps> = ({
  thumbnailBase64,
  sourceApp,
  accentColor,
  title,
  isPlaying,
  onColorExtracted,
  onPaletteExtracted,
}) => {
  const [imgError, setImgError] = useState(false);
  const [palette, setPalette] = useState<FourColorPalette>(() =>
    getFallbackFourPalette(sourceApp, accentColor)
  );

  useEffect(() => {
    setImgError(false);
    if (!thumbnailBase64) {
      setPalette(getFallbackFourPalette(sourceApp, accentColor));
    }
  }, [thumbnailBase64, sourceApp, accentColor]);

  const hasValidThumbnail =
    !imgError &&
    Boolean(
      thumbnailBase64 &&
        typeof thumbnailBase64 === "string" &&
        thumbnailBase64.trim().length > 30 &&
        thumbnailBase64 !== "null"
    );

  const renderFallbackIcon = () => {
    switch (sourceApp) {
      case "Chrome":
        return (
          <div className="artwork-source-chrome">
            <img
              src={iconChrome}
              alt="Google Chrome"
              className="artwork-source-icon"
              draggable={false}
            />
          </div>
        );
      case "YouTube":
        return (
          <div className="artwork-source-badge" title="YouTube">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <rect width="24" height="24" rx="6" fill="#FF0000" />
              <path d="M10 8L16 12L10 16V8Z" fill="white" />
            </svg>
          </div>
        );
      case "Udemy":
        return (
          <div className="artwork-source-badge" title="Udemy">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <rect width="24" height="24" rx="6" fill="#A435F0" />
              <path
                d="M12 7C9.24 7 7 9.24 7 12V14C7 15.66 8.34 17 10 17H14C15.66 17 17 15.66 17 14V12C17 9.24 14.76 7 12 7ZM15 14C15 14.55 14.55 15 14 15H10C9.45 15 9 14.55 9 14V12C9 10.34 10.34 9 12 9C13.66 9 15 10.34 15 12V14Z"
                fill="white"
              />
            </svg>
          </div>
        );
      case "Spotify":
        return (
          <div className="artwork-source-badge" title="Spotify">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <rect width="24" height="24" rx="6" fill="#1DB954" />
              <path
                d="M17.5 13.5C14.8 11.9 10.4 11.7 7.9 12.5C7.5 12.6 7.1 12.4 7 12C6.9 11.6 7.1 11.2 7.5 11.1C10.4 10.2 15.3 10.4 18.4 12.2C18.8 12.4 18.9 12.9 18.7 13.3C18.5 13.6 17.9 13.7 17.5 13.5ZM17.4 15.8C17.2 16.1 16.8 16.2 16.5 16C14.1 14.5 10.6 14.1 8 14.9C7.7 15 7.3 14.8 7.2 14.5C7.1 14.2 7.3 13.8 7.6 13.7C10.6 12.8 14.5 13.3 17.2 14.9C17.5 15.1 17.6 15.5 17.4 15.8ZM16.3 18C16.1 18.2 15.8 18.3 15.6 18.1C13.6 16.9 10.9 16.6 8.4 17.2C8.1 17.3 7.9 17.1 7.8 16.8C7.7 16.5 7.9 16.3 8.2 16.2C11 15.6 14 15.9 16.2 17.3C16.4 17.5 16.5 17.8 16.3 18Z"
                fill="black"
              />
            </svg>
          </div>
        );
      case "MediaPlayer":
      case "Unknown":
      default:
        return (
          <div className="artwork-source-badge" title="Reproductor multimedia">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#A1A1AA" strokeWidth="2">
              <path d="M9 18V5l12-2v13" />
              <circle cx="6" cy="18" r="3" />
              <circle cx="18" cy="16" r="3" />
            </svg>
          </div>
        );
    }
  };

  return (
    <div className="artwork-container">
      {/* Espectro Wave de Audio estratificado (3 capas, reactivas al bajo) situado por detrás de la carátula */}
      <AudioWaveSpectrum isPlaying={isPlaying} palette={palette} />

      {/* Carátula fija de 52x52 con esquinas redondeadas en primer plano */}
      <div className="artwork-wrapper">
        {hasValidThumbnail && thumbnailBase64 ? (
          <img
            src={
              thumbnailBase64.startsWith("data:")
                ? thumbnailBase64
                : `data:image/jpeg;base64,${thumbnailBase64}`
            }
            alt={title}
            className="artwork-image"
            draggable={false}
            onLoad={(e) => {
              const four = extractFourColors(e.currentTarget);
              setPalette(four);
              if (onPaletteExtracted) {
                onPaletteExtracted(four);
              }
              if (onColorExtracted) {
                onColorExtracted(four.backgroundColor);
              }
            }}
            onError={() => setImgError(true)}
          />
        ) : (
          <div className="artwork-source-fallback">{renderFallbackIcon()}</div>
        )}
      </div>
    </div>
  );
};
