import { useState, useEffect, useRef } from "react";

export function useMarquee(text: string) {
  const containerRef = useRef<HTMLDivElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);
  const [isOverflowing, setIsOverflowing] = useState(false);
  const [overflowDistance, setOverflowDistance] = useState(0);

  useEffect(() => {
    const checkOverflow = () => {
      if (containerRef.current && textRef.current) {
        const containerWidth = containerRef.current.clientWidth;
        const textWidth = textRef.current.scrollWidth;
        const diff = textWidth - containerWidth;

        if (diff > 4) {
          setIsOverflowing(true);
          setOverflowDistance(diff);
        } else {
          setIsOverflowing(false);
          setOverflowDistance(0);
        }
      }
    };

    // Check after DOM update
    const timeout = setTimeout(checkOverflow, 50);

    const resizeObserver = new ResizeObserver(checkOverflow);
    if (containerRef.current) {
      resizeObserver.observe(containerRef.current);
    }

    return () => {
      clearTimeout(timeout);
      resizeObserver.disconnect();
    };
  }, [text]);

  return { containerRef, textRef, isOverflowing, overflowDistance };
}
