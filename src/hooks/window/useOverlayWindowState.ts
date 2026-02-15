import { useRef, useState } from 'react';

export const useOverlayWindowState = () => {
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const [hasCustomPosition, setHasCustomPosition] = useState(false);
  const lastKnownPosition = useRef<{ x: number; y: number } | null>(null);
  const isLoadingGeometry = useRef(true);
  const [isGeometryLoaded, setIsGeometryLoaded] = useState(false);
  const currentScale = useRef(1);

  const isDragging = useRef(false);
  const isResizing = useRef(false);
  const lastUserResize = useRef<number>(0);
  const isProgrammaticResize = useRef(false);

  const getInitialWidth = () => {
    const saved = localStorage.getItem('plugin-width');
    if (saved) {
      const value = parseInt(saved, 10);
      if (value > 200) return value;
    }
    return 400;
  };

  const getInitialHeight = () => {
    const saved = localStorage.getItem('plugin-height');
    if (saved) {
      const value = parseInt(saved, 10);
      if (value > 200) return value;
    }
    return 700;
  };

  const expandedWidthRef = useRef(getInitialWidth());
  const expandedHeightRef = useRef(getInitialHeight());
  const [_expandedWidth, setExpandedWidth] = useState(getInitialWidth);
  const [_expandedHeight, setExpandedHeight] = useState(getInitialHeight);

  return {
    wrapRef,
    hasCustomPosition,
    setHasCustomPosition,
    lastKnownPosition,
    isLoadingGeometry,
    isGeometryLoaded,
    setIsGeometryLoaded,
    currentScale,
    isDragging,
    isResizing,
    lastUserResize,
    isProgrammaticResize,
    expandedWidthRef,
    expandedHeightRef,
    setExpandedWidth,
    setExpandedHeight,
  };
};
