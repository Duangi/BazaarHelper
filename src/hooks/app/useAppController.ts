import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

import type { AppShellProps } from '../../components/layout/AppShell';
import { useResourceCatalog } from '../features/useResourceCatalog';
import { useMonsterTabLogic } from '../features/useMonsterTabLogic';
import { useVersionUpdate } from '../features/useVersionUpdate';
import { useCardRecognition } from '../features/useCardRecognition';
import { useItemCardState } from '../features/useItemCardState';
import { useTierInfoRenderer } from '../features/useTierInfoRenderer';
import { useSearchPanelState } from '../search/useSearchPanelState';
import { useUpdateActions } from '../features/useUpdateActions';
import { useMatchHistory } from '../sync/useMatchHistory';
import { useAppSettingsState } from './useAppSettingsState';
import { useOverlayWindowState } from '../window/useOverlayWindowState';
import { useUnifiedItemCardRenderer } from '../features/useUnifiedItemCardRenderer';
import { useOverlayActions } from '../window/useOverlayActions';
import { useSyncDataPipeline } from '../sync/useSyncDataPipeline';
import { useDayTabSelection } from '../features/useDayTabSelection';
import { useAppCoreState } from './useAppCoreState';
import { useAppLifecycleEffects } from './useAppLifecycleEffects';
import { useAppViewBindings } from './useAppViewBindings';
import type { IslandStatusType, SyncPayload } from '../../types';

