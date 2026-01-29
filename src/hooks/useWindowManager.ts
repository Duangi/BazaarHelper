import { useEffect, useRef } from 'react';
import { getCurrentWindow, LogicalPosition, LogicalSize, currentMonitor } from "@tauri-apps/api/window";

interface UseWindowManagerProps {
  showVersionScreen: boolean;
  isCollapsed: boolean;
  hasCustomPosition: boolean;
  expandedWidthRef: React.MutableRefObject<number>;
  expandedHeightRef: React.MutableRefObject<number>;
  lastKnownPosition: React.MutableRefObject<{ x: number; y: number } | null>;
}

export const useWindowManager = ({
  showVersionScreen,
  isCollapsed,
  hasCustomPosition,
  expandedWidthRef,
  expandedHeightRef,
  lastKnownPosition
}: UseWindowManagerProps) => {
  const lastLayout = useRef<string>("");
  const currentScale = useRef(1.0);
  const isDragging = useRef(false);
  const isResizing = useRef(false);
  const lastUserResize = useRef<number>(0);
  const isProgrammaticResize = useRef(false);

  useEffect(() => {
    const syncLayout = async () => {
      const appWindow = getCurrentWindow();
      
      let logicalScale = 1.0;
      let pX = 0, pY = 0, pWidth = 1920, pHeight = 1080;
      
      try {
        const monitor = await currentMonitor();
        if (monitor && monitor.size) {
          logicalScale = monitor.scaleFactor || 1.0;
          pX = monitor.position.x;
          pY = monitor.position.y;
          pWidth = Math.round(monitor.size.width / logicalScale);
          pHeight = Math.round(monitor.size.height / logicalScale);
        }
      } catch (e) {
        // 使用默认值
      }
      
      currentScale.current = logicalScale;

      let targetW = 0, targetH = 0, targetX = 0, targetY = 0;

      if (showVersionScreen) {
        targetW = 600;
        targetH = 850;
        targetX = Math.round(pX + (pWidth - targetW) / 2);
        targetY = Math.round(pY + (pHeight - targetH) / 2);
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
        const currentW = Math.round(size.width);
        const currentH = Math.round(size.height);
        const currentX = Math.round(pos.x / logicalScale);
        const currentY = Math.round(pos.y / logicalScale);

        const layoutKey = `${targetW}-${targetH}-${targetX}-${targetY}`;
        if (lastLayout.current === layoutKey) return;
        lastLayout.current = layoutKey;

        const now = Date.now();
        const recentlyResized = lastUserResize.current && (now - lastUserResize.current < 1000);
        
        // 收起/展开时只改变高度，不改变宽度
        if (!isResizing.current && !recentlyResized && currentH !== targetH) {
          isProgrammaticResize.current = true;
          await appWindow.setSize(new LogicalSize(currentW, targetH));
          setTimeout(() => { isProgrammaticResize.current = false; }, 200);
        }

        if (!isDragging.current && (currentX !== targetX || currentY !== targetY)) {
          await appWindow.setPosition(new LogicalPosition(targetX, targetY));
        }

        await appWindow.setAlwaysOnTop(true);
        await appWindow.show();
      } catch (e) {
        console.error("[Layout] Sync failed:", e);
        lastLayout.current = "";
        await appWindow.show().catch(() => {});
      }
    };

    syncLayout();
  }, [showVersionScreen, isCollapsed, hasCustomPosition]);

  return {
    isDragging,
    isResizing,
    lastUserResize,
    isProgrammaticResize,
    currentScale
  };
};
