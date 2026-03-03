import { useEffect, type Dispatch, type MutableRefObject, type SetStateAction } from 'react';
import { currentMonitor } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

interface UseWindowGeometryRestoreOptions {
  enabled?: boolean;
  currentScale: MutableRefObject<number>;
  lastKnownPosition: MutableRefObject<{ x: number; y: number } | null>;
  expandedWidthRef: MutableRefObject<number>;
  expandedHeightRef: MutableRefObject<number>;
  isLoadingGeometry: MutableRefObject<boolean>;
  setHasCustomPosition: Dispatch<SetStateAction<boolean>>;
  setExpandedWidth: Dispatch<SetStateAction<number>>;
  setExpandedHeight: Dispatch<SetStateAction<number>>;
  setIsGeometryLoaded: Dispatch<SetStateAction<boolean>>;
}

export const useWindowGeometryRestore = ({
  enabled = true,
  currentScale,
  lastKnownPosition,
  expandedWidthRef,
  expandedHeightRef,
  isLoadingGeometry,
  setHasCustomPosition,
  setExpandedWidth,
  setExpandedHeight,
  setIsGeometryLoaded,
}: UseWindowGeometryRestoreOptions) => {
  useEffect(() => {
    if (!enabled) return;

    const loadSavedPosition = async () => {
      try {
        const geometry = await invoke<{ x?: number; y?: number; width?: number; height?: number }>(
          'get_window_geometry',
          { windowLabel: 'main' },
        );
        console.log('[Frontend] Loaded geometry from backend:', geometry);

        let scale = 1.0;
        try {
          const monitor = await currentMonitor();
          if (monitor) scale = monitor.scaleFactor;
        } catch (e) {
          console.warn('Failed to get monitor scale', e);
        }
        currentScale.current = scale;

        if (geometry.x !== undefined && geometry.y !== undefined) {
          lastKnownPosition.current = { x: geometry.x, y: geometry.y };
          setHasCustomPosition(true);
          console.log('[Frontend] Using saved position:', geometry.x, geometry.y);
        } else {
          const x = localStorage.getItem('plugin-pos-x');
          const y = localStorage.getItem('plugin-pos-y');
          if (x !== null && y !== null) {
            lastKnownPosition.current = { x: parseInt(x, 10), y: parseInt(y, 10) };
            setHasCustomPosition(true);
            console.log('[Frontend] Using localStorage position:', x, y);
          }
        }

        if (geometry.width && geometry.height && geometry.width > 200 && geometry.height > 200) {
          const logicalW = Math.round(geometry.width / scale);
          const logicalH = Math.round(geometry.height / scale);
          console.log(
            `[Frontend] Using saved size (Physical -> Logical): ${geometry.width}x${geometry.height} -> ${logicalW}x${logicalH}`,
          );

          expandedWidthRef.current = logicalW;
          expandedHeightRef.current = logicalH;
          setExpandedWidth(logicalW);
          setExpandedHeight(logicalH);

          localStorage.setItem('plugin-width', logicalW.toString());
          localStorage.setItem('plugin-height', logicalH.toString());
        }
      } catch (e) {
        console.error('[Frontend] Failed to load saved geometry:', e);
      } finally {
        setTimeout(() => {
          isLoadingGeometry.current = false;
          setIsGeometryLoaded(true);
          console.log('[Frontend] Geometry loading complete, syncLayout enabled');
        }, 1000);
      }
    };

    void loadSavedPosition();
  }, [
    enabled,
    currentScale,
    expandedHeightRef,
    expandedWidthRef,
    isLoadingGeometry,
    lastKnownPosition,
    setExpandedHeight,
    setExpandedWidth,
    setHasCustomPosition,
    setIsGeometryLoaded,
  ]);
};
