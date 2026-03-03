import { useState } from 'react';

import type { MonsterData, TabType, SyncPayload } from '../../types';

const VALID_TABS: TabType[] = ['history', 'items', 'search', 'monster', 'card'];

const getInitialTab = (): TabType => {
  try {
    const raw = localStorage.getItem('bh-active-tab') as TabType | null;
    if (raw && VALID_TABS.includes(raw)) {
      return raw;
    }
  } catch {
    // ignore localStorage failures
  }
  return 'history';
};

export const useAppCoreState = () => {
  const [activeTab, setActiveTab] = useState<TabType>(getInitialTab);
  const [syncData, setSyncData] = useState<SyncPayload & { monster: any[] }>({
    hand_items: [],
    stash_items: [],
    all_tags: [],
    monster: [],
  });
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [allMonsters, setAllMonsters] = useState<Record<string, MonsterData>>({});
  const [selectedDay, setSelectedDay] = useState<string>('');
  const [templateLoading, setTemplateLoading] = useState({
    loaded: 0,
    total: 0,
    is_complete: false,
    current_name: '',
  });
  const [currentDay, setCurrentDay] = useState<number | null>(null);

  return {
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
  };
};
