import type { ComponentProps, CSSProperties, MouseEvent } from 'react';

import { MainContentSection } from './MainContentSection';
import { AppTopBar } from './AppTopBar';
import { ErrorToast } from './ErrorToast';
import { InstallingOverlay } from './InstallingOverlay';
import { ToastLayer } from './ToastLayer';
import { VersionGateScreen } from '../settings/VersionGateScreen';

import type { Toast } from '../../types';
import type { IslandStatusType } from '../../types';

export interface AppShellProps {
  showVersionScreen: boolean;
  isCollapsed: boolean;
  islandStatusText: string;
  islandStatusType: IslandStatusType;
  versionGateProps: ComponentProps<typeof VersionGateScreen>;
  overlayClassName: string;
  overlayStyle: CSSProperties;
  onOverlayMouseLeave: (e: MouseEvent<HTMLDivElement>) => void;
  onTopStripMouseEnter: () => void;
  onOverlayMouseDown: (e: MouseEvent<HTMLDivElement>) => void;
  errorMessage: string | null;
  onCloseError: () => void;
  showTopBar?: boolean;
  topBarProps?: ComponentProps<typeof AppTopBar>;
  mainContentProps: ComponentProps<typeof MainContentSection>;
  isInstalling: boolean;
  toasts: Toast[];
  onRemoveToast: (id: number) => void;
}

export const AppShell = ({
  showVersionScreen,
  isCollapsed,
  islandStatusText,
  islandStatusType,
  versionGateProps,
  overlayClassName,
  overlayStyle,
  onOverlayMouseLeave,
  onTopStripMouseEnter,
  onOverlayMouseDown,
  errorMessage,
  onCloseError,
  showTopBar = false,
  topBarProps,
  mainContentProps,
  isInstalling,
  toasts,
  onRemoveToast,
}: AppShellProps) => {
  if (showVersionScreen) {
    return <VersionGateScreen {...versionGateProps} />;
  }

  return (
    <div
      className={overlayClassName}
      style={overlayStyle}
      onMouseLeave={onOverlayMouseLeave}
      onMouseDownCapture={onOverlayMouseDown}
    >
      <div className="window-drag-strip" data-tauri-drag-region onMouseEnter={onTopStripMouseEnter} />
      {isCollapsed ? (
        <div className="dynamic-island-chip" data-tauri-drag-region>
          <span className={`dynamic-island-dot ${islandStatusType}`} />
          <span className="dynamic-island-text">{islandStatusText}</span>
        </div>
      ) : null}
      <ErrorToast errorMessage={errorMessage} onClose={onCloseError} />
      {showTopBar && topBarProps ? <AppTopBar {...topBarProps} /> : null}
      <MainContentSection {...mainContentProps} />
      <InstallingOverlay visible={isInstalling} />
      {!isCollapsed ? <ToastLayer toasts={toasts} onRemove={onRemoveToast} /> : null}
    </div>
  );
};
