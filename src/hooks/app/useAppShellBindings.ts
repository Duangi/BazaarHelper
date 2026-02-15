import type { AppShellProps } from '../../components/layout/AppShell';
import type { IslandStatusType } from '../../types';

export const useAppShellBindings = (options: {
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
  setErrorMessage: (value: string | null) => void;
  showSettings: boolean;
  setShowSettings: (value: boolean) => void;
  handleToggleCollapse: () => Promise<void>;
  mainContentProps: AppShellProps['mainContentProps'];
  isInstalling: boolean;
  toasts: AppShellProps['toasts'];
  onRemoveToast: AppShellProps['onRemoveToast'];
}): AppShellProps => {
  return {
    showVersionScreen: options.showVersionScreen,
    isCollapsed: options.isCollapsed,
    islandStatusText: options.islandStatusText,
    islandStatusType: options.islandStatusType,
    versionGateProps: {
      fontSize: options.fontSize,
      announcement: options.announcement,
      currentVersion: options.currentVersion,
      updateStatus: options.updateStatus,
      updateAvailableVersion: options.updateAvailableVersion,
      downloadProgress: options.downloadProgress,
      onStartUpdateDownload: options.startUpdateDownload,
      onEnterApp: options.enterApp,
      onInstallReady: options.handleInstallReady,
    },
    overlayClassName: `overlay ${options.isCollapsed ? 'collapsed' : 'expanded'}`,
    overlayStyle: {
      '--user-font-size': `${options.fontSize}px`,
      '--font-scale': options.fontSize / 16,
    } as any,
    onOverlayMouseLeave: options.handleOverlayMouseLeave,
    onTopStripMouseEnter: options.handleTopStripMouseEnter,
    onOverlayMouseDown: options.handleOverlayMouseDown,
    errorMessage: options.errorMessage,
    onCloseError: () => options.setErrorMessage(null),
    showTopBar: false,
    topBarProps: {
      isCollapsed: options.isCollapsed,
      showSettings: options.showSettings,
      updateStatus: options.updateStatus,
      downloadProgress: options.downloadProgress,
      onToggleSettings: () => options.setShowSettings(!options.showSettings),
      onToggleCollapse: () => void options.handleToggleCollapse(),
    },
    mainContentProps: options.mainContentProps,
    isInstalling: options.isInstalling,
    toasts: options.toasts,
    onRemoveToast: options.onRemoveToast,
  };
};
