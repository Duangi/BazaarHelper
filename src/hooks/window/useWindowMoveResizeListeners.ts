import { useEffect, useRef, type Dispatch, type MutableRefObject, type SetStateAction } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

interface UseWindowMoveResizeListenersOptions {
  showVersionScreen: boolean;
  isCollapsed: boolean;
  setHasCustomPosition: Dispatch<SetStateAction<boolean>>;
  lastKnownPosition: MutableRefObject<{ x: number; y: number } | null>;
  expandedWidthRef: MutableRefObject<number>;
  expandedHeightRef: MutableRefObject<number>;
  isDragging: MutableRefObject<boolean>;
  isResizing: MutableRefObject<boolean>;
  lastUserResize: MutableRefObject<number>;
  isProgrammaticResize: MutableRefObject<boolean>;
}

export const useWindowMoveResizeListeners = ({
  showVersionScreen,
  isCollapsed,
  setHasCustomPosition,
  lastKnownPosition,
  expandedWidthRef,
  expandedHeightRef,
  isDragging,
  isResizing,
  lastUserResize,
  isProgrammaticResize,
}: UseWindowMoveResizeListenersOptions) => {
  const isInitialized = useRef(false);
  const moveDebounceTimer = useRef<number | null>(null);
  const saveSizeTimer = useRef<number | null>(null);
  const showVersionScreenRef = useRef(showVersionScreen);
  const isCollapsedRef = useRef(isCollapsed);

  useEffect(() => {
    showVersionScreenRef.current = showVersionScreen;
  }, [showVersionScreen]);

  useEffect(() => {
    isCollapsedRef.current = isCollapsed;
  }, [isCollapsed]);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    let unlistenMove: (() => void) | null = null;
    let unlistenResize: (() => void) | null = null;

    const setupListeners = async () => {
      setTimeout(() => {
        isInitialized.current = true;
      }, 500);

      unlistenMove = await appWindow.listen<{ x: number; y: number }>('tauri://move', (event) => {
        if (!isInitialized.current) return;

        isDragging.current = true;
        setHasCustomPosition(true);
        lastKnownPosition.current = { x: event.payload.x, y: event.payload.y };
        localStorage.setItem('plugin-pos-x', event.payload.x.toString());
        localStorage.setItem('plugin-pos-y', event.payload.y.toString());

        if (moveDebounceTimer.current) clearTimeout(moveDebounceTimer.current);
        moveDebounceTimer.current = window.setTimeout(() => {
          const physicalX = event.payload.x;
          const physicalY = event.payload.y;
          console.log('[Frontend] Saving position after move:', physicalX, physicalY);

          invoke('save_window_geometry', {
            windowLabel: 'main',
            x: physicalX,
            y: physicalY,
          }).catch(console.error);
          setTimeout(() => {
            isDragging.current = false;
          }, 120);
        }, 250);
      });

      unlistenResize = await appWindow.listen<{ width: number; height: number }>('tauri://resize', async () => {
        if (isProgrammaticResize.current) {
          setTimeout(() => {
            isProgrammaticResize.current = false;
          }, 200);
          return;
        }
        if (!isInitialized.current) return;
        if (showVersionScreenRef.current) return;

        isResizing.current = true;
        lastUserResize.current = Date.now();

        try {
          const factor = await appWindow.scaleFactor();
          const size = await appWindow.innerSize();
          const logicalWidth = Math.round(size.width / factor);
          const logicalHeight = Math.round(size.height / factor);

          if (logicalWidth > 150 && logicalHeight > 150) {
            expandedWidthRef.current = logicalWidth;
            if (!isCollapsedRef.current) {
              expandedHeightRef.current = logicalHeight;
            }

            if (saveSizeTimer.current) clearTimeout(saveSizeTimer.current);
            saveSizeTimer.current = window.setTimeout(() => {
              console.log('[Frontend] Saving size after resize:', size.width, size.height);

              invoke('save_window_geometry', {
                windowLabel: 'main',
                width: Math.round(size.width),
                height: Math.round(size.height),
              }).catch(console.error);

              localStorage.setItem('plugin-width', logicalWidth.toString());
              if (!isCollapsedRef.current) {
                localStorage.setItem('plugin-height', logicalHeight.toString());
              }
              setTimeout(() => {
                isResizing.current = false;
              }, 500);
            }, 2000);
          }
        } catch (e) {
          console.error('[Resize] Failed to get window size:', e);
        }
      });
    };

    void setupListeners();

    return () => {
      if (unlistenMove) unlistenMove();
      if (unlistenResize) unlistenResize();
      if (moveDebounceTimer.current) clearTimeout(moveDebounceTimer.current);
      if (saveSizeTimer.current) clearTimeout(saveSizeTimer.current);
    };
  }, [
    expandedHeightRef,
    expandedWidthRef,
    isDragging,
    isProgrammaticResize,
    isResizing,
    lastKnownPosition,
    lastUserResize,
    setHasCustomPosition,
  ]);
};
