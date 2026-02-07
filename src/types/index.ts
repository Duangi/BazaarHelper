// 类型定义
export interface ItemData {
  uuid: string;
  instance_id?: string;
  name: string;
  name_cn: string;
  tier: string;
  available_tiers: string;
  size?: string;
  tags: string;
  hidden_tags: string;
  processed_tags: string[];
  heroes: string[];
  cooldown?: number;
  cooldown_tiers: string;
  damage_tiers: string;
  heal_tiers: string;
  shield_tiers: string;
  ammo_tiers: string;
  crit_tiers: string;
  multicast_tiers: string;
  burn_tiers: string;
  poison_tiers: string;
  regen_tiers: string;
  lifesteal_tiers: string;
  skills: string[];
  enchantments: string[];
  description: string;
  image: string;
  displayImg?: string;
  displayHeroImg?: string;
  // Card Recognition specific fields
  matchLabel?: string; // "✓ Match" or "? Maybe"
  matchConfidence?: number;
  matchCount?: number;
}

export interface SyncPayload {
  hand_items: ItemData[];
  stash_items: ItemData[];
  all_tags: string[];
}

export interface TierInfo {
  description: string[];
  extra_description: string[];
  cd: string | null;
}

export interface MonsterSubItem {
  id?: string;
  name: string;
  name_cn?: string;
  name_en?: string;
  tier?: string;
  current_tier?: string;
  starting_tier?: string;
  tags?: string[];
  tiers: Record<string, TierInfo | null>;
  image: string;
  displayImg?: string;
  size?: string;
  skills?: Array<{en: string, cn: string} | string>;  // 支持 SkillText 或纯字符串
}

export interface MonsterData {
  name: string;
  name_zh: string;
  available?: string;
  health?: any;
  skills?: MonsterSubItem[];
  items?: MonsterSubItem[];
  image?: string;
  displayImg?: string;
  displayImgBg?: string;
}

export type TabType = "items" | "search" | "monster" | "card";

export interface Toast {
  id: number;
  message: string;
  type: 'success' | 'error' | 'warning' | 'info';
}

export interface SearchQuery {
  keyword: string;
  item_type: string;
  size: string;
  start_tier: string;
  hero: string;
  tags: string;
  hidden_tags: string;
}
