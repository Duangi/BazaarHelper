import React from 'react';
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { exit } from '@tauri-apps/plugin-process';

interface TopBarProps {
  isCollapsed: boolean;
  expandedWidthRef: React.MutableRefObject<number>;
  expandedHeightRef: React.MutableRefObject<number>;
  isProgrammaticResize: React.MutableRefObject<boolean>;
  onToggleCollapse: () => void;
  onToggleSettings: () => void;
}

export const TopBar: React.FC<TopBarProps> = ({
  isCollapsed,
  expandedWidthRef,
  expandedHeightRef,
  isProgrammaticResize,
  onToggleCollapse,
  onToggleSettings
}) => {
  const appWindow = getCurrentWindow();

  const handleCollapse = async () => {
    if (expandedHeightRef.current < 200) {
      expandedHeightRef.current = 700;
    }
    
    const newCollapsed = !isCollapsed;
    
    try {
      const targetW = expandedWidthRef.current;
      const targetH = newCollapsed ? 45 : expandedHeightRef.current;
      
      isProgrammaticResize.current = true;
      await appWindow.setSize(new LogicalSize(targetW, targetH));
      setTimeout(() => { isProgrammaticResize.current = false; }, 300);
      
      onToggleCollapse();
    } catch (e) {
      console.error('Failed to resize on collapse/expand:', e);
      onToggleCollapse();
    }
  };

  return (
    <div className="top-bar">
      <div className="drag-handle" data-tauri-drag-region>
        <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <circle cx="9" cy="7" r="1.5" fill="currentColor"/>
          <circle cx="15" cy="7" r="1.5" fill="currentColor"/>
          <circle cx="9" cy="12" r="1.5" fill="currentColor"/>
          <circle cx="15" cy="12" r="1.5" fill="currentColor"/>
          <circle cx="9" cy="17" r="1.5" fill="currentColor"/>
          <circle cx="15" cy="17" r="1.5" fill="currentColor"/>
        </svg>
      </div>

      <button className="settings-btn" onClick={onToggleSettings} title="设置">
        <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M12 15C13.6569 15 15 13.6569 15 12C15 10.3431 13.6569 9 12 9C10.3431 9 9 10.3431 9 12C9 13.6569 10.3431 15 12 15Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33 1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82 1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
        </svg>
      </button>
      
      <div className="collapse-btn" onClick={handleCollapse}>
        {isCollapsed ? "展开" : "收起"}
        <span className={`collapse-arrow ${isCollapsed ? 'collapsed' : 'expanded'}`}>▾</span>
      </div>
      
      <button className="close-btn" onClick={() => exit(0)} title="关闭">
        <svg className="close-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M18 6L6 18M6 6L18 18" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
        </svg>
      </button>

      <div
        className="resize-handle-right"
        onMouseDown={() => {
          try { (appWindow as any).startResizing('Right'); } catch (e) { console.error('startResizing Right failed', e); }
        }}
      />
      <div
        className="resize-handle-br"
        onMouseDown={() => {
          try { (appWindow as any).startResizing('BottomRight'); } catch (e) { console.error('startResizing BottomRight failed', e); }
        }}
      />
    </div>
  );
};
