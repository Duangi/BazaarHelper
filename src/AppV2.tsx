import {
  Suspense,
  lazy,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
} from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import {
  currentMonitor,
  getCurrentWindow,
  LogicalSize,
  LogicalPosition,
  PhysicalPosition,
  PhysicalSize,
} from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

import { MainShell } from './components/layout/MainShell';
import { ToastLayer } from './components/layout/ToastLayer';
import { VersionGateScreen } from './components/settings/VersionGateScreen';
import { AppSettingsPanel } from './components/settings/AppSettingsPanel';
import { useCardRecognition } from './hooks/features/useCardRecognition';
import { useDayTabSelection } from './hooks/features/useDayTabSelection';
import { useItemCardState } from './hooks/features/useItemCardState';
import { useMonsterTabLogic } from './hooks/features/useMonsterTabLogic';
import { useResourceCatalog } from './hooks/features/useResourceCatalog';
import { useTierInfoRenderer } from './hooks/features/useTierInfoRenderer';
import { useUnifiedItemCardRenderer } from './hooks/features/useUnifiedItemCardRenderer';
import { useYoloScan } from './hooks/features/useYoloScan';
import { useSearchPanelState } from './hooks/search/useSearchPanelState';
import { useMatchHistory } from './hooks/sync/useMatchHistory';
import { useMonstersBootstrap } from './hooks/sync/useMonstersBootstrap';
import { useSyncDataPipeline } from './hooks/sync/useSyncDataPipeline';
import { MAC_SCREEN_PROFILE } from './platforms/macos/screenProfile';
import type { IslandStatusType, ItemData, MonsterData, SearchItemLite, SyncPayload, TabType, Toast } from './types';

const TAB_LABELS: Record<TabType, string> = {
  history: '历史战绩',
  monster: '野怪一览',
  card: '卡牌识别',
  items: '手头物品',
  search: '百科搜索',
};

const STARTUP_NOTICE = [
  '集市小抄已启动。',
  '已迁移功能：历史战绩、野怪一览、手头物品。',
  '其余页面正在持续完善。',
].join('\n');

const HistoryViewLazy = lazy(async () => ({
  default: (await import('./views/HistoryView')).HistoryView,
}));

const MonsterTabViewLazy = lazy(async () => ({
  default: (await import('./views/MonsterTabView')).MonsterTabView,
}));

const CardRecognitionViewLazy = lazy(async () => ({
  default: (await import('./views/CardRecognitionView')).CardRecognitionView,
}));

const ItemsViewLazy = lazy(async () => ({
  default: (await import('./views/ItemsView')).ItemsView,
}));

const SearchFiltersPanelLazy = lazy(async () => ({
  default: (await import('./components/search/SearchFiltersPanel')).SearchFiltersPanel,
}));

const VirtualSearchResultsLazy = lazy(async () => ({
  default: (await import('./components/search/VirtualSearchResults')).VirtualSearchResults,
}));

const COLLAPSED_WIDTH = MAC_SCREEN_PROFILE.collapsed.width;
const COLLAPSED_HEIGHT = MAC_SCREEN_PROFILE.collapsed.height;
const MIN_EXPANDED_WIDTH = MAC_SCREEN_PROFILE.expanded.minWidth;
const MIN_EXPANDED_HEIGHT = MAC_SCREEN_PROFILE.expanded.minHeight;
const DEFAULT_EXPANDED_WIDTH = MAC_SCREEN_PROFILE.expanded.defaultWidth;
const DEFAULT_EXPANDED_HEIGHT = MAC_SCREEN_PROFILE.expanded.defaultHeight;
const STARTUP_WIDTH = MAC_SCREEN_PROFILE.startup.width;
const STARTUP_HEIGHT = MAC_SCREEN_PROFILE.startup.height;
const NON_DRAG_SELECTOR = 'button, input, textarea, select, a, [data-no-drag], .no-drag';
const ISLAND_BASE_TEXT = '集市小抄运行中';

const buildSearchSkeletonItem = (item: SearchItemLite): ItemData => ({
  uuid: item.uuid,
  name: item.name,
  name_cn: item.name_cn,
  tier: item.tier,
  available_tiers: item.available_tiers || '',
  size: item.size,
  tags: item.tags || '',
  hidden_tags: item.hidden_tags || '',
  processed_tags: item.processed_tags || [],
  heroes: item.heroes || [],
  cooldown_tiers: '',
  damage_tiers: '',
  heal_tiers: '',
  shield_tiers: '',
  ammo_tiers: '',
  crit_tiers: '',
  multicast_tiers: '',
  burn_tiers: '',
  poison_tiers: '',
  regen_tiers: '',
  lifesteal_tiers: '',
  skills: [],
  enchantments: [],
  description: '',
  image: '',
  displayImg: item.displayImg,
});

