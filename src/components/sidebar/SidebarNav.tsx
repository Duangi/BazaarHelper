import React from 'react';
import type { TabType } from '../../types';

import iconSetting from '../../assets/icons/sidebar/setting.svg';
import { SIDEBAR_NAV_ITEMS } from './sidebarConfig';

interface SidebarNavProps {
  activeTab: TabType;
  onTabChange: (tab: TabType) => void;
  onOpenSettings: () => void;
}

export const SidebarNav: React.FC<SidebarNavProps> = ({
  activeTab,
  onTabChange,
  onOpenSettings,
}) => {
  return (
    <aside className="sidebar-nav">
      <div className="sidebar-nav-surface">
        <div className="sidebar-brand" title="集市小抄" data-tauri-drag-region>
          <span className="sidebar-brand-short">集</span>
          <span className="sidebar-brand-full">集市小抄</span>
        </div>

        <div className="sidebar-nav-list">
          {SIDEBAR_NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              className={`sidebar-nav-btn ${activeTab === item.id ? 'active' : ''}`}
              onClick={() => onTabChange(item.id)}
              title={item.label}
            >
              <img src={item.icon} alt="" className="sidebar-nav-icon" />
              <span className="sidebar-nav-label">{item.label}</span>
            </button>
          ))}
        </div>

        <button className="sidebar-settings-btn" onClick={onOpenSettings} title="系统设置">
          <img src={iconSetting} alt="" className="sidebar-nav-icon" />
          <span className="sidebar-nav-label">系统设置</span>
        </button>
      </div>
    </aside>
  );
};
