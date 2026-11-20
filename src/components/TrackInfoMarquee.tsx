import React from "react";
import { useMarquee } from "../hooks/useMarquee";

interface TrackInfoMarqueeProps {
  title: string;
  artist: string;
}

export const TrackInfoMarquee: React.FC<TrackInfoMarqueeProps> = ({
  title,
  artist,
}) => {
  const {
    containerRef: titleContainerRef,
    textRef: titleTextRef,
    isOverflowing: isTitleOverflowing,
    overflowDistance: titleOverflowDistance,
  } = useMarquee(title);

  const {
    containerRef: artistContainerRef,
    textRef: artistTextRef,
    isOverflowing: isArtistOverflowing,
    overflowDistance: artistOverflowDistance,
  } = useMarquee(artist);

  // Calcula una duración amigable de lectura según la distancia
  const titleDuration = Math.max(6, (titleOverflowDistance / 24) + 4);
  const artistDuration = Math.max(6, (artistOverflowDistance / 24) + 4);

  return (
    <div className="track-info-container">
      {/* Título de la pista */}
      <div
        ref={titleContainerRef}
        className={`marquee-container ${isTitleOverflowing ? "overflowing" : ""}`}
      >
        <span
          ref={titleTextRef}
          className={`track-title-text marquee-text ${
            isTitleOverflowing ? "animate" : ""
          }`}
          style={
            isTitleOverflowing
              ? ({
                  "--overflow-dist": `${titleOverflowDistance}px`,
                  "--marquee-duration": `${titleDuration}s`,
                } as React.CSSProperties)
              : undefined
          }
          title={title}
        >
          {title}
        </span>
      </div>

      {/* Artista / Fuente */}
      <div
        ref={artistContainerRef}
        className={`marquee-container ${isArtistOverflowing ? "overflowing" : ""}`}
      >
        <span
          ref={artistTextRef}
          className={`track-artist-text marquee-text ${
            isArtistOverflowing ? "animate" : ""
          }`}
          style={
            isArtistOverflowing
              ? ({
                  "--overflow-dist": `${artistOverflowDistance}px`,
                  "--marquee-duration": `${artistDuration}s`,
                } as React.CSSProperties)
              : undefined
          }
          title={artist}
        >
          {artist}
        </span>
      </div>
    </div>
  );
};
