import { useBackendEventListeners } from '../events/useBackendEventListeners';
import { useCurrentDayBootstrap } from '../sync/useCurrentDayBootstrap';
import { useGlobalOverlayEvents } from '../events/useGlobalOverlayEvents';
import { useInitialSync } from '../sync/useInitialSync';
import { useMonsterHotkeyListener } from '../events/useMonsterHotkeyListener';
import { useMonstersBootstrap } from '../sync/useMonstersBootstrap';
import { useTemplateLoadingProgress } from '../sync/useTemplateLoadingProgress';
import { useWindowGeometryRestore } from '../window/useWindowGeometryRestore';
import { useWindowLayoutSync } from '../window/useWindowLayoutSync';
import { useWindowMoveResizeListeners } from '../window/useWindowMoveResizeListeners';
import { useYoloScan } from '../features/useYoloScan';

export const useAppLifecycleEffects = (options: any) => {
  const appRuntimeEnabled = !options.showVersionScreen;

  useInitialSync({
    enabled: appRuntimeEnabled,
    templateLoadingComplete: options.templateLoadingComplete,
    processItems: options.processItems,
    setCurrentDay: (day) => options.setCurrentDay(day),
    setSelectedDay: options.setSelectedDay,
    setSyncData: options.setSyncData,
  });

  useMonsterHotkeyListener({
    enabled: appRuntimeEnabled,
    currentDay: options.currentDay,
    onSwitchToMonsterTab: options.onSwitchToMonsterTab,
    onAutoRecognition: options.onAutoRecognition,
  });

  useWindowMoveResizeListeners({
    enabled: appRuntimeEnabled,
    showVersionScreen: options.showVersionScreen,
    isCollapsed: options.isCollapsed,
    setHasCustomPosition: options.setHasCustomPosition,
    lastKnownPosition: options.lastKnownPosition,
    expandedWidthRef: options.expandedWidthRef,
    expandedHeightRef: options.expandedHeightRef,
    isDragging: options.isDragging,
    isResizing: options.isResizing,
    lastUserResize: options.lastUserResize,
    isProgrammaticResize: options.isProgrammaticResize,
  });

  useGlobalOverlayEvents();

  useBackendEventListeners({
    enabled: appRuntimeEnabled,
    isResizing: options.isResizing,
    processSyncPayload: options.processSyncPayload,
    handleAutoRecognition: options.onAutoRecognition,
    handleRecognizeCard: options.handleRecognizeCard,
    setIsCollapsed: options.setIsCollapsed,
    setCurrentDay: options.setCurrentDay,
    setSelectedDay: options.setSelectedDay,
    setIdentifiedNames: options.setIdentifiedNames,
    setExpandedMonsters: options.setExpandedMonsters,
    setActiveTab: (tab) => options.setActiveTab(tab),
    setDetectionHotkey: options.setDetectionHotkey,
    setCardDetectionHotkey: options.setCardDetectionHotkey,
    setToggleCollapseHotkey: options.setToggleCollapseHotkey,
    setDetailDisplayHotkey: options.setDetailDisplayHotkey,
    updateIslandStatus: options.updateIslandStatus,
  });

  useCurrentDayBootstrap({
    enabled: appRuntimeEnabled,
    currentDay: options.currentDay,
    setCurrentDay: (day) => options.setCurrentDay(day),
    updateDayTabSelection: options.updateDayTabSelection,
  });

  useWindowGeometryRestore({
    enabled: appRuntimeEnabled,
    currentScale: options.currentScale,
    lastKnownPosition: options.lastKnownPosition,
    expandedWidthRef: options.expandedWidthRef,
    expandedHeightRef: options.expandedHeightRef,
    isLoadingGeometry: options.isLoadingGeometry,
    setHasCustomPosition: options.setHasCustomPosition,
    setExpandedWidth: options.setExpandedWidth,
    setExpandedHeight: options.setExpandedHeight,
    setIsGeometryLoaded: options.setIsGeometryLoaded,
  });

  useTemplateLoadingProgress(options.setTemplateLoading, appRuntimeEnabled);

  useYoloScan({
    enabled: appRuntimeEnabled,
    enableYoloAuto: options.enableYoloAuto,
    isCollapsed: options.isCollapsed,
    yoloScanInterval: options.yoloScanInterval,
    yoloHotkey: options.yoloHotkey,
    setErrorMessage: options.setErrorMessage,
    updateIslandStatus: options.updateIslandStatus,
    showToast: options.showToast,
  });

  useMonstersBootstrap({ enabled: appRuntimeEnabled, setAllMonsters: options.setAllMonsters });

  useWindowLayoutSync({
    showVersionScreen: options.showVersionScreen,
    isCollapsed: options.isCollapsed,
    hasCustomPosition: options.hasCustomPosition,
    isGeometryLoaded: options.isGeometryLoaded,
    isLoadingGeometry: options.isLoadingGeometry,
    currentScale: options.currentScale,
    lastKnownPosition: options.lastKnownPosition,
    expandedWidthRef: options.expandedWidthRef,
    expandedHeightRef: options.expandedHeightRef,
    isResizing: options.isResizing,
    lastUserResize: options.lastUserResize,
    isProgrammaticResize: options.isProgrammaticResize,
    isDragging: options.isDragging,
    yoloHotkey: options.yoloHotkey,
    detailDisplayHotkey: options.detailDisplayHotkey,
    setYoloHotkey: options.setYoloHotkey,
    setHasCustomPosition: options.setHasCustomPosition,
    setIsCollapsed: options.setIsCollapsed,
    setExpandedWidth: options.setExpandedWidth,
    setExpandedHeight: options.setExpandedHeight,
    showToast: options.showToast,
  });
};
