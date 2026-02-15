import { getCurrentWindow } from '@tauri-apps/api/window';
import type { MouseEvent } from 'react';

interface VersionGateScreenProps {
  fontSize: number;
  announcement: string;
  currentVersion: string;
  updateStatus: string;
  updateAvailableVersion?: string;
  downloadProgress: number;
  onStartUpdateDownload: () => void;
  onEnterApp: () => void;
  onInstallReady: () => void;
}

export function VersionGateScreen({
  fontSize,
  announcement,
  currentVersion,
  updateStatus,
  updateAvailableVersion,
  downloadProgress,
  onStartUpdateDownload,
  onEnterApp,
  onInstallReady,
}: VersionGateScreenProps) {
  const appWindow = getCurrentWindow();
  const MANUAL_MINIMIZE_KEY = 'bh-window-manual-minimize';
  const NON_DRAG_SELECTOR = 'button, a, input, textarea, select, [data-no-drag], .no-drag';

  const startDragging = (event?: MouseEvent<HTMLElement>) => {
    if (event?.button !== undefined && event.button !== 0) return;
    const target = event?.target as HTMLElement | null;
    if (target?.closest(NON_DRAG_SELECTOR)) return;
    void appWindow.startDragging().catch((err) => {
      console.error('[VersionGate] startDragging failed:', err);
    });
  };

  return (
    <div
      className="update-screen"
      onMouseDownCapture={startDragging}
      style={{
        '--user-font-size': `${fontSize}px`,
        '--font-scale': fontSize / 16,
      } as any}
    >
      <div className="update-content">
        <div className="update-topbar">
          <div className="update-drag-region" data-tauri-drag-region onMouseDown={startDragging} />
          <button
            className="update-minimize-btn"
            title="最小化"
            onClick={async () => {
              localStorage.setItem(MANUAL_MINIMIZE_KEY, '1');
              try {
                await appWindow.hide();
              } catch (err) {
                console.warn('[VersionGate] hide failed:', err);
              }
            }}
          >
            -
          </button>
        </div>

        <h1 className="bulletin-title" data-tauri-drag-region onMouseDown={startDragging}>集市小抄</h1>

        <div className="bulletin-body">
          {announcement ? (
            <div className="bulletin-text">
              {announcement.split('\n').map((line, i) => (
                <p key={i}>{line}</p>
              ))}
            </div>
          ) : (
            <div className="bulletin-loading">正在获取最新公告...</div>
          )}
        </div>

        <div className="version-info-row">
          <span className="current-v">当前版本: v{currentVersion}</span>
          <div className="update-status-tag">
            {updateStatus === 'checking' && <span className="status-checking">检查更新中...</span>}
            {updateStatus === 'available' && (
              <span className="status-available pulsate">新版本 v{updateAvailableVersion} 可用</span>
            )}
            {updateStatus === 'none' && <span className="status-none">已是最新版</span>}
          </div>
        </div>

        <div className="bulletin-actions">
          {updateStatus === 'available' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', width: '100%' }}>
              <button className="update-now-btn" onClick={onStartUpdateDownload}>
                立即更新
              </button>
              <button
                className="enter-btn"
                style={{ background: 'transparent', border: '1px solid rgba(255,255,255,0.2)', opacity: 0.8 }}
                onClick={onEnterApp}
              >
                直接进入插件
              </button>
            </div>
          )}

          {updateStatus === 'downloading' && (
            <div style={{ width: '100%', textAlign: 'center' }}>
              <div style={{ fontSize: '14px', color: '#58a6ff', marginBottom: '8px' }}>
                正在下载更新... {downloadProgress}%
              </div>
              <div
                style={{
                  width: '100%',
                  height: '6px',
                  background: 'rgba(255, 255, 255, 0.1)',
                  borderRadius: '3px',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    width: `${downloadProgress}%`,
                    height: '100%',
                    background: 'linear-gradient(to right, var(--c-golden), #fff)',
                    transition: 'width 0.3s ease',
                  }}
                ></div>
              </div>
            </div>
          )}

          {updateStatus === 'ready' && (
            <button className="update-now-btn" onClick={onInstallReady}>
              更新已就绪，点击安装
            </button>
          )}

          {(updateStatus === 'none' || updateStatus === 'checking') && (
            <button className="enter-btn" onClick={onEnterApp}>
              进入插件
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
