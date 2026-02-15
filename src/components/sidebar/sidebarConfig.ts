import type { TabType } from '../../types';

import iconHistory from '../../assets/icons/sidebar/history.svg';
import iconBattle from '../../assets/icons/sidebar/battle.svg';
import iconScanner from '../../assets/icons/sidebar/scanner.svg';
import iconChest from '../../assets/icons/sidebar/chest.svg';
import iconSearch from '../../assets/icons/sidebar/search.svg';

export interface SidebarNavItem {
  id: TabType;
  label: string;
  icon: string;
}

export const SIDEBAR_NAV_ITEMS: SidebarNavItem[] = [
  { id: 'history', label: '历史战绩', icon: iconHistory },
  { id: 'monster', label: '野怪一览', icon: iconBattle },
  { id: 'card', label: '卡牌识别', icon: iconScanner },
  { id: 'items', label: '手头物品', icon: iconChest },
  { id: 'search', label: '百科搜索', icon: iconSearch },
];