export default function AppV2() {
  const appWindow = getCurrentWindow();
  const [showVersionScreen, setShowVersionScreen] = useState(true);
  const [activeTab, setActiveTab] = useState<TabType>('history');
  const [showSettings, setShowSettings] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [selectedDay, setSelectedDay] = useState('Day 1');
  const [currentDay, setCurrentDay] = useState<number | null>(1);
  const [allMonsters, setAllMonsters] = useState<Record<string, MonsterData>>({});
  const [syncData, setSyncData] = useState<SyncPayload & { monster: any[] }>({
    hand_items: [],
    stash_items: [],
    all_tags: [],
    monster: [],
  });
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [fontSize, setFontSize] = useState<number>(() => {
    const saved = localStorage.getItem('user-font-size');
    return saved ? Number.parseInt(saved, 10) : 16;
  });
  const [settingsExpanded, setSettingsExpanded] = useState({
    ui: false,
    yolo: false,
    hotkeys: false,
    debug: false,
  });
  const [enableYoloAuto, setEnableYoloAuto] = useState(() => localStorage.getItem('enable-yolo-auto') === 'true');
  const [useGpuAcceleration, setUseGpuAcceleration] = useState(() => {
    const raw = localStorage.getItem('use-gpu-acceleration');
    return raw === null ? true : raw === 'true';
  });
  const [yoloScanInterval, setYoloScanInterval] = useState(() => {
    const saved = localStorage.getItem('yolo-scan-interval');
    return saved ? Number.parseFloat(saved) : 1.0;
  });
  const [showYoloMonitor, setShowYoloMonitor] = useState(false);
  const [yoloHotkey, setYoloHotkey] = useState<number | null>(() => {
    const saved = localStorage.getItem('yolo-hotkey');
    if (!saved) return null;
    const parsed = Number.parseInt(saved, 10);
    return Number.isFinite(parsed) ? parsed : null;
  });
  const [detectionHotkey, setDetectionHotkey] = useState<number | null>(null);
  const [cardDetectionHotkey, setCardDetectionHotkey] = useState<number | null>(null);
  const [toggleCollapseHotkey, setToggleCollapseHotkey] = useState<number | null>(null);
  const [detailDisplayHotkey, setDetailDisplayHotkey] = useState<number | null>(() => {
    const saved = localStorage.getItem('detail-display-hotkey');
    if (!saved) return null;
    const parsed = Number.parseInt(saved, 10);
    return Number.isFinite(parsed) ? parsed : null;
  });
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [isRecordingCardHotkey, setIsRecordingCardHotkey] = useState(false);
  const [isRecordingToggleHotkey, setIsRecordingToggleHotkey] = useState(false);
  const [isRecordingYoloHotkey, setIsRecordingYoloHotkey] = useState(false);
  const [isRecordingDetailHotkey, setIsRecordingDetailHotkey] = useState(false);
  const [showResetHotkeysConfirm, setShowResetHotkeysConfirm] = useState(false);
  const [, setExpandedWidth] = useState(DEFAULT_EXPANDED_WIDTH);
  const [, setExpandedHeight] = useState(DEFAULT_EXPANDED_HEIGHT);
  const [, setHasCustomPosition] = useState(false);
  const [currentVersion, setCurrentVersion] = useState('1.6.0');
  const [updateStatus, setUpdateStatus] =
    useState<'none' | 'checking' | 'available' | 'downloading' | 'ready'>('none');
  const [updateAvailable, setUpdateAvailable] = useState<Update | null>(null);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [announcement, setAnnouncement] = useState('');
  const [islandStatusType, setIslandStatusType] = useState<IslandStatusType>(
    enableYoloAuto ? 'success' : 'info',
  );
  const [islandPinnedName, setIslandPinnedName] = useState<string | null>(null);
  const readStoredExpandedSize = () => {
    const width = Number(localStorage.getItem('window-expanded-width')) || DEFAULT_EXPANDED_WIDTH;
    const height = Number(localStorage.getItem('window-expanded-height')) || DEFAULT_EXPANDED_HEIGHT;
    return { width, height };
  };
  const expandedSizeRef = useRef<{ width: number; height: number }>(readStoredExpandedSize());
  const isSwitchingRef = useRef(false);
  const isDraggingWindowRef = useRef(false);
  const dragGuardUntilRef = useRef(0);
  const lastCollapsedAtRef = useRef(0);
  const showVersionScreenRef = useRef(showVersionScreen);
  const restoringMainLayoutRef = useRef(false);
  const hasRestoredMainGeometryRef = useRef(false);
  const moveSaveTimerRef = useRef<number | null>(null);
  const resizeSaveTimerRef = useRef<number | null>(null);

  useEffect(() => {
    showVersionScreenRef.current = showVersionScreen;
  }, [showVersionScreen]);

  const getMonitorLogicalBounds = useCallback(async () => {
    const monitor = await currentMonitor().catch(() => null);
    if (!monitor) {
      return {
        maxWidth: 1600,
        maxHeight: 1000,
      };
    }
    const logicalWidth = Math.floor(monitor.size.width / monitor.scaleFactor);
    const logicalHeight = Math.floor(monitor.size.height / monitor.scaleFactor);
    return {
      maxWidth: Math.max(MIN_EXPANDED_WIDTH, logicalWidth - MAC_SCREEN_PROFILE.monitorMargin.width),
      maxHeight: Math.max(MIN_EXPANDED_HEIGHT, logicalHeight - MAC_SCREEN_PROFILE.monitorMargin.height),
    };
  }, []);

  const clampExpandedSize = useCallback(
    async (raw: { width: number; height: number }) => {
      const { maxWidth, maxHeight } = await getMonitorLogicalBounds();
      return {
        width: Math.min(maxWidth, Math.max(MIN_EXPANDED_WIDTH, Math.round(raw.width))),
        height: Math.min(maxHeight, Math.max(MIN_EXPANDED_HEIGHT, Math.round(raw.height))),
      };
    },
    [getMonitorLogicalBounds],
  );

  const persistExpandedSize = useCallback((size: { width: number; height: number }) => {
    expandedSizeRef.current = size;
    setExpandedWidth(size.width);
    setExpandedHeight(size.height);
    localStorage.setItem('window-expanded-width', String(size.width));
    localStorage.setItem('window-expanded-height', String(size.height));
  }, [setExpandedHeight, setExpandedWidth]);

  const ensureWindowLogicalSize = useCallback(
    async (logicalWidth: number, logicalHeight: number): Promise<boolean> => {
      const checkCloseEnough = async () => {
        const [inner, scale] = await Promise.all([
          appWindow.innerSize().catch(() => null),
          appWindow.scaleFactor().catch(() => 1),
        ]);
        if (!inner) return false;
        const currentLogicalWidth = inner.width / Math.max(1, scale);
        const currentLogicalHeight = inner.height / Math.max(1, scale);
        const diffW = Math.abs(currentLogicalWidth - logicalWidth);
        const diffH = Math.abs(currentLogicalHeight - logicalHeight);
        return diffW <= 8 && diffH <= 8;
      };

      try {
        await appWindow.setSize(new LogicalSize(logicalWidth, logicalHeight));
      } catch (error) {
        console.warn('[AppV2] set logical size failed:', error);
      }

      if (await checkCloseEnough()) {
        return true;
      }

      const scale = await appWindow.scaleFactor().catch(() => 1);
      try {
        await appWindow.setSize(
          new PhysicalSize(
            Math.max(1, Math.round(logicalWidth * scale)),
            Math.max(1, Math.round(logicalHeight * scale)),
          ),
        );
      } catch (error) {
        console.warn('[AppV2] set physical size fallback failed:', error);
      }

      return checkCloseEnough();
    },
    [appWindow],
  );

  useEffect(() => {
    // Clamp possibly-corrupted saved size once on mount.
    void (async () => {
      const clamped = await clampExpandedSize(expandedSizeRef.current);
      persistExpandedSize(clamped);
    })();
  }, [clampExpandedSize, persistExpandedSize]);

  const centerStartupWindow = useCallback(async () => {
    try {
      await appWindow.setSize(new LogicalSize(STARTUP_WIDTH, STARTUP_HEIGHT));
      await appWindow.center();
      await appWindow.show();
    } catch (error) {
      console.warn('[AppV2] center startup window failed:', error);
    }
  }, [appWindow]);

  const restoreMainWindowGeometry = useCallback(async () => {
    if (restoringMainLayoutRef.current) return;
    restoringMainLayoutRef.current = true;
    try {
      const geometry = await invoke<{
        x?: number | null;
        y?: number | null;
        width?: number | null;
        height?: number | null;
      }>('get_window_geometry', { windowLabel: 'main' }).catch(() => null);

      // Important: avoid immediate setSize during version-screen -> main transition on macOS.
      // This transition can trigger WebView/NSPanel instability on some machines.
      // We only restore position now; expanded size is stored as future target.
      if (
        geometry
        && typeof geometry.width === 'number'
        && typeof geometry.height === 'number'
        && geometry.width > 100
        && geometry.height > 100
      ) {
        const scale = await appWindow.scaleFactor().catch(() => 1);
        const restoredLogical = await clampExpandedSize({
          width: geometry.width / Math.max(1, scale),
          height: geometry.height / Math.max(1, scale),
        });
        persistExpandedSize(restoredLogical);
      } else {
        const safe = await clampExpandedSize(expandedSizeRef.current);
        persistExpandedSize(safe);
      }

      let targetX: number | null = null;
      let targetY: number | null = null;
      if (geometry && typeof geometry.x === 'number' && typeof geometry.y === 'number') {
        targetX = geometry.x;
        targetY = geometry.y;
      } else {
        const rawX = localStorage.getItem('plugin-pos-x');
        const rawY = localStorage.getItem('plugin-pos-y');
        if (rawX && rawY) {
          const parsedX = Number.parseInt(rawX, 10);
          const parsedY = Number.parseInt(rawY, 10);
          if (Number.isFinite(parsedX) && Number.isFinite(parsedY)) {
            targetX = parsedX;
            targetY = parsedY;
          }
        }
      }

      let restoredByNative = false;
      const nativeRestore = await invoke<{ applied?: boolean }>('restore_main_window_geometry').catch(() => null);
      if (nativeRestore?.applied) {
        restoredByNative = true;
        setHasCustomPosition(true);
      }

      if (!restoredByNative && targetX != null && targetY != null) {
        // Fallback for old clients/localStorage-only state.
        try {
          await appWindow.setPosition(new LogicalPosition(targetX, targetY));
        } catch {
          await appWindow.setPosition(new PhysicalPosition(targetX, targetY));
        }
        setHasCustomPosition(true);
      }

      await appWindow.show();
    } catch (error) {
      console.warn('[AppV2] restore main window geometry failed:', error);
    } finally {
      restoringMainLayoutRef.current = false;
    }
  }, [appWindow, clampExpandedSize, persistExpandedSize]);

  useEffect(() => {
    if (showVersionScreen) {
      void centerStartupWindow();
    }
  }, [centerStartupWindow, showVersionScreen]);

  useEffect(() => {
    if (showVersionScreen) {
      hasRestoredMainGeometryRef.current = false;
      return undefined;
    }
    if (hasRestoredMainGeometryRef.current) {
      return undefined;
    }
    hasRestoredMainGeometryRef.current = true;
    const timer1 = window.setTimeout(() => {
      void restoreMainWindowGeometry();
    }, MAC_SCREEN_PROFILE.timing.restoreLayoutDelayMs);
    // Re-apply once more to override potential late center/show operations from startup view.
    const timer2 = window.setTimeout(() => {
      void restoreMainWindowGeometry();
    }, MAC_SCREEN_PROFILE.timing.restoreLayoutRetryDelayMs);
    return () => {
      window.clearTimeout(timer1);
      window.clearTimeout(timer2);
    };
  }, [restoreMainWindowGeometry, showVersionScreen]);

  useEffect(() => {
    let unlistenMove: (() => void) | null = null;
    let unlistenResize: (() => void) | null = null;

    const setupListeners = async () => {
      unlistenMove = await appWindow.listen<{ x: number; y: number }>('tauri://move', (event) => {
        if (showVersionScreenRef.current || restoringMainLayoutRef.current) return;
        setHasCustomPosition(true);
        localStorage.setItem('plugin-pos-x', String(event.payload.x));
        localStorage.setItem('plugin-pos-y', String(event.payload.y));

        if (moveSaveTimerRef.current) {
          window.clearTimeout(moveSaveTimerRef.current);
        }
        moveSaveTimerRef.current = window.setTimeout(() => {
          void invoke('save_window_geometry', {
            windowLabel: 'main',
            x: event.payload.x,
            y: event.payload.y,
          }).catch((error) => console.warn('[AppV2] save move geometry failed:', error));
        }, MAC_SCREEN_PROFILE.timing.saveMoveDebounceMs);
      });

      unlistenResize = await appWindow.listen('tauri://resize', () => {
        if (showVersionScreenRef.current || restoringMainLayoutRef.current || isCollapsed) return;
        if (resizeSaveTimerRef.current) {
          window.clearTimeout(resizeSaveTimerRef.current);
        }
        resizeSaveTimerRef.current = window.setTimeout(() => {
          void (async () => {
            try {
              const [inner, scale] = await Promise.all([
                appWindow.innerSize(),
                appWindow.scaleFactor().catch(() => 1),
              ]);
              if (inner.width <= COLLAPSED_WIDTH + 30 || inner.height <= COLLAPSED_HEIGHT + 30) {
                return;
              }
              const clamped = await clampExpandedSize({
                width: inner.width / Math.max(1, scale),
                height: inner.height / Math.max(1, scale),
              });
              persistExpandedSize(clamped);
              await invoke('save_window_geometry', {
                windowLabel: 'main',
                width: inner.width,
                height: inner.height,
              });
            } catch (error) {
              console.warn('[AppV2] save resize geometry failed:', error);
            }
          })();
        }, MAC_SCREEN_PROFILE.timing.saveResizeDebounceMs);
      });
    };

    void setupListeners();
    return () => {
      if (unlistenMove) unlistenMove();
      if (unlistenResize) unlistenResize();
      if (moveSaveTimerRef.current) {
        window.clearTimeout(moveSaveTimerRef.current);
      }
      if (resizeSaveTimerRef.current) {
        window.clearTimeout(resizeSaveTimerRef.current);
      }
    };
  }, [appWindow, clampExpandedSize, isCollapsed, persistExpandedSize]);

  const applyCollapsedState = useCallback(
    async (nextCollapsed: boolean) => {
      if (isSwitchingRef.current) return;
      if (nextCollapsed === isCollapsed) return;

      isSwitchingRef.current = true;
      try {
        if (nextCollapsed) {
          const [current, scale] = await Promise.all([
            appWindow.innerSize().catch(() => null),
            appWindow.scaleFactor().catch(() => 1),
          ]);
          if (
            current
            && current.width > COLLAPSED_WIDTH + 40
            && current.height > COLLAPSED_HEIGHT + 40
          ) {
            // innerSize() is physical px; normalize to logical px before persisting.
            const normalized = await clampExpandedSize({
              width: current.width / Math.max(1, scale),
              height: current.height / Math.max(1, scale),
            });
            persistExpandedSize(normalized);
          }
          const resized = await ensureWindowLogicalSize(COLLAPSED_WIDTH, COLLAPSED_HEIGHT);
          if (!resized) {
            throw new Error('failed to apply collapsed window size');
          }
          lastCollapsedAtRef.current = Date.now();
        } else {
          const safe = await clampExpandedSize(expandedSizeRef.current);
          persistExpandedSize(safe);
          const resized = await ensureWindowLogicalSize(safe.width, safe.height);
          if (!resized) {
            throw new Error('failed to apply expanded window size');
          }
        }
        setIsCollapsed(nextCollapsed);
      } catch (error) {
        console.error('[AppV2] collapse/expand resize failed:', error);
      } finally {
        isSwitchingRef.current = false;
      }
    },
    [appWindow, clampExpandedSize, ensureWindowLogicalSize, isCollapsed, persistExpandedSize],
  );

  const handleOverlayMouseDown = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      if (event.button !== 0) return;
      const target = event.target as HTMLElement | null;
      if (!target) return;
      if (target.closest(NON_DRAG_SELECTOR)) return;
      const now = Date.now();
      dragGuardUntilRef.current = now + 900;
      isDraggingWindowRef.current = true;
      void appWindow.startDragging().catch((err) => {
        isDraggingWindowRef.current = false;
        console.error('[AppV2] startDragging failed:', err);
      });
    },
    [appWindow],
  );

  const handleOverlayMouseOutCapture = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      if (isCollapsed) return;
      if (isDraggingWindowRef.current || Date.now() < dragGuardUntilRef.current) return;
      if (event.relatedTarget) return;
      void applyCollapsedState(true);
    },
    [applyCollapsedState, isCollapsed],
  );

  useEffect(() => {
    const releaseDragging = () => {
      isDraggingWindowRef.current = false;
    };
    window.addEventListener('mouseup', releaseDragging);
    window.addEventListener('blur', releaseDragging);
    return () => {
      window.removeEventListener('mouseup', releaseDragging);
      window.removeEventListener('blur', releaseDragging);
    };
  }, []);

  const panelTitle = useMemo(() => {
    if (showSettings) return '系统设置';
    return TAB_LABELS[activeTab];
  }, [activeTab, showSettings]);
  const islandStatusText = islandPinnedName ? `当前识别: ${islandPinnedName}` : ISLAND_BASE_TEXT;
  const historyEnabled = !showVersionScreen && !isCollapsed && !showSettings && activeTab === 'history';
  const monsterEnabled = !showVersionScreen && !isCollapsed && !showSettings && activeTab === 'monster';
  const cardEnabled = !showVersionScreen && !isCollapsed && !showSettings && activeTab === 'card';
  const itemsEnabled = !showVersionScreen && !isCollapsed && !showSettings && activeTab === 'items';
  const searchEnabled = !showVersionScreen && !isCollapsed && !showSettings && activeTab === 'search';
  const resourceCatalogEnabled = monsterEnabled || showSettings || searchEnabled;
  const { matchHistory, isLoadingHistory, loadMatchHistory } = useMatchHistory({
    enabled: historyEnabled,
  });
  const updateDayTabSelection = useDayTabSelection(setSelectedDay);
  useMonstersBootstrap({
    enabled: monsterEnabled,
    setAllMonsters,
  });
  const { hiddenTagIcons, skillsArtMap, itemSizes, sponsorIcons } = useResourceCatalog(resourceCatalogEnabled);
  const {
    manualMonsters,
    identifiedNames,
    expandedMonsters,
    isRecognizing,
    toggleMonsterExpand,
    handleDayChange,
    handleAutoRecognition,
  } = useMonsterTabLogic({
    enabled: !showVersionScreen && !isCollapsed,
    activeTab,
    selectedDay,
    setSelectedDay,
    allMonsters,
    setCurrentDay: (day: number) => setCurrentDay(day),
    skillsArtMap,
    updateDayTabSelection,
  });
  const { renderTierInfo } = useTierInfoRenderer(itemSizes);
  const {
    searchQuery,
    setSearchQuery,
    searchResults,
    isSearching,
    isSearchFilterCollapsed,
    setIsSearchFilterCollapsed,
    selectedTags,
    setSelectedTags,
    selectedHiddenTags,
    setSelectedHiddenTags,
    matchMode,
    setMatchMode,
    setIsInputFocused,
    lastItemSize,
    setLastItemSize,
    searchFilterHeight,
    isResizingFilter,
    setResizeStartY,
    setResizeStartHeight,
    setIsResizingFilter,
    scrollAreaRef,
    handleScroll,
    searchScrollTop,
    searchViewportHeight,
  } = useSearchPanelState({
    activeTab,
    selectedDay,
    skillsArtMap,
  });
  const [searchDetailMap, setSearchDetailMap] = useState<Record<string, ItemData>>({});
  const [searchDetailLoading, setSearchDetailLoading] = useState<Set<string>>(new Set());
  const [searchDetailMissing, setSearchDetailMissing] = useState<Set<string>>(new Set());

  const showToast = useCallback((message: string, type: Toast['type'] = 'info') => {
    const id = Date.now() + Math.floor(Math.random() * 1000);
    setToasts((prev) => [...prev, { id, message, type }]);
  }, []);
  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);
  const updateIslandStatus = useCallback(
    (message: string, type: IslandStatusType = 'info') => {
      const text = (message || '').trim();
      if (!text) return;

      // Keep island text stable; YOLO scanning only updates the status dot.
      if (text.startsWith('YOLO(自动):')) {
        if (enableYoloAuto) {
          setIslandStatusType('success');
        }
        return;
      }
      if (text.startsWith('YOLO(手动):')) {
        if (text.includes('正在扫描')) {
          setIslandStatusType('warning');
        } else if (text.includes('扫描完成')) {
          setIslandStatusType(text.includes('共 0') ? 'warning' : 'success');
        }
        return;
      }

      if (text.startsWith('卡牌详情:')) {
        const raw = text.replace(/^卡牌详情:\s*/, '');
        const name = raw.split('(')[0]?.trim() || '未知卡牌';
        setIslandPinnedName(name);
        setIslandStatusType('success');
        return;
      }

      if (text.includes('详情识别失败')) {
        setIslandStatusType('error');
        return;
      }
      if (text.includes('未识别到详情目标')) {
        setIslandStatusType(enableYoloAuto ? 'success' : 'info');
        return;
      }

      setIslandStatusType(type);
    },
    [enableYoloAuto],
  );

  useEffect(() => {
    if (!islandPinnedName) {
      setIslandStatusType(enableYoloAuto ? 'success' : 'info');
    }
  }, [enableYoloAuto, islandPinnedName]);
  const { processSyncPayload } = useSyncDataPipeline({ setSyncData });
  const { pinnedItems, expandedItems, setExpandedItems, togglePin, toggleExpand, getSortedItems } =
    useItemCardState();
  const { recognizedCards, isRecognizingCard, handleRecognizeCard } = useCardRecognition({
    setActiveTab,
    setExpandedItems,
    setErrorMessage,
    showToast,
  });
  const renderUnifiedItemCard = useUnifiedItemCardRenderer({
    expandedItems,
    activeTab,
    recognizedCards,
    allTags: syncData.all_tags || [],
    toggleExpand,
  });

  const loadItemsSnapshot = useCallback(async () => {
    try {
      const state = await invoke<any>('get_sync_state');
      if (!state) return;
      await processSyncPayload({
        hand_items: state.hand_items || [],
        stash_items: state.stash_items || [],
        all_tags: state.all_tags || [],
      });
    } catch (error) {
      console.warn('[AppV2] loadItemsSnapshot failed:', error);
    }
  }, [processSyncPayload]);

  const handleManualCheckUpdate = useCallback(async () => {
    setUpdateStatus('checking');
    try {
      const updater = await check();
      if (updater) {
        setUpdateAvailable(updater);
        setUpdateStatus('available');
      } else {
        setUpdateAvailable(null);
        setUpdateStatus('none');
      }
    } catch (error) {
      console.error('[AppV2] Manual update check failed:', error);
      setUpdateStatus('none');
      showToast('检查更新失败', 'error');
    }
  }, [showToast]);

  const startUpdateDownload = useCallback(async () => {
    if (!updateAvailable) return;
    try {
      setUpdateStatus('downloading');
      setDownloadProgress(0);
      let contentLength = 0;
      let downloaded = 0;
      await updateAvailable.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength || 0;
            setDownloadProgress(0);
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setDownloadProgress(Math.min(100, Math.round((downloaded / contentLength) * 100)));
            }
            break;
          case 'Finished':
            setUpdateStatus('ready');
            setDownloadProgress(100);
            break;
        }
      });
    } catch (error) {
      console.error('[AppV2] Update download failed:', error);
      setUpdateStatus('available');
      showToast('更新下载失败', 'error');
    }
  }, [showToast, updateAvailable]);

  const handleInstallReady = useCallback(() => {
    setTimeout(() => {
      void invoke('prepare_app_exit')
        .catch(() => {})
        .finally(() => {
          void relaunch();
        });
    }, 1000);
  }, []);

  const handleConfirmResetHotkeys = useCallback(async () => {
    setShowResetHotkeysConfirm(false);
    try {
      await invoke('reset_all_hotkeys');
      const [detection, card, toggle, yolo, detail] = await Promise.all([
        invoke<number | null>('get_detection_hotkey'),
        invoke<number | null>('get_card_detection_hotkey'),
        invoke<number | null>('get_toggle_collapse_hotkey'),
        invoke<number | null>('get_yolo_hotkey'),
        invoke<number | null>('get_detail_display_hotkey'),
      ]);
      setDetectionHotkey(detection || 0);
      setCardDetectionHotkey(card || 0);
      setToggleCollapseHotkey(toggle || 0);
      setYoloHotkey(yolo || 0);
      setDetailDisplayHotkey(detail || 0);
      localStorage.removeItem('detection-hotkey');
      localStorage.removeItem('card-detection-hotkey');
      localStorage.removeItem('toggle-collapse-hotkey');
      localStorage.removeItem('yolo-hotkey');
      localStorage.removeItem('detail-display-hotkey');
      showToast('所有快捷键已重置', 'success');
    } catch (error) {
      console.error('[AppV2] Reset hotkeys failed:', error);
      showToast('重置快捷键失败', 'error');
    }
  }, [showToast]);

  useYoloScan({
    enabled: !showVersionScreen,
    enableYoloAuto,
    isCollapsed,
    yoloScanInterval,
    yoloHotkey,
    setErrorMessage,
    updateIslandStatus,
    showToast,
  });

  useEffect(() => {
    if (!searchEnabled) return;
    const currentIds = new Set(searchResults.map((item) => item.uuid));
    setSearchDetailMap((prev) => {
      const next: Record<string, ItemData> = {};
      Object.entries(prev).forEach(([id, detail]) => {
        if (currentIds.has(id)) next[id] = detail;
      });
      return next;
    });
    setSearchDetailLoading((prev) => {
      const next = new Set<string>();
      prev.forEach((id) => {
        if (currentIds.has(id)) next.add(id);
      });
      return next;
    });
    setSearchDetailMissing((prev) => {
      const next = new Set<string>();
      prev.forEach((id) => {
        if (currentIds.has(id)) next.add(id);
      });
      return next;
    });
  }, [searchEnabled, searchResults]);

  useEffect(() => {
    if (searchEnabled) return;
    if (Object.keys(searchDetailMap).length > 0) setSearchDetailMap({});
    if (searchDetailLoading.size > 0) setSearchDetailLoading(new Set());
    if (searchDetailMissing.size > 0) setSearchDetailMissing(new Set());
  }, [searchEnabled, searchDetailLoading, searchDetailMap, searchDetailMissing]);

  useEffect(() => {
    if (!searchEnabled) return;
    const expandedSearchIds = searchResults
      .filter((item) => expandedItems.has(item.uuid))
      .map((item) => item.uuid)
      .filter((id) => !searchDetailMap[id] && !searchDetailLoading.has(id) && !searchDetailMissing.has(id));

    if (expandedSearchIds.length === 0) return;

    setSearchDetailLoading((prev) => {
      const next = new Set(prev);
      expandedSearchIds.forEach((id) => next.add(id));
      return next;
    });

    let cancelled = false;
    const fetchDetails = async () => {
      const settled = await Promise.all(
        expandedSearchIds.map(async (id) => {
          try {
            const detail = await invoke<ItemData | null>('get_item_info', { id });
            return { id, detail };
          } catch (error) {
            console.error('[Search] load detail failed:', id, error);
            return { id, detail: null };
          }
        }),
      );

      if (cancelled) {
        setSearchDetailLoading((prev) => {
          const next = new Set(prev);
          expandedSearchIds.forEach((id) => next.delete(id));
          return next;
        });
        return;
      }

      const missingIds: string[] = [];
      const foundIds: string[] = [];

      setSearchDetailMap((prev) => {
        const next = { ...prev };
        settled.forEach(({ id, detail }) => {
          if (detail) {
            next[id] = detail;
            foundIds.push(id);
          } else {
            missingIds.push(id);
          }
        });
        return next;
      });

      setSearchDetailLoading((prev) => {
        const next = new Set(prev);
        expandedSearchIds.forEach((id) => next.delete(id));
        return next;
      });

      if (missingIds.length > 0 || foundIds.length > 0) {
        setSearchDetailMissing((prev) => {
          const next = new Set(prev);
          foundIds.forEach((id) => next.delete(id));
          missingIds.forEach((id) => next.add(id));
          return next;
        });
      }
    };

    void fetchDetails();
    return () => {
      cancelled = true;
      setSearchDetailLoading((prev) => {
        const next = new Set(prev);
        expandedSearchIds.forEach((id) => next.delete(id));
        return next;
      });
    };
  }, [searchEnabled, expandedItems, searchResults]);

  useEffect(() => {
    if (showVersionScreen || !itemsEnabled) return;
    let mounted = true;

    void loadItemsSnapshot();
    const unlistenPromise = listen<SyncPayload>('sync-items', (event) => {
      if (!mounted) return;
      void processSyncPayload(event.payload);
    });

    return () => {
      mounted = false;
      void unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, [itemsEnabled, loadItemsSnapshot, processSyncPayload, showVersionScreen]);

  useEffect(() => {
    let mounted = true;
    void (async () => {
      try {
        const version = await getVersion();
        if (mounted) setCurrentVersion(version);
      } catch {
        // ignore
      }
    })();

    const endpoint = 'https://gh.llkk.cc/https://raw.githubusercontent.com/Duangi/BazaarHelper/main/update.json';
    fetch(`${endpoint}?t=${Date.now()}`)
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (!mounted) return;
        setAnnouncement(typeof data?.notes === 'string' ? data.notes : '');
      })
      .catch(() => {
        if (mounted) setAnnouncement('');
      });

    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    if (showVersionScreen || !showSettings) return;
    let mounted = true;
    void Promise.all([
      invoke<number | null>('get_detection_hotkey'),
      invoke<number | null>('get_card_detection_hotkey'),
      invoke<number | null>('get_toggle_collapse_hotkey'),
      invoke<number | null>('get_yolo_hotkey'),
      invoke<number | null>('get_detail_display_hotkey'),
    ]).then(([detection, card, toggle, yolo, detail]) => {
      if (!mounted) return;
      setDetectionHotkey(detection);
      setCardDetectionHotkey(card);
      setToggleCollapseHotkey(toggle);
      setYoloHotkey(yolo);
      setDetailDisplayHotkey(detail);
    }).catch((error) => {
      console.warn('[AppV2] failed to load hotkeys:', error);
    });

    return () => {
      mounted = false;
    };
  }, [showSettings, showVersionScreen]);

  useEffect(() => {
    if (showVersionScreen) return;
    let mounted = true;
    const unlistenPromise = listen<void>('hotkey-card', () => {
      if (!mounted) return;
      void handleRecognizeCard(true);
    });
    return () => {
      mounted = false;
      void unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, [showVersionScreen, handleRecognizeCard]);

  useEffect(() => {
    if (showVersionScreen) return;
    let mounted = true;
    const unlistenPromise = listen<{ message?: string; type?: IslandStatusType }>('island-status', (event) => {
      if (!mounted) return;
      const message = event.payload?.message;
      if (!message) return;
      const type = event.payload?.type ?? 'info';
      updateIslandStatus(message, type);
      if (!isCollapsed && !message.startsWith('YOLO(') && !message.includes('正在识别详情')) {
        showToast(message, type as Toast['type']);
      }
    });
    return () => {
      mounted = false;
      void unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, [isCollapsed, showVersionScreen, showToast, updateIslandStatus]);

  useEffect(() => {
    if (showVersionScreen) return;
    let mounted = true;
    const unlistenPromise = listen<{ visible?: boolean; name?: string }>(
      'detail-popup-visibility',
      (event) => {
        if (!mounted) return;
        const visible = event.payload?.visible === true;
        if (visible) {
          const name = (event.payload?.name || '').trim();
          if (name) {
            setIslandPinnedName(name);
            setIslandStatusType('success');
          }
          return;
        }
        setIslandPinnedName(null);
        setIslandStatusType(enableYoloAuto ? 'success' : 'info');
      },
    );

    return () => {
      mounted = false;
      void unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, [enableYoloAuto, showVersionScreen]);

  useEffect(() => {
    if (showVersionScreen) return;
    // Pre-create detail popup window to avoid first-trigger race in fullscreen/game mode.
    void invoke('warmup_detail_popup').catch((error) => {
      console.warn('[AppV2] warmup_detail_popup failed:', error);
    });
  }, [showVersionScreen]);

  useEffect(() => {
    if (showVersionScreen) return;
    if (detailDisplayHotkey == null) return;
    void invoke('set_detail_display_hotkey', { hotkey: detailDisplayHotkey }).catch((error) => {
      console.warn('[AppV2] set_detail_display_hotkey failed:', error);
    });
  }, [detailDisplayHotkey, showVersionScreen]);

  useEffect(() => {
    if (detailDisplayHotkey == null) {
      localStorage.removeItem('detail-display-hotkey');
      return;
    }
    localStorage.setItem('detail-display-hotkey', String(detailDisplayHotkey));
  }, [detailDisplayHotkey]);

  if (showVersionScreen) {
    return (
      <VersionGateScreen
        fontSize={16}
        announcement={STARTUP_NOTICE}
        currentVersion={currentVersion}
        updateStatus="none"
        downloadProgress={0}
        onStartUpdateDownload={() => {}}
        onInstallReady={() => {}}
        onEnterApp={() => {
          setShowVersionScreen(false);
        }}
      />
    );
  }

  return (
    <div
      className={`overlay ${isCollapsed ? 'collapsed' : 'expanded'}`}
      style={
        {
          '--user-font-size': '16px',
          '--font-scale': 1,
        } as CSSProperties
      }
      onMouseDownCapture={handleOverlayMouseDown}
      onMouseOutCapture={handleOverlayMouseOutCapture}
      onMouseLeave={() => {
        if (!isCollapsed) {
          if (isDraggingWindowRef.current || Date.now() < dragGuardUntilRef.current) return;
          void applyCollapsedState(true);
        }
      }}
    >
      <div
        className="window-drag-strip"
        data-tauri-drag-region
        onMouseEnter={() => {
          if (isCollapsed) {
            if (Date.now() - lastCollapsedAtRef.current < 220) {
              return;
            }
            void applyCollapsedState(false);
          }
        }}
      />

      {isCollapsed ? (
        <div className="dynamic-island-chip" data-tauri-drag-region>
          <span className={`dynamic-island-dot ${islandStatusType}`} />
          <span className="dynamic-island-text">{islandStatusText}</span>
        </div>
      ) : (
        <MainShell
          activeTab={activeTab}
          onTabChange={(tab) => {
            setShowSettings(false);
            setActiveTab(tab);
          }}
          onOpenSettings={() => setShowSettings(true)}
        >
          {!showSettings && searchEnabled ? (
            <Suspense fallback={null}>
              <SearchFiltersPanelLazy
                isSearchFilterCollapsed={isSearchFilterCollapsed}
                setIsSearchFilterCollapsed={setIsSearchFilterCollapsed}
                matchMode={matchMode}
                setMatchMode={setMatchMode}
                searchFilterHeight={searchFilterHeight}
                searchQuery={searchQuery}
                setSearchQuery={setSearchQuery}
                setIsInputFocused={setIsInputFocused}
                lastItemSize={lastItemSize}
                setLastItemSize={setLastItemSize}
                selectedTags={selectedTags}
                setSelectedTags={setSelectedTags}
                selectedHiddenTags={selectedHiddenTags}
                setSelectedHiddenTags={setSelectedHiddenTags}
                hiddenTagIcons={hiddenTagIcons}
                isSearching={isSearching}
                searchResultsCount={searchResults.length}
                setResizeStartY={setResizeStartY}
                setResizeStartHeight={setResizeStartHeight}
                setIsResizingFilter={setIsResizingFilter}
                isResizingFilter={isResizingFilter}
              />
            </Suspense>
          ) : null}
          <div className="scroll-area" ref={scrollAreaRef} onScroll={handleScroll} data-no-drag>
            <div className="items">
              {showSettings ? (
                <AppSettingsPanel
                  visible
                  inline
                  onClose={() => setShowSettings(false)}
                  settingsExpanded={settingsExpanded}
                  setSettingsExpanded={setSettingsExpanded}
                  fontSize={fontSize}
                  setFontSize={setFontSize}
                  setExpandedWidth={setExpandedWidth}
                  setExpandedHeight={setExpandedHeight}
                  setHasCustomPosition={setHasCustomPosition}
                  showToast={showToast}
                  enableYoloAuto={enableYoloAuto}
                  setEnableYoloAuto={setEnableYoloAuto}
                  useGpuAcceleration={useGpuAcceleration}
                  setUseGpuAcceleration={setUseGpuAcceleration}
                  yoloScanInterval={yoloScanInterval}
                  setYoloScanInterval={setYoloScanInterval}
                  showYoloMonitor={showYoloMonitor}
                  setShowYoloMonitor={setShowYoloMonitor}
                  yoloHotkey={yoloHotkey}
                  setYoloHotkey={setYoloHotkey}
                  isRecordingYoloHotkey={isRecordingYoloHotkey}
                  setIsRecordingYoloHotkey={setIsRecordingYoloHotkey}
                  detailDisplayHotkey={detailDisplayHotkey}
                  setDetailDisplayHotkey={setDetailDisplayHotkey}
                  isRecordingDetailHotkey={isRecordingDetailHotkey}
                  setIsRecordingDetailHotkey={setIsRecordingDetailHotkey}
                  detectionHotkey={detectionHotkey}
                  setDetectionHotkey={setDetectionHotkey}
                  isRecordingHotkey={isRecordingHotkey}
                  setIsRecordingHotkey={setIsRecordingHotkey}
                  cardDetectionHotkey={cardDetectionHotkey}
                  setCardDetectionHotkey={setCardDetectionHotkey}
                  isRecordingCardHotkey={isRecordingCardHotkey}
                  setIsRecordingCardHotkey={setIsRecordingCardHotkey}
                  toggleCollapseHotkey={toggleCollapseHotkey}
                  setToggleCollapseHotkey={setToggleCollapseHotkey}
                  isRecordingToggleHotkey={isRecordingToggleHotkey}
                  setIsRecordingToggleHotkey={setIsRecordingToggleHotkey}
                  showResetHotkeysConfirm={showResetHotkeysConfirm}
                  setShowResetHotkeysConfirm={setShowResetHotkeysConfirm}
                  onConfirmResetHotkeys={handleConfirmResetHotkeys}
                  currentVersion={currentVersion}
                  updateStatus={updateStatus}
                  downloadProgress={downloadProgress}
                  updateAvailableVersion={updateAvailable?.version}
                  onManualCheckUpdate={handleManualCheckUpdate}
                  onStartUpdateDownload={() => {
                    void startUpdateDownload();
                  }}
                  onInstallReady={handleInstallReady}
                  announcement={announcement}
                  sponsorIcons={sponsorIcons}
                />
              ) : historyEnabled ? (
                <Suspense fallback={<div className="empty-tip">加载历史战绩中...</div>}>
                  <HistoryViewLazy
                    records={matchHistory}
                    isLoading={isLoadingHistory}
                    onReload={() => {
                      void loadMatchHistory();
                    }}
                  />
                </Suspense>
              ) : monsterEnabled ? (
                <Suspense fallback={<div className="empty-tip">加载野怪一览中...</div>}>
                  <MonsterTabViewLazy
                    selectedDay={selectedDay}
                    setSelectedDay={setSelectedDay}
                    handleDayChange={(day) => {
                      void handleDayChange(day);
                    }}
                    isRecognizing={isRecognizing}
                    handleAutoRecognition={handleAutoRecognition}
                    showToast={showToast}
                    templateLoading={{
                      loaded: 0,
                      total: 0,
                      is_complete: true,
                      current_name: '',
                    }}
                    manualMonsters={manualMonsters}
                    identifiedNames={identifiedNames}
                    expandedMonsters={expandedMonsters}
                    toggleMonsterExpand={toggleMonsterExpand}
                    renderTierInfo={renderTierInfo}
                  />
                </Suspense>
              ) : cardEnabled ? (
                <Suspense fallback={<div className="empty-tip">加载卡牌识别中...</div>}>
                  <CardRecognitionViewLazy
                    recognizedCards={recognizedCards}
                    isRecognizing={isRecognizingCard}
                    expandedItems={expandedItems}
                    onToggleExpand={toggleExpand}
                    onRecognize={() => {
                      void handleRecognizeCard(false);
                    }}
                    renderItemCard={renderUnifiedItemCard}
                  />
                </Suspense>
              ) : itemsEnabled ? (
                <Suspense fallback={<div className="empty-tip">加载手头物品中...</div>}>
                  <ItemsViewLazy
                    handItems={syncData.hand_items}
                    stashItems={syncData.stash_items}
                    pinnedItems={pinnedItems}
                    expandedItems={expandedItems}
                    onTogglePin={togglePin}
                    onToggleExpand={toggleExpand}
                    renderItemCard={renderUnifiedItemCard}
                    getSortedItems={getSortedItems}
                  />
                </Suspense>
              ) : searchEnabled ? (
                <div className="card-list">
                  {searchResults.length > 0 ? (
                    <Suspense fallback={<div className="empty-tip">加载搜索结果中...</div>}>
                      <VirtualSearchResultsLazy
                        items={searchResults}
                        scrollTop={searchScrollTop}
                        viewportHeight={searchViewportHeight}
                        isItemExpanded={(item) => expandedItems.has(item.uuid)}
                        renderItem={(item) => {
                          const detail = searchDetailMap[item.uuid];
                          const isLoadingDetail = searchDetailLoading.has(item.uuid);
                          const isMissingDetail = searchDetailMissing.has(item.uuid);
                          const renderItemData = detail
                            ? { ...detail, displayImg: item.displayImg || detail.displayImg }
                            : {
                                ...buildSearchSkeletonItem(item),
                                description: isLoadingDetail
                                  ? '正在加载详情...'
                                  : (isMissingDetail ? '暂无该条目的详细数据' : ''),
                              };
                          return renderUnifiedItemCard(
                            renderItemData,
                            pinnedItems.has(item.uuid),
                            (e) => togglePin(item.uuid, e),
                          );
                        }}
                      />
                    </Suspense>
                  ) : null}
                  {searchResults.length === 0 && (
                    <div className="empty-tip">{isSearching ? '搜索中...' : '未找到结果'}</div>
                  )}
                </div>
              ) : (
                <div
                  style={{
                    margin: '16px',
                    padding: '20px',
                    borderRadius: '10px',
                    border: '1px solid rgba(255,205,25,0.25)',
                    background: 'rgba(0,0,0,0.25)',
                  }}
                >
                  <div style={{ fontSize: '18px', color: '#ffcd19', fontWeight: 700, marginBottom: '10px' }}>
                    {panelTitle}
                  </div>
                  <div style={{ color: 'rgba(255,255,255,0.8)', lineHeight: 1.6 }}>
                    {showSettings
                      ? '设置页暂未迁移，当前保留空壳。'
                      : activeTab === 'history'
                        ? '历史战绩加载条件未满足。'
                      : activeTab === 'monster'
                        ? `野怪页加载条件未满足（currentDay=${currentDay ?? '-'}）`
                      : activeTab === 'card'
                        ? '卡牌识别页加载条件未满足。'
                      : activeTab === 'items'
                        ? '手头物品页加载条件未满足。'
                      : activeTab === 'search'
                        ? '百科搜索页加载条件未满足。'
                        : '该页面当前不加载业务数据，等待逐个迁移。'}
                  </div>
                  <div style={{ marginTop: '14px' }}>
                    <button
                      className="bulk-btn"
                      style={{ padding: '6px 12px' }}
                      onClick={() => {
                        void applyCollapsedState(true);
                      }}
                    >
                      收起为灵动岛
                    </button>
                  </div>
                </div>
              )}
              {cardEnabled && errorMessage ? (
                <div
                  style={{
                    margin: '0 16px 16px',
                    padding: '10px 12px',
                    borderRadius: '8px',
                    border: '1px solid rgba(255, 115, 115, 0.45)',
                    background: 'rgba(120, 20, 20, 0.25)',
                    color: '#ffd8d8',
                    fontSize: '13px',
                  }}
                >
                  {errorMessage}
                </div>
              ) : null}
            </div>
          </div>
        </MainShell>
      )}
      {!isCollapsed ? <ToastLayer toasts={toasts} onRemove={removeToast} /> : null}
    </div>
  );
}
