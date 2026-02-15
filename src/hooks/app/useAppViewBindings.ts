import type { AppShellProps } from '../../components/layout/AppShell';
import type { IslandStatusType } from '../../types';
import { useAppShellBindings } from './useAppShellBindings';

export const useAppViewBindings = (options: {
  showVersionScreen: boolean;
  fontSize: number;
  announcement: string;
  currentVersion: string;
  updateStatus: string;
  updateAvailableVersion?: string;
  downloadProgress: number;
  startUpdateDownload: () => void;
  enterApp: () => void;
  handleInstallReady: () => void;
  isCollapsed: boolean;
  islandStatusText: string;
  islandStatusType: IslandStatusType;
  handleOverlayMouseLeave: AppShellProps['onOverlayMouseLeave'];
  handleTopStripMouseEnter: AppShellProps['onTopStripMouseEnter'];
  handleOverlayMouseDown: AppShellProps['onOverlayMouseDown'];
  errorMessage: string | null;
  setErrorMessage: (v: string | null) => void;
  showSettings: boolean;
  setShowSettings: (v: boolean) => void;
  handleToggleCollapse: () => Promise<void>;
  mainContentProps: AppShellProps['mainContentProps'];
  isInstalling: boolean;
  toasts: AppShellProps['toasts'];
  onRemoveToast: (id: number) => void;
}) => {
  return useAppShellBindings({
    showVersionScreen: options.showVersionScreen,
    fontSize: options.fontSize,
    announcement: options.announcement,
    currentVersion: options.currentVersion,
    updateStatus: options.updateStatus,
    updateAvailableVersion: options.updateAvailableVersion,
    downloadProgress: options.downloadProgress,
    startUpdateDownload: options.startUpdateDownload,
    enterApp: options.enterApp,
    handleInstallReady: options.handleInstallReady,
    isCollapsed: options.isCollapsed,
    islandStatusText: options.islandStatusText,
    islandStatusType: options.islandStatusType,
    handleOverlayMouseLeave: options.handleOverlayMouseLeave,
    handleTopStripMouseEnter: options.handleTopStripMouseEnter,
    handleOverlayMouseDown: options.handleOverlayMouseDown,
    errorMessage: options.errorMessage,
    setErrorMessage: options.setErrorMessage,
    showSettings: options.showSettings,
    setShowSettings: options.setShowSettings,
    handleToggleCollapse: options.handleToggleCollapse,
    mainContentProps: options.mainContentProps,
    isInstalling: options.isInstalling,
    toasts: options.toasts,
    onRemoveToast: options.onRemoveToast,
  });
};
