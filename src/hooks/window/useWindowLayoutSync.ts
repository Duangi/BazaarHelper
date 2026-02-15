import { useEffect, useRef, type Dispatch, type MutableRefObject, type SetStateAction } from 'react';
import { getCurrentWindow, LogicalPosition, LogicalSize, currentMonitor } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface UseWindowLayoutSyncOptions {
  showVersionScreen: boolean;
  isCollapsed: boolean;
  hasCustomPosition: boolean;
  isGeometryLoaded: boolean;
  isLoadingGeometry: MutableRefObject<boolean>;
  currentScale: MutableRefObject<number>;
  lastKnownPosition: MutableRefObject<{ x: number; y: number } | null>;
  expandedWidthRef: MutableRefObject<number>;
  expandedHeightRef: MutableRefObject<number>;
  isResizing: MutableRefObject<boolean>;
  lastUserResize: MutableRefObject<number>;
  isProgrammaticResize: MutableRefObject<boolean>;
  isDragging: MutableRefObject<boolean>;
  yoloHotkey: number | null;
  detailDisplayHotkey: number | null;
  setYoloHotkey: Dispatch<SetStateAction<number | null>>;
  setHasCustomPosition: Dispatch<SetStateAction<boolean>>;
  setIsCollapsed: Dispatch<SetStateAction<boolean>>;
  setExpandedWidth: Dispatch<SetStateAction<number>>;
  setExpandedHeight: Dispatch<SetStateAction<number>>;
  showToast: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
}

