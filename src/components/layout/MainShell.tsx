import React from 'react';
import type { TabType } from '../../types';
import { SidebarNav } from '../sidebar/SidebarNav';

import './MainShell.css';

interface MainShellProps {
  activeTab: TabType;
  onTabChange: (tab: TabType) => void;
  onOpenProfile?: () => void;
  onOpenSettings: () => void;
  children: React.ReactNode;
}

export const MainShell: React.FC<MainShellProps> = ({
  activeTab,
  onTabChange,
  onOpenProfile,
  onOpenSettings,
  children,
}) => {
  return (
    <div className="main-shell">
      <SidebarNav
        activeTab={activeTab}
        onTabChange={onTabChange}
        onOpenProfile={onOpenProfile}
        onOpenSettings={onOpenSettings}
      />
      <div className="main-panel">{children}</div>
    </div>
  );
};
