import { useCallback } from 'react';
import type { MouseEvent, MutableRefObject } from 'react';
import { currentMonitor, getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

interface UseOverlayActionsOptions {
  expandedHeightRef: MutableRefObject<number>;
  expandedWidthRef: MutableRefObject<number>;
  isCollapsed: boolean;
  setIsCollapsed: (value: boolean) => void;
  isProgrammaticResize: MutableRefObject<boolean>;
  isInputFocused: boolean;
}

const DRAG_ALWAYS_BLOCK_SELECTOR = [
  'button',
  'input',
  'textarea',
  'select',
  'option',
  'a',
  '[role="button"]',
  '[contenteditable="true"]',
  '[data-no-drag]',
  '.no-drag',
].join(',');

const DRAG_SCROLL_AREA_SELECTOR = ['.scroll-area', '.settings-content', '.settings-log-viewer'].join(',');

const MAC_TOP_EDGE_THRESHOLD = 96;
const DEFAULT_TOP_EDGE_THRESHOLD = 12;
const COLLAPSED_ISLAND_WIDTH = 320;
const COLLAPSED_ISLAND_HEIGHT = 54;

const isWindowNearTopEdge = async () => {
  const appWindow = getCurrentWindow();
  const [pos, monitor] = await Promise.all([appWindow.outerPosition(), currentMonitor().catch(() => null)]);
  const monitorTop = monitor?.position.y ?? 0;
  const isMac = /mac/i.test(navigator.userAgent);
  const threshold = isMac ? MAC_TOP_EDGE_THRESHOLD : DEFAULT_TOP_EDGE_THRESHOLD;
  return pos.y <= monitorTop + threshold;
};

export const useOverlayActions = ({
  expandedHeightRef,
  expandedWidthRef,
  isCollapsed,
  setIsCollapsed,
  isProgrammaticResize,
  isInputFocused,
}: UseOverlayActionsOptions) => {
  const applyCollapsedState = useCallback(
    async (nextCollapsed: boolean) => {
      if (nextCollapsed === isCollapsed) return;

      if (expandedHeightRef.current < 200) {
        expandedHeightRef.current = 700;
      }

      try {
        const appWindow = getCurrentWindow();
        const targetW = nextCollapsed
          ? COLLAPSED_ISLAND_WIDTH
          : Math.max(expandedWidthRef.current, COLLAPSED_ISLAND_WIDTH + 40);
        const targetH = nextCollapsed ? COLLAPSED_ISLAND_HEIGHT : expandedHeightRef.current;

        isProgrammaticResize.current = true;
        await appWindow.setSize(new LogicalSize(targetW, targetH));
        setTimeout(() => {
          isProgrammaticResize.current = false;
        }, 300);

        setIsCollapsed(nextCollapsed);
      } catch (e) {
        console.error('Failed to resize on collapse/expand:', e);
        setIsCollapsed(nextCollapsed);
      }
    },
    [expandedHeightRef, expandedWidthRef, isCollapsed, isProgrammaticResize, setIsCollapsed],
  );

  const handleToggleCollapse = useCallback(async () => {
    await applyCollapsedState(!isCollapsed);
  }, [applyCollapsedState, isCollapsed]);

  const handleOverlayMouseLeave = useCallback(
    (_e: MouseEvent<HTMLDivElement>) => {
      if (isInputFocused || isCollapsed) return;

      void (async () => {
        try {
          const nearTop = await isWindowNearTopEdge();
          if (!nearTop) return;
          await applyCollapsedState(true);
        } catch (err) {
          console.error('[Window] auto-collapse failed:', err);
        }
      })();
    },
    [applyCollapsedState, isCollapsed, isInputFocused],
  );

  const handleTopStripMouseEnter = useCallback(() => {
    if (!isCollapsed) return;

    void (async () => {
      try {
        const nearTop = await isWindowNearTopEdge();
        if (!nearTop) return;
        await applyCollapsedState(false);
      } catch (err) {
        console.error('[Window] auto-expand failed:', err);
      }
    })();
  }, [applyCollapsedState, isCollapsed]);

  const handleOverlayMouseDown = useCallback((e: MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (!target) return;
    if (target.closest(DRAG_ALWAYS_BLOCK_SELECTOR)) return;
    if (target.closest(DRAG_SCROLL_AREA_SELECTOR)) return;

    void getCurrentWindow()
      .startDragging()
      .catch((err) => console.error('[Window] startDragging failed:', err));
  }, []);

  return {
    handleToggleCollapse,
    handleOverlayMouseLeave,
    handleTopStripMouseEnter,
    handleOverlayMouseDown,
  };
};
