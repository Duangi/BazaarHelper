import { useState } from 'react';

import type { MonsterData, TabType, SyncPayload } from '../../types';

export const useAppCoreState = () => {
  const [activeTab, setActiveTab] = useState<TabType>('history');
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