export const useAppController = (): AppShellProps => {
  const selfTestMode = import.meta.env.VITE_BH_SELF_TEST === '1';
  const selfTestTab = (import.meta.env.VITE_BH_SELF_TEST_TAB || '').trim().toLowerCase();
  const disableHistorySync = import.meta.env.VITE_BH_DISABLE_HISTORY_SYNC === '1';
  const [islandStatusText, setIslandStatusText] = useState('集市小抄 运行中');
  const [islandStatusType, setIslandStatusType] = useState<IslandStatusType>('info');
  const islandStatusTimerRef = useRef<number | null>(null);

  const updateIslandStatus = useCallback((message: string, type: IslandStatusType = 'info') => {
    setIslandStatusText(message);
    setIslandStatusType(type);

    if (islandStatusTimerRef.current) {
      window.clearTimeout(islandStatusTimerRef.current);
    }
    islandStatusTimerRef.current = window.setTimeout(() => {
      setIslandStatusText('集市小抄 运行中');
      setIslandStatusType('info');
      islandStatusTimerRef.current = null;
    }, 6000);
  }, []);

  useEffect(() => {
    return () => {
      if (islandStatusTimerRef.current) {
        window.clearTimeout(islandStatusTimerRef.current);
      }
    };
  }, []);
  const {
    activeTab,
    setActiveTab,
    syncData,
    setSyncData,
    isCollapsed,
    setIsCollapsed,
    allMonsters,
    setAllMonsters,
    selectedDay,
    setSelectedDay,
    templateLoading,
    setTemplateLoading,
    currentDay,
    setCurrentDay,
  } = useAppCoreState();

  const {
    announcement,
    currentVersion,
    downloadProgress,
    enterApp,
    errorMessage,
    isInstalling,
    setAnnouncement,
    setErrorMessage,
    setIsInstalling,
    setUpdateAvailable,
    setUpdateStatus,
    showVersionScreen,
    startUpdateDownload,
    updateAvailable,
    updateStatus,
  } = useVersionUpdate();

  const appRuntimeEnabled = !showVersionScreen;

  useEffect(() => {
    if (!selfTestMode || !appRuntimeEnabled) return;

    const order: Array<'history' | 'items' | 'search' | 'monster' | 'card'> = [
      'history',
      'items',
      'search',
      'monster',
      'card',
    ];
    if (
      selfTestTab === 'history'
      || selfTestTab === 'items'
      || selfTestTab === 'search'
      || selfTestTab === 'monster'
      || selfTestTab === 'card'
    ) {
      setActiveTab(selfTestTab);
      console.log(`[SelfTest] fixed tab => ${selfTestTab}`);
      return;
    }

    let idx = 0;

    setActiveTab(order[idx]);
    const timer = window.setInterval(() => {
      idx = (idx + 1) % order.length;
      setActiveTab(order[idx]);
      console.log(`[SelfTest] switched tab => ${order[idx]}`);
    }, 45_000);

    return () => window.clearInterval(timer);
  }, [appRuntimeEnabled, selfTestMode, selfTestTab, setActiveTab]);

  useEffect(() => {
    try {
      localStorage.setItem('bh-active-tab', activeTab);
    } catch {
      // ignore localStorage failures
    }
  }, [activeTab]);

  const {
    fontSize,
    setFontSize,
    showSettings,
    setShowSettings,
    enableYoloAuto,
    setEnableYoloAuto,
    yoloScanInterval,
    setYoloScanInterval,
    useGpuAcceleration,
    setUseGpuAcceleration,
    showYoloMonitor,
    setShowYoloMonitor,
    yoloHotkey,
    setYoloHotkey,
    toasts,
    setToasts,
    showToast,
    detectionHotkey,
    setDetectionHotkey,
    cardDetectionHotkey,
    setCardDetectionHotkey,
    toggleCollapseHotkey,
    setToggleCollapseHotkey,
    detailDisplayHotkey,
    setDetailDisplayHotkey,
    isRecordingHotkey,
    setIsRecordingHotkey,
    isRecordingCardHotkey,
    setIsRecordingCardHotkey,
    isRecordingToggleHotkey,
    setIsRecordingToggleHotkey,
    isRecordingYoloHotkey,
    setIsRecordingYoloHotkey,
    isRecordingDetailHotkey,
    setIsRecordingDetailHotkey,
    showResetHotkeysConfirm,
    setShowResetHotkeysConfirm,
    settingsExpanded,
    setSettingsExpanded,
  } = useAppSettingsState();

  const { matchHistory, isLoadingHistory, loadMatchHistory } = useMatchHistory({
    enabled: appRuntimeEnabled && activeTab === 'history' && !disableHistorySync,
  });
  const { processItems, processSyncPayload } = useSyncDataPipeline({ setSyncData });
  const deferredSyncPayloadRef = useRef<SyncPayload | null>(null);

  const processSyncPayloadRouted = useCallback(async (payload: SyncPayload) => {
    if (activeTab === 'search' || isCollapsed) {
      // Search tab and collapsed island are DB-only/presentation-light.
      // Defer live sync updates to avoid high-frequency heavy rerenders in idle mode.
      deferredSyncPayloadRef.current = payload;
      return;
    }
    await processSyncPayload(payload);
  }, [activeTab, isCollapsed, processSyncPayload]);

  useEffect(() => {
    if (activeTab === 'search') return;
    const pending = deferredSyncPayloadRef.current;
    if (!pending) return;
    deferredSyncPayloadRef.current = null;
    void processSyncPayload(pending);
  }, [activeTab, processSyncPayload]);

  const showToastSmart = useCallback((message: string, type: 'success' | 'error' | 'warning' | 'info' = 'info') => {
    if (isCollapsed) {
      updateIslandStatus(message, type as IslandStatusType);
      return;
    }
    showToast(message, type);
  }, [isCollapsed, showToast, updateIslandStatus]);

  const logMonitorBaselineEnabledRef = useRef<boolean | null>(null);
  const logMonitorPausedByIslandRef = useRef(false);

  useEffect(() => {
    if (!appRuntimeEnabled) return;

    let disposed = false;

    const syncRuntimeLogMonitor = async () => {
      try {
        if (logMonitorBaselineEnabledRef.current === null) {
          const enabled = await invoke<boolean>('get_game_log_monitor_enabled');
          if (disposed) return;
          logMonitorBaselineEnabledRef.current = enabled;
        }

        const baselineEnabled = logMonitorBaselineEnabledRef.current ?? true;
        if (!baselineEnabled) return;

        if (isCollapsed) {
          if (!logMonitorPausedByIslandRef.current) {
            await invoke<boolean>('set_game_log_monitor_runtime', { enabled: false });
            if (disposed) return;
            logMonitorPausedByIslandRef.current = true;
          }
          return;
        }

        if (logMonitorPausedByIslandRef.current) {
          await invoke<boolean>('set_game_log_monitor_runtime', { enabled: true });
          if (disposed) return;
          logMonitorPausedByIslandRef.current = false;
        }
      } catch (error) {
        if (!disposed) {
          console.warn('[LogMonitor] runtime auto pause/resume failed:', error);
        }
      }
    };

    void syncRuntimeLogMonitor();

    return () => {
      disposed = true;
    };
  }, [appRuntimeEnabled, isCollapsed]);

  useEffect(() => {
    return () => {
      if (logMonitorPausedByIslandRef.current) {
        void invoke<boolean>('set_game_log_monitor_runtime', { enabled: true }).catch(() => {});
      }
    };
  }, []);

  const { hiddenTagIcons, sponsorIcons, skillsArtMap, itemSizes } = useResourceCatalog(appRuntimeEnabled);

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
    isInputFocused,
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

  const updateDayTabSelection = useDayTabSelection(setSelectedDay);

  const {
    manualMonsters,
    identifiedNames,
    expandedMonsters,
    isRecognizing,
    setIdentifiedNames,
    setExpandedMonsters,
    toggleMonsterExpand,
    handleDayChange,
    handleAutoRecognition,
  } = useMonsterTabLogic({
    enabled: appRuntimeEnabled,
    activeTab,
    selectedDay,
    setSelectedDay,
    allMonsters,
    setCurrentDay: (day: number) => setCurrentDay(day),
    skillsArtMap,
    updateDayTabSelection,
  });

  const {
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
  } = useOverlayWindowState();

  const { handleConfirmResetHotkeys, handleManualCheckUpdate, handleInstallReady } = useUpdateActions({
    showToast: showToastSmart,
    currentVersion: currentVersion || '',
    setAnnouncement,
    setIsInstalling,
    setShowResetHotkeysConfirm,
    setDetectionHotkey,
    setCardDetectionHotkey,
    setToggleCollapseHotkey,
    setYoloHotkey,
    setDetailDisplayHotkey,
    setUpdateStatus,
    setUpdateAvailable,
  });

  const { handleToggleCollapse, handleOverlayMouseLeave, handleTopStripMouseEnter, handleOverlayMouseDown } =
    useOverlayActions({
      expandedHeightRef,
      expandedWidthRef,
      isCollapsed,
      setIsCollapsed,
      isProgrammaticResize,
      isInputFocused,
    });

  const { pinnedItems, expandedItems, setExpandedItems, togglePin, toggleExpand, getSortedItems } =
    useItemCardState();

  const { recognizedCards, isRecognizingCard, handleRecognizeCard } = useCardRecognition({
    setActiveTab,
    setExpandedItems,
    setErrorMessage,
    showToast: showToastSmart,
  });

  const { renderTierInfo } = useTierInfoRenderer(itemSizes);

  const renderUnifiedItemCard = useUnifiedItemCardRenderer({
    expandedItems,
    activeTab,
    recognizedCards,
    allTags: syncData.all_tags || [],
    toggleExpand,
  });

  const onRemoveToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, [setToasts]);

  useAppLifecycleEffects({
    templateLoadingComplete: templateLoading.is_complete,
    processItems,
    setCurrentDay: (day: number) => setCurrentDay(day),
    setSelectedDay,
    setSyncData,
    currentDay,
    onSwitchToMonsterTab: () => setActiveTab('monster'),
    onAutoRecognition: handleAutoRecognition,
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
    processSyncPayload: processSyncPayloadRouted,
    handleRecognizeCard,
    setIsCollapsed,
    setIdentifiedNames,
    setExpandedMonsters,
    setActiveTab: (tab: any) => setActiveTab(tab),
    setDetectionHotkey,
    setCardDetectionHotkey,
    setToggleCollapseHotkey,
    setDetailDisplayHotkey,
    updateDayTabSelection,
    currentScale,
    isLoadingGeometry,
    setExpandedWidth,
    setExpandedHeight,
    setIsGeometryLoaded,
    setTemplateLoading,
    enableYoloAuto,
    yoloScanInterval,
    yoloHotkey,
    setErrorMessage,
    setAllMonsters,
    hasCustomPosition,
    isGeometryLoaded,
    detailDisplayHotkey,
    setYoloHotkey,
    showToast: showToastSmart,
    updateIslandStatus,
  });

  const handleCloseSettings = useCallback(() => setShowSettings(false), [setShowSettings]);

  const settingsPanelProps = useMemo(() => ({
    visible: showSettings,
    onClose: handleCloseSettings,
    settingsExpanded,
    setSettingsExpanded,
    fontSize,
    setFontSize,
    setExpandedWidth,
    setExpandedHeight,
    setHasCustomPosition,
    showToast: showToastSmart,
    enableYoloAuto,
    setEnableYoloAuto,
    useGpuAcceleration,
    setUseGpuAcceleration,
    yoloScanInterval,
    setYoloScanInterval,
    showYoloMonitor,
    setShowYoloMonitor,
    yoloHotkey,
    setYoloHotkey,
    isRecordingYoloHotkey,
    setIsRecordingYoloHotkey,
    detailDisplayHotkey,
    setDetailDisplayHotkey,
    isRecordingDetailHotkey,
    setIsRecordingDetailHotkey,
    detectionHotkey,
    setDetectionHotkey,
    isRecordingHotkey,
    setIsRecordingHotkey,
    cardDetectionHotkey,
    setCardDetectionHotkey,
    isRecordingCardHotkey,
    setIsRecordingCardHotkey,
    toggleCollapseHotkey,
    setToggleCollapseHotkey,
    isRecordingToggleHotkey,
    setIsRecordingToggleHotkey,
    showResetHotkeysConfirm,
    setShowResetHotkeysConfirm,
    onConfirmResetHotkeys: handleConfirmResetHotkeys,
    currentVersion: currentVersion || '...',
    updateStatus,
    downloadProgress,
    updateAvailableVersion: updateAvailable?.version,
    onManualCheckUpdate: handleManualCheckUpdate,
    onStartUpdateDownload: startUpdateDownload,
    onInstallReady: handleInstallReady,
    announcement,
    sponsorIcons,
  }), [
    announcement,
    cardDetectionHotkey,
    currentVersion,
    detailDisplayHotkey,
    detectionHotkey,
    downloadProgress,
    enableYoloAuto,
    fontSize,
    handleCloseSettings,
    handleConfirmResetHotkeys,
    handleInstallReady,
    handleManualCheckUpdate,
    isRecordingCardHotkey,
    isRecordingDetailHotkey,
    isRecordingHotkey,
    isRecordingToggleHotkey,
    isRecordingYoloHotkey,
    setCardDetectionHotkey,
    setDetectionHotkey,
    setDetailDisplayHotkey,
    setEnableYoloAuto,
    setExpandedHeight,
    setExpandedWidth,
    setFontSize,
    setHasCustomPosition,
    setIsRecordingCardHotkey,
    setIsRecordingDetailHotkey,
    setIsRecordingHotkey,
    setIsRecordingToggleHotkey,
    setIsRecordingYoloHotkey,
    setShowResetHotkeysConfirm,
    setSettingsExpanded,
    setShowYoloMonitor,
    setToggleCollapseHotkey,
    setUseGpuAcceleration,
    setYoloHotkey,
    setYoloScanInterval,
    settingsExpanded,
    showResetHotkeysConfirm,
    showSettings,
    showToastSmart,
    showYoloMonitor,
    sponsorIcons,
    startUpdateDownload,
    toggleCollapseHotkey,
    updateAvailable?.version,
    updateStatus,
    useGpuAcceleration,
    yoloHotkey,
    yoloScanInterval,
  ]);

  const mainContentProps = useMemo(() => ({
    isCollapsed,
    activeTab,
    setActiveTab,
    showSettings,
    setShowSettings,
    settingsPanelProps,
    isSearchFilterCollapsed,
    setIsSearchFilterCollapsed,
    matchMode,
    setMatchMode,
    searchFilterHeight,
    searchQuery,
    setSearchQuery,
    setIsInputFocused,
    lastItemSize,
    setLastItemSize,
    selectedTags,
    setSelectedTags,
    selectedHiddenTags,
    setSelectedHiddenTags,
    hiddenTagIcons,
    isSearching,
    searchResults,
    setResizeStartY,
    setResizeStartHeight,
    setIsResizingFilter,
    isResizingFilter,
    scrollAreaRef,
    handleScroll,
    searchScrollTop,
    searchViewportHeight,
    wrapRef,
    matchHistory,
    isLoadingHistory,
    loadMatchHistory,
    selectedDay,
    setSelectedDay,
    handleDayChange,
    isRecognizing,
    handleAutoRecognition,
    showToast: showToastSmart,
    templateLoading,
    manualMonsters,
    identifiedNames,
    expandedMonsters,
    toggleMonsterExpand,
    renderTierInfo,
    recognizedCards,
    isRecognizingCard,
    expandedItems,
    toggleExpand,
    handleRecognizeCard,
    renderUnifiedItemCard,
    handItems: syncData.hand_items,
    stashItems: syncData.stash_items,
    pinnedItems,
    togglePin,
    getSortedItems,
  }), [
    activeTab,
    expandedItems,
    expandedMonsters,
    getSortedItems,
    handleAutoRecognition,
    handleDayChange,
    handleRecognizeCard,
    handleScroll,
    hiddenTagIcons,
    identifiedNames,
    isCollapsed,
    isLoadingHistory,
    isRecognizing,
    isRecognizingCard,
    isResizingFilter,
    isSearchFilterCollapsed,
    isSearching,
    lastItemSize,
    loadMatchHistory,
    manualMonsters,
    matchHistory,
    matchMode,
    pinnedItems,
    recognizedCards,
    renderTierInfo,
    renderUnifiedItemCard,
    scrollAreaRef,
    searchFilterHeight,
    searchQuery,
    searchResults,
    searchScrollTop,
    searchViewportHeight,
    selectedDay,
    selectedHiddenTags,
    selectedTags,
    setActiveTab,
    setIsInputFocused,
    setIsResizingFilter,
    setIsSearchFilterCollapsed,
    setLastItemSize,
    setMatchMode,
    setResizeStartHeight,
    setResizeStartY,
    setSearchQuery,
    setSelectedDay,
    setSelectedHiddenTags,
    setSelectedTags,
    setShowSettings,
    settingsPanelProps,
    showSettings,
    showToastSmart,
    syncData.hand_items,
    syncData.stash_items,
    templateLoading,
    toggleExpand,
    toggleMonsterExpand,
    togglePin,
    wrapRef,
  ]);

  return useAppViewBindings({
    showVersionScreen,
    fontSize,
    announcement,
    currentVersion: currentVersion || '...',
    updateStatus,
    updateAvailableVersion: updateAvailable?.version,
    downloadProgress,
    startUpdateDownload,
    enterApp,
    handleInstallReady,
    isCollapsed,
    islandStatusText,
    islandStatusType,
    handleOverlayMouseLeave,
    handleTopStripMouseEnter,
    handleOverlayMouseDown,
    errorMessage,
    setErrorMessage,
    showSettings,
    setShowSettings,
    handleToggleCollapse,
    mainContentProps,
    isInstalling,
    toasts,
    onRemoveToast,
  });
};