export const useWindowLayoutSync = ({
  showVersionScreen,
  isCollapsed,
  hasCustomPosition,
  isGeometryLoaded,
  isLoadingGeometry,
  currentScale,
  lastKnownPosition,
  expandedWidthRef,
  expandedHeightRef,
  isResizing,
  lastUserResize,
  isProgrammaticResize,
  isDragging,
  yoloHotkey,
  detailDisplayHotkey,
  setYoloHotkey,
  setHasCustomPosition,
  setIsCollapsed,
  setExpandedWidth,
  setExpandedHeight,
  showToast,
}: UseWindowLayoutSyncOptions) => {
  useEffect(() => {
    const unlisten = listen('reset-window-geometry', () => {
      console.log('[Window] Reset geometry event received');
      setHasCustomPosition(false);
      localStorage.removeItem('window-has-custom-position');
      localStorage.removeItem('window-last-position');
      setIsCollapsed(false);
      localStorage.removeItem('window-is-collapsed');
      setExpandedWidth(600);
      setExpandedHeight(850);
      localStorage.setItem('window-expanded-width', '600');
      localStorage.setItem('window-expanded-height', '850');

      showToast('窗口已复位到默认位置和大小', 'success');
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [setExpandedHeight, setExpandedWidth, setHasCustomPosition, setIsCollapsed, showToast]);

  useEffect(() => {
    if (yoloHotkey && detailDisplayHotkey && yoloHotkey === detailDisplayHotkey) {
      console.warn('[Hotkey] Conflict detected between YOLO and Detail Display. Resetting YOLO hotkey.');
      setYoloHotkey(null);
      localStorage.removeItem('yolo-hotkey');
      invoke('set_yolo_hotkey', { hotkey: null }).catch(console.error);
      showToast('检测到按键冲突，已自动清除 YOLO 热键', 'warning');
    }
  }, [detailDisplayHotkey, setYoloHotkey, showToast, yoloHotkey]);

  useEffect(() => {
    const detectScale = async () => {
      try {
        const monitor = await currentMonitor();
        if (monitor) {
          currentScale.current = monitor.scaleFactor;
          const { height } = monitor.size;
          const logicalHeight = height / monitor.scaleFactor;
          console.log(`[Screen] height: ${height}, scale: ${monitor.scaleFactor}, logical: ${logicalHeight}`);

          if (!localStorage.getItem('plugin-height')) {
            setExpandedHeight(Math.max(600, Math.floor(logicalHeight - 200)));
          }
        }
      } catch (e) {
        console.error('检测屏幕信息失败:', e);
      }
    };
    void detectScale();
  }, [currentScale, setExpandedHeight]);

  const lastLayout = useRef<string>('');
  useEffect(() => {
    if (isLoadingGeometry.current) {
      console.log('[Layout] Skipping syncLayout - still loading geometry from backend');
      return;
    }

    const syncLayout = async () => {
      const appWindow = getCurrentWindow();
      const manualMinimized = localStorage.getItem('bh-window-manual-minimize') === '1';
      if (manualMinimized) {
        const [isMinimized, isVisible] = await Promise.all([
          appWindow.isMinimized().catch(() => false),
          appWindow.isVisible().catch(() => true),
        ]);
        if (isMinimized || !isVisible) return;
        localStorage.removeItem('bh-window-manual-minimize');
      }

      let logicalScale = 1.0;
      let pX = 0;
      let pY = 0;
      let pWidth = 1920;
      let pHeight = 1080;

      try {
        const monitor = await currentMonitor();
        if (monitor && monitor.size) {
          logicalScale = monitor.scaleFactor || 1.0;
          pX = monitor.position.x;
          pY = monitor.position.y;
          pWidth = Math.round(monitor.size.width / logicalScale);
          pHeight = Math.round(monitor.size.height / logicalScale);
        }
      } catch {
      }

      currentScale.current = logicalScale;

      let targetW = 0;
      let targetH = 0;
      let targetX = 0;
      let targetY = 0;

      if (showVersionScreen) {
        targetW = 600;
        targetH = 850;
        if (hasCustomPosition && lastKnownPosition.current) {
          targetX = Math.round(lastKnownPosition.current.x / logicalScale);
          targetY = Math.round(lastKnownPosition.current.y / logicalScale);
        } else {
          targetX = Math.round(pX + (pWidth - targetW) / 2);
          targetY = Math.round(pY + (pHeight - targetH) / 2);
        }
      } else {
        targetW = Math.round(Math.min(expandedWidthRef.current, pWidth - 20));
        targetH = Math.round(Math.min(isCollapsed ? 45 : expandedHeightRef.current, pHeight - 40));

        if (hasCustomPosition && lastKnownPosition.current) {
          targetX = Math.round(lastKnownPosition.current.x / logicalScale);
          targetY = Math.round(lastKnownPosition.current.y / logicalScale);
        } else {
          targetX = Math.round(pX + pWidth - targetW);
          targetY = Math.round(pY);
        }
      }

      try {
        if (appWindow.setShadow) await appWindow.setShadow(false);

        const size = await appWindow.innerSize();
        const pos = await appWindow.outerPosition();
        const currentWLogical = Math.round(size.width / logicalScale);
        const currentHLogical = Math.round(size.height / logicalScale);
        const currentX = Math.round(pos.x / logicalScale);
        const currentY = Math.round(pos.y / logicalScale);

        const layoutKey = `${targetW}-${targetH}-${targetX}-${targetY}`;
        if (lastLayout.current === layoutKey) return;
        lastLayout.current = layoutKey;

        const now = Date.now();
        const recentlyResized = lastUserResize.current && (now - lastUserResize.current < 1000);
        const shouldSkipResize =
          isResizing.current || recentlyResized || isProgrammaticResize.current || isDragging.current;

        const widthDiff = Math.abs(currentWLogical - targetW);
        const heightDiff = Math.abs(currentHLogical - targetH);

        if (!shouldSkipResize && (widthDiff > 5 || heightDiff > 5)) {
          isProgrammaticResize.current = true;
          console.log(`[Layout] Resizing: ${currentWLogical}x${currentHLogical} -> ${targetW}x${targetH}`);
          await appWindow.setSize(new LogicalSize(targetW, targetH));
          setTimeout(() => {
            isProgrammaticResize.current = false;
          }, 200);
        }

        // Avoid forcing position in normal app mode: this was causing drag jitter/snap-back.
        // Keep explicit positioning only for the version gate screen.
        if (
          showVersionScreen &&
          !isDragging.current &&
          (Math.abs(currentX - targetX) > 2 || Math.abs(currentY - targetY) > 2)
        ) {
          console.log(`[Layout] Moving: ${currentX},${currentY} -> ${targetX},${targetY}`);
          await appWindow.setPosition(new LogicalPosition(targetX, targetY));
        }

        await appWindow.setAlwaysOnTop(!showVersionScreen);
        await appWindow.show();
      } catch (e) {
        console.error('[Layout] Sync failed:', e);
        lastLayout.current = '';
        await appWindow.show().catch(() => {});
      }
    };

    void syncLayout();
  }, [
    currentScale,
    expandedHeightRef,
    expandedWidthRef,
    hasCustomPosition,
    isCollapsed,
    isDragging,
    isGeometryLoaded,
    isLoadingGeometry,
    isProgrammaticResize,
    isResizing,
    lastKnownPosition,
    lastUserResize,
    showVersionScreen,
  ]);
};
