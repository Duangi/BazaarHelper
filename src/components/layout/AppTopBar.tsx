import { getCurrentWindow } from '@tauri-apps/api/window';

interface AppTopBarProps {
  isCollapsed: boolean;
  showSettings: boolean;
  updateStatus: string;
  downloadProgress: number;
  onToggleSettings: () => void;
  onToggleCollapse: () => void;
}

export function AppTopBar({
  isCollapsed,
  showSettings,
  updateStatus,
  downloadProgress,
  onToggleSettings,
  onToggleCollapse,
}: AppTopBarProps) {
  const appWindow = getCurrentWindow();

  return (
    <>
      <div className="top-bar">
        <div className="drag-handle" data-tauri-drag-region>
          <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <circle cx="9" cy="7" r="1.5" fill="currentColor" />
            <circle cx="15" cy="7" r="1.5" fill="currentColor" />
            <circle cx="9" cy="12" r="1.5" fill="currentColor" />
            <circle cx="15" cy="12" r="1.5" fill="currentColor" />
            <circle cx="9" cy="17" r="1.5" fill="currentColor" />
            <circle cx="15" cy="17" r="1.5" fill="currentColor" />
          </svg>
        </div>

        <button
          className="settings-btn"
          onClick={onToggleSettings}
          title="设置"
          style={{ position: 'relative' }}
        >
          <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 15C13.6569 15 15 13.6569 15 12C15 10.3431 13.6569 9 12 9C10.3431 9 9 10.3431 9 12C9 13.6569 10.3431 15 12 15Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33 1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82 1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          {(updateStatus === 'available' || updateStatus === 'ready') && (
            <div
              style={{
                position: 'absolute',
                top: '4px',
                right: '4px',
                width: '8px',
                height: '8px',
                backgroundColor: '#ff4d4f',
                borderRadius: '50%',
                border: '1px solid #28231e',
              }}
            />
          )}
          {updateStatus === 'downloading' && (
            <div
              style={{
                position: 'absolute',
                top: '4px',
                right: '4px',
                width: '8px',
                height: '8px',
                backgroundColor: '#58a6ff',
                borderRadius: '50%',
                border: '1px solid #28231e',
                animation: 'pulse 1s infinite',
              }}
            />
          )}
        </button>

        <div className="collapse-btn" onClick={onToggleCollapse}>
          {isCollapsed ? '展开' : '收起'}
          <span className={`collapse-arrow ${isCollapsed ? 'collapsed' : 'expanded'}`}>▾</span>
        </div>

        <button className="close-btn" onClick={() => appWindow.hide()} title="隐藏窗口">
          <svg className="close-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M18 6L6 18M6 6L18 18" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>

        <div
          className="resize-handle-right"
          onMouseDown={() => {
            try {
              (appWindow as any).startResizing('Right');
            } catch (e) {
              console.error('startResizing Right failed', e);
            }
          }}
        />
        <div
          className="resize-handle-br"
          onMouseDown={() => {
            try {
              (appWindow as any).startResizing('BottomRight');
            } catch (e) {
              console.error('startResizing BottomRight failed', e);
            }
          }}
        />
      </div>

      {updateStatus === 'downloading' && !showSettings && (
        <div
          style={{
            width: '100%',
            height: '2px',
            backgroundColor: 'rgba(255,255,255,0.1)',
            position: 'relative',
            overflow: 'hidden',
            zIndex: 100,
          }}
        >
          <div
            style={{
              position: 'absolute',
              left: 0,
              top: 0,
              height: '100%',
              width: `${downloadProgress}%`,
              backgroundColor: '#58a6ff',
              boxShadow: '0 0 4px #58a6ff',
              transition: 'width 0.3s ease',
            }}
          />
        </div>
      )}
    </>
  );
}
