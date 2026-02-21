import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, PhysicalSize, PhysicalPosition } from "@tauri-apps/api/window"; 
// Note: ResizeDirection will be used as a string if not exported as Enum, 
// checking if it is exported. If TypeScript fails, I'll switch to strings or cast.
// Actually, let's look at how to import it. It should be in window.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import "./App.css";
import { UnifiedItemCard } from "./components/UnifiedItemCard";

// Debounce helper
const debounce = (func: Function, wait: number) => {
  let timeout: any;
  return (...args: any[]) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => func(...args), wait);
  };
};

// 常量定义
const KEYWORD_COLORS: Record<string, string> = {
  "弹药": "#ff8e00",
  "灼烧": "#ff9f45",
  "充能": "#00ecc3",
  "冷却": "#00ecc3",
  "加速": "#00ecc3",
  "暴击率": "#f5503d",
  "伤害": "#f5503d",
  "飞行": "#f4cf20",
  "冻结": "#00ccff",
  "金币": "#ffd700",
  "治疗": "#8eea31",
  "生命值": "#8eea31",
  "最大生命值": "#8eea31",
  "收入": "#ffcd19",
  "吸血": "#9d4a6f",
  "剧毒": "#0ebe4f",
  "生命再生": "#8eea31",
  "护盾": "#f4cf20",
  "减速": "#cb9f6e",
  "价值": "#ffcd19"
};

const TIER_COLORS = ["#cd7f32", "#c0c0c0", "#ffd700", "#b9f2ff"];

const filterTextForTier = (text: string, currentTier: string) => {
    const tierMap: Record<string, number> = { bronze: 0, silver: 1, gold: 2, diamond: 3 };
    const tierIdx = tierMap[currentTier.toLowerCase()] ?? 0;
    
    // Replace slash-separated numbers with single value
    return text.replace(/(\d+(?:\/\d+)*)/g, (match) => {
        if (!match.includes('/')) return match;
        const nums = match.split('/');
        
        let offset = 0;
        // Guess mapping based on count
        if (nums.length === 4) offset = 0;      // B, S, G, D
        else if (nums.length === 5) offset = 0; // B, S, G, D, L
        else if (nums.length === 3) offset = 1; // S, G, D
        else if (nums.length === 2) offset = 2; // G, D
        
        let targetIdx = tierIdx - offset;
        
        // Clamp index to available range
        if (targetIdx < 0) targetIdx = 0;
        if (targetIdx >= nums.length) targetIdx = nums.length - 1;
        
        return nums[targetIdx];
    });
};

// --- Type Definitions ---
interface SkillText {
    en: string;
    cn: string;
}

interface ResourceCatalogMaps {
    skills_art_map: Record<string, string>;
    item_sizes: Record<string, string>;
}

interface ItemData {
    id?: string;
    uuid: string;
    instance_id?: string;
    name: string;
    name_cn: string;
    name_en?: string;
    tier: string;
    available_tiers: string;
    size?: string;
    tags: string;
    processed_tags: string[];
    hidden_tags?: string;
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
    skills: SkillText[];
    skills_passive?: SkillText[];
    quests?: any;
    enchantments: string[];
    description: string;
    image: string;
    displayImg?: string;
    displayHeroImg?: string;
    // For Monster Sub Items
    current_tier?: string;
    tiers?: Record<string, { cd: string, description: string[], extra_description?: string[] } | null>;
    starting_tier?: string;
}

interface MonsterSubItem extends ItemData {
    // Inherits from ItemData, but explicitly add id for monster skills/items
    id?: string;
    cardFrameImg?: string;
}

interface MonsterData {
    id: string;
    name: string;
    name_zh: string;
    health: number;
    available: string;
    description?: string;
    image: string;
    bgImage?: string;
    skills?: MonsterSubItem[];
    items?: MonsterSubItem[];
    
    displayImg?: string;
    displayImgBg?: string;
}

interface EventChoice {
    name: string;
    name_zh?: string;
    description: string;
    description_zh?: string;
    icon?: string;
    icon_path?: string;
    displayIcon?: string;
}

interface EventData {
    InternalName: string;
    name?: string;
    name_en?: string;
    desc?: string;
    choices: EventChoice[];
    image_paths?: {
        bg?: string;
        char?: string;
    };
    
    displayImgBg?: string;
    displayImgChar?: string;
}

type DetailData = ItemData | MonsterData | EventData;
type DetailType = 'item' | 'monster' | 'event';

// --- Helper Functions ---

const getTierValue = (tierString: string | undefined, tier: string, startingTier?: string) => {
    if (!tierString) return null;
    const values = tierString.split('/').map(v => parseFloat(v.trim()));
    
    const tierMap: {[key: string]: number} = { 'bronze': 0, 'silver': 1, 'gold': 2, 'diamond': 3, 'legendary': 0 };
    const targetTierIndex = tierMap[tier.toLowerCase()] ?? 0;
    
    // 确定物品起始等级索引
    let startTierIndex = 0;
    if (startingTier) {
            const startLower = startingTier.toLowerCase();
            if (startLower.includes('silver')) startTierIndex = 1;
            else if (startLower.includes('gold')) startTierIndex = 2;
            else if (startLower.includes('diamond')) startTierIndex = 3;
    }

    const relativeIndex = targetTierIndex - startTierIndex;
    
    // 如果请求的等级低于起始等级，返回null
    if (relativeIndex < 0) return null;

    // 如果只有1个值（如 "7000"）或者请求的索引超出了提供的值的数量，使用最后一个值
    if (relativeIndex >= values.length && values.length > 0) {
        return values[values.length - 1];
    }
    
    const val = values[relativeIndex];
    return (val !== undefined && !isNaN(val)) ? val : null;
};

// 从描述文本中提取数值
const extractStatFromDescription = (descriptions: string[], statKeyword: string): number | null => {
    if (!descriptions || !Array.isArray(descriptions)) return null;
    
    for (const desc of descriptions) {
        // 匹配 "造成X伤害"、"治疗X生命值"、"+X护盾"、"X灼烧" 等模式
        // 增加对空格的支持，以及 "量提高X" 的支持
        const patterns = [
            new RegExp(`造成\\s*(\\d+)\\s*${statKeyword}`),
            new RegExp(`造成\\s*${statKeyword}\\s*(\\d+)`),
            new RegExp(`治疗\\s*(\\d+)\\s*${statKeyword}`),
            new RegExp(`提高\\s*(\\d+)\\s*${statKeyword}`), 
            new RegExp(`${statKeyword}量提高\\s*(\\d+)`), 
            new RegExp(`获得\\s*(\\d+)\\s*(?:点)?${statKeyword}`),
            new RegExp(`\\+\\s*(\\d+)\\s*${statKeyword}`),
            new RegExp(`(\\d+)\\s*${statKeyword}`),
            new RegExp(`${statKeyword}\\s*(\\d+)`)
        ];
        
        for (const pattern of patterns) {
            const match = desc.match(pattern);
            if (match && match[1]) {
                return parseInt(match[1], 10);
            }
        }
    }
    return null;
};

// 计算派生属性值
const calculateDerivedStats = (descriptions: string[], baseStats: {damage?: number, burn?: number, heal?: number, shield?: number, poison?: number}) => {
    const derived: {damage?: number, burn?: number, heal?: number, shield?: number, poison?: number} = {};
    
    if (!descriptions || !Array.isArray(descriptions)) return derived;
    
    for (const desc of descriptions) {
        // "造成灼烧，等量于此物品伤害"
        if (desc.includes('造成灼烧') && desc.includes('此物品伤害')) {
            if (desc.includes('%')) {
                const percentMatch = desc.match(/(\d+)%/);
                if (percentMatch && baseStats.damage) {
                    derived.burn = Math.ceil(baseStats.damage * parseInt(percentMatch[1]) / 100);
                }
            } else if (!desc.includes('%')) {
                derived.burn = baseStats.damage || 0;
            }
        }
        
        if (desc.includes('造成剧毒') && desc.includes('此物品伤害')) {
            if (desc.includes('%')) {
                const percentMatch = desc.match(/(\d+)%/);
                if (percentMatch && baseStats.damage) {
                    derived.poison = Math.ceil(baseStats.damage * parseInt(percentMatch[1]) / 100);
                }
            } else {
                derived.poison = baseStats.damage || 0;
            }
        }
        
        if (desc.includes('获得护盾') && desc.includes('此物品伤害')) {
            if (desc.includes('%')) {
                const percentMatch = desc.match(/(\d+)%/);
                if (percentMatch && baseStats.damage) {
                    derived.shield = Math.ceil(baseStats.damage * parseInt(percentMatch[1]) / 100);
                }
            } else {
                derived.shield = baseStats.damage || 0;
            }
        }
        
        if (desc.includes('获得治疗') && desc.includes('此物品伤害')) {
            if (desc.includes('%')) {
                const percentMatch = desc.match(/(\d+)%/);
                if (percentMatch && baseStats.damage) {
                    derived.heal = Math.ceil(baseStats.damage * parseInt(percentMatch[1]) / 100);
                }
            } else {
                derived.heal = baseStats.damage || 0;
            }
        }
        
        if (desc.includes('造成伤害') && desc.includes('此物品灼烧')) {
            const multiplerMatch = desc.match(/(\d+)倍/);
            if (multiplerMatch && baseStats.burn) {
                derived.damage = baseStats.burn * parseInt(multiplerMatch[1]);
            }
        }
    }
    
    return derived;
};

// 获取物品的当前tier数值（用于显示标记）
const getTierStatValue = (item: any, tier: string) => {
    const stats: {type: string, value: number, color: string}[] = [];
    
    // 优先使用 _tiers，如果没有则回退到单值字段
    let damage = getTierValue(item.damage_tiers, tier, item.starting_tier);
    if (damage === null && item.damage !== undefined && item.damage > 0) damage = item.damage;
    
    let heal = getTierValue(item.heal_tiers, tier, item.starting_tier);
    if (heal === null && item.heal !== undefined && item.heal > 0) heal = item.heal;
    
    let shield = getTierValue(item.shield_tiers, tier, item.starting_tier);
    if (shield === null && item.shield !== undefined && item.shield > 0) shield = item.shield;
    
    let burn = getTierValue(item.burn_tiers, tier, item.starting_tier);
    if (burn === null && item.burn !== undefined && item.burn > 0) burn = item.burn;
    
    let poison = getTierValue(item.poison_tiers, tier, item.starting_tier);
    if (poison === null && item.poison !== undefined && item.poison > 0) poison = item.poison;
    
    let regen = getTierValue(item.regen_tiers, tier, item.starting_tier);
    if (regen === null && item.regen !== undefined && item.regen > 0) regen = item.regen;
    
    let lifesteal = getTierValue(item.lifesteal_tiers, tier, item.starting_tier);
    if (lifesteal === null && item.lifesteal !== undefined && item.lifesteal > 0) lifesteal = item.lifesteal;
    
    // 如果没有找到数值，从 tiers.description 中提取
    if (item.tiers && item.tiers[tier]) {
        const tierInfo = item.tiers[tier];
        const allDesc = [...(tierInfo.description || []), ...(tierInfo.extra_description || [])];
        
        if (!damage) damage = extractStatFromDescription(allDesc, '伤害');
        if (!heal) heal = extractStatFromDescription(allDesc, '治疗') || extractStatFromDescription(allDesc, '生命值');
        if (!shield) shield = extractStatFromDescription(allDesc, '护盾');
        if (!burn) burn = extractStatFromDescription(allDesc, '灼烧');
        if (!poison) poison = extractStatFromDescription(allDesc, '中毒') || extractStatFromDescription(allDesc, '剧毒');
        if (!regen) regen = extractStatFromDescription(allDesc, '生命再生') || extractStatFromDescription(allDesc, '回复');
        if (!lifesteal) {
            // 吸血通常不带数字，如果描述中有"吸血"就标记为1
            if (allDesc.some(d => d.includes('吸血'))) lifesteal = 1;
        }
        
        // 计算派生属性
        const derived = calculateDerivedStats(allDesc, {damage: damage || undefined, burn: burn || undefined, heal: heal || undefined, shield: shield || undefined, poison: poison || undefined});
        if (derived.damage && !damage) damage = derived.damage;
        if (derived.burn && !burn) burn = derived.burn;
        if (derived.heal && !heal) heal = derived.heal;
        if (derived.shield && !shield) shield = derived.shield;
        if (derived.poison && !poison) poison = derived.poison;
    }
    
    // 特殊处理：lifesteal=100表示将伤害转换为吸血
    if (lifesteal === 100 && damage) {
        // 不显示伤害，用吸血颜色显示伤害数值
        stats.push({ type: 'lifesteal', value: damage, color: '#9d4a6f' });
    } else {
        // 正常情况
        if (damage) stats.push({ type: 'damage', value: damage, color: '#f5503d' });
        if (lifesteal && lifesteal !== 100) stats.push({ type: 'lifesteal', value: lifesteal, color: '#9d4a6f' });
    }
    
    if (heal) stats.push({ type: 'heal', value: heal, color: '#8eea31' });
    if (shield) stats.push({ type: 'shield', value: shield, color: '#f4cf20' });
    if (burn) stats.push({ type: 'burn', value: burn, color: '#ff9f45' });
    if (poison) stats.push({ type: 'poison', value: poison, color: '#5DB75C' });
    if (regen) stats.push({ type: 'regen', value: regen, color: '#8eea31' });

    return stats;
};

// --- Components ---

export default function DetailPopup() {
    const [data, setData] = useState<{ type: DetailType; data: DetailData } | null>(null);
    const [scale, setScale] = useState(0);
    const [isVisible, setIsVisible] = useState(false);
    const [allTags] = useState<string[]>([]);
    const [hoveredMonsterItem, setHoveredMonsterItem] = useState<MonsterSubItem | null>(null);
    const [expandedMonsterItem, setExpandedMonsterItem] = useState<MonsterSubItem | null>(null);
    const [isDragging, setIsDragging] = useState(false);
    const isDraggingRef = useRef(false);
    const resizeStartRef = useRef({ x: 0, y: 0, width: 0, height: 0 });
    const [skillsArtMap, setSkillsArtMap] = useState<Record<string, string>>({});
    const skillsArtMapRef = useRef<Record<string, string>>({}); // Add Ref to access state in listeners
    const itemInfoCacheRef = useRef<Map<string, ItemData | null>>(new Map());
    const imgCache = useRef<Map<string, string>>(new Map());
    const hideTimeoutRef = useRef<any>(null);
    const [contentScale, setContentScale] = useState(1.0);
    const [resizeMode, setResizeMode] = useState<string | null>(null);
    const resizeModeRef = useRef<string | null>(null);
    const wasResizingRef = useRef(false);
    const preloadDoneRef = useRef(false);
    const preloadPromiseRef = useRef<Promise<void> | null>(null);
    
    const ensurePreloadedResources = async () => {
        if (preloadDoneRef.current) return;
        if (preloadPromiseRef.current) {
            await preloadPromiseRef.current;
            return;
        }

        preloadPromiseRef.current = (async () => {
            try {
                const maps = await invoke<ResourceCatalogMaps>('get_resource_catalog_maps');
                setSkillsArtMap(maps.skills_art_map || {});
                preloadDoneRef.current = true;
                console.log('[DetailPopup] Lazy preloaded resources:', {
                    arts: Object.keys(maps.skills_art_map || {}).length,
                });
            } catch (e) {
                console.error('[DetailPopup] Failed to preload resources:', e);
            } finally {
                preloadPromiseRef.current = null;
            }
        })();

        await preloadPromiseRef.current;
    };

    const getItemInfoCached = async (id?: string): Promise<ItemData | null> => {
        const key = (id || '').trim();
        if (!key) return null;
        if (itemInfoCacheRef.current.has(key)) {
            return itemInfoCacheRef.current.get(key) || null;
        }
        try {
            const info = await invoke<ItemData | null>('get_item_info', { id: key });
            itemInfoCacheRef.current.set(key, info || null);
            return info || null;
        } catch {
            itemInfoCacheRef.current.set(key, null);
            return null;
        }
    };
    
    // Sync font scale from settings
    useEffect(() => {
        try {
            const savedSettings = localStorage.getItem('userSettings');
            if (savedSettings) {
                const parsed = JSON.parse(savedSettings);
                const fs = parsed.fontSize || 16;
                document.documentElement.style.setProperty('--font-scale', (fs / 16).toString());
            }
        } catch (e) {
            console.error("Failed to sync font scale", e);
        }
    }, []);

    // Update ref when state changes
    useEffect(() => { skillsArtMapRef.current = skillsArtMap; }, [skillsArtMap]);
    useEffect(() => { isDraggingRef.current = isDragging; }, [isDragging]);
    useEffect(() => { resizeModeRef.current = resizeMode; }, [resizeMode]);
    
    // Restore window geometry on mount
    useEffect(() => {
        (async () => {
            try {
                const geometry = await invoke<{ x: number; y: number; width: number; height: number } | null>('get_window_geometry', { label: 'detail-popup' });
                if (geometry) {
                    const win = getCurrentWindow();
                    await win.setPosition(new PhysicalPosition(geometry.x, geometry.y));
                    await win.setSize(new PhysicalSize(geometry.width, geometry.height));
                }
            } catch (e) {
                console.error('Failed to restore geometry:', e);
            }
        })();
    }, []);
    
    // 全局监听鼠标事件
    useEffect(() => {
        const handleMouseUp = () => {
            if (isDraggingRef.current) {
                setTimeout(() => {
                    setIsDragging(false);
                    isDraggingRef.current = false;
                }, 150);
            }
            if (resizeModeRef.current === 'zoom') {
                document.body.style.cursor = '';
                setResizeMode(null);
                resizeModeRef.current = null;
            } else if (resizeModeRef.current) { 
                // Cleanup other resize modes if any sticky state remains
                document.body.style.cursor = '';
                setResizeMode(null);
                resizeModeRef.current = null;
            }
        };

        const handleMouseMove = async (e: MouseEvent) => {
            if (resizeModeRef.current === 'zoom') {
                document.body.style.cursor = 'nwse-resize';
                // Use y-axis movement to control zoom
                const deltaY = e.clientY - resizeStartRef.current.y;
                // Reuse 'width' in resizeStartRef as starting scale
                const startScale = resizeStartRef.current.width;
                const sensitivity = 0.005;
                const newScale = Math.max(0.5, Math.min(3.0, startScale + deltaY * sensitivity));
                setContentScale(newScale);
            }
        };
        window.addEventListener('mouseup', handleMouseUp);
        window.addEventListener('mousemove', handleMouseMove);

        // Listen for native resize/move to save geometry
        let unlistenResize: Promise<() => void> | null = null;
        let unlistenMove: Promise<() => void> | null = null;

        const saveGeometry = debounce(async () => {
            try {
                const win = getCurrentWindow();
                const pos = await win.outerPosition();
                // Use innerSize for saving to avoid "growing window" issue on Windows
                // because setSize sets the inner size, but outerSize includes invisible borders.
                const size = await win.innerSize();
                await invoke('save_detail_popup_geometry', {
                    x: pos.x, y: pos.y, width: size.width, height: size.height
                });
            } catch (e) { console.error(e); }
        }, 500);

        // Use official helper methods for events
        const win = getCurrentWindow();
        unlistenResize = win.onResized(saveGeometry);
        unlistenMove = win.onMoved(saveGeometry);

        return () => {
            window.removeEventListener('mouseup', handleMouseUp);
            window.removeEventListener('mousemove', handleMouseMove);
            if (unlistenResize) unlistenResize.then(f => f());
            if (unlistenMove) unlistenMove.then(f => f());
        };
    }, []);
    
    const containerRef = useRef<HTMLDivElement>(null);
    const MAX_CACHE_SIZE = 200;

    // NOTE: resources are lazily preloaded on first show-detail-popup event.

    const getImg = async (path: string | null | undefined) => {
        if (!path) return "";
        if (imgCache.current.has(path)) {
            const value = imgCache.current.get(path)!;
            return value;
        }
        try {
            // 与 App.tsx 一致：resources/${path}
            const fullPath = await resolveResource(`resources/${path}`);
            const url = convertFileSrc(fullPath);
            if (imgCache.current.size >= MAX_CACHE_SIZE) {
                const firstKey = imgCache.current.keys().next().value;
                if (firstKey) imgCache.current.delete(firstKey);
            }
            imgCache.current.set(path, url);
            console.log("[DetailPopup getImg] Successfully loaded:", path, "->", url);
            return url;
        } catch (e) {
            console.error("[DetailPopup getImg] Failed to load:", path, e);
            return "";
        }
    };

    // 渲染文本（照抄 App.tsx）
    const renderText = (text: any) => {
        if (!text) return null;
        
        let content = "";
        if (typeof text === 'string') {
          content = text;
        } else if (text.cn) {
          content = text.cn;
        } else if (text.en) {
          content = text.en;
        } else {
          return null;
        }
        
        const parts = content.split(/(\d+(?:\/\d+)+)/g);
        
        return parts.map((part, i) => {
          if (part.includes('/')) {
            const nums = part.split('/');
            return (
              <span key={i} className="progression-nums">
                {nums.map((n, idx) => {
                  let colorIdx = idx;
                  if (nums.length === 2) colorIdx = idx + 2;
                  else if (nums.length === 3) colorIdx = idx + 1;
                  
                  return (
                    <span key={idx}>
                      <span style={{ color: TIER_COLORS[colorIdx] || '#fff', fontWeight: 'bold' }}>{n}</span>
                      {idx < nums.length - 1 && <span style={{ color: '#fff' }}>/</span>}
                    </span>
                  );
                })}
              </span>
            );
          }
    
          const keywords = Object.keys(KEYWORD_COLORS);
          const allMatches = [...new Set([...keywords, ...allTags])].filter(k => k.length > 0);
          
          if (allMatches.length === 0) return part;
          
          const regex = new RegExp(`(${allMatches.join('|')})`, 'g');
          const subParts = part.split(regex);
          
          return subParts.map((sub, j) => {
            if (KEYWORD_COLORS[sub]) {
              return <span key={j} style={{ color: KEYWORD_COLORS[sub], fontWeight: 'bold' }}>{sub}</span>;
            }
            if (allTags.includes(sub)) {
              return <span key={j} style={{ color: '#8eba31', fontWeight: 'bold' }}>{sub}</span>;
            }
            return sub;
          });
        });
    };

    useEffect(() => {
        console.log("[DetailPopup] Component mounted, setting up listeners");
        
        // 监听容器点击事件，点击外部关闭
        // const handleContainerClick = (e: MouseEvent) => {
        //     // 我们检查点击是否在内容区域外，或者是在我们的容器背景上
        //     if (e.target === e.currentTarget || (e.target as HTMLElement).classList.contains('detail-popup-container')) {
        //         console.log("[DetailPopup] Clicked outside content, hiding");
        //         invoke("hide_detail_popup");
        //     }
        // };
        
        // 使用一个稳定的监听器
        let showUnlisten: (() => void) | null = null;
        let hideUnlisten: (() => void) | null = null;

        const setupListeners = async () => {
            showUnlisten = await listen<{ type: DetailType; data: DetailData }>(
                "show-detail-popup",
                async (event) => {
                    console.log("[DetailPopup] Received show-detail-popup event:", event.payload);
                    await ensurePreloadedResources();
                    
                    // Cancel pending hide
                    if (hideTimeoutRef.current) {
                        clearTimeout(hideTimeoutRef.current);
                        hideTimeoutRef.current = null;
                        console.log("[DetailPopup] Cancelled pending hide due to new show event");
                    }

                    const { type, data: rawData } = event.payload;
                    
                    let processedData: any = { ...rawData };

                    if (type === 'item') {
                         const itemData = rawData as ItemData;
                         console.log("[DetailPopup] Item uuid:", itemData.uuid, "name:", itemData.name);
                         let imageUrl = "";
                         if (itemData.uuid || itemData.name) {
                             const imagePath = `images/${itemData.uuid || itemData.name}.webp`;
                             imageUrl = await getImg(imagePath);
                         }
                         
                         // Hero Avatar Logic
                         let displayHeroImg = "";
                         if (itemData.heroes && itemData.heroes.length > 0) {
                            const rawHero = itemData.heroes[0];
                            // Handle format like "HeroName" or "HeroName / Other"
                            // Map known heroes to filenames
                            const heroKey = rawHero.split(' / ')[0].trim();
                            
                            const heroFileMap: Record<string, string> = {
                                'pygmalien': 'pygmalien',
                                'jules': 'jules',
                                'vanessa': 'vanessa',
                                'mak': 'mak',
                                'dooley': 'dooley',
                                'stelle': 'stelle',
                                // Add mappings for potential different casing or names
                                'pig': 'pygmalien',
                                'common': '' // Common items have no hero avatar
                            };
                            
                            const normalizedKey = heroKey.toLowerCase();
                            // Direct match or lookup in map
                            const filename = heroFileMap[normalizedKey] || (Object.values(heroFileMap).includes(normalizedKey) ? normalizedKey : null);
                            
                            if (filename) {
                                displayHeroImg = await getImg(`images/heroes/${filename}.webp`);
                            }
                         }
                         processedData.displayHeroImg = displayHeroImg;

                         processedData.displayImg = imageUrl || itemData.displayImg;
                    } else if (type === 'monster') {
                         const m = rawData as MonsterData;
                         
                         // 1. Monster Image
                         let filename = m.image ? m.image.split('/').pop() || `${m.name_zh}.webp` : `${m.name_zh}.webp`;
                         let displayImg = await getImg(`assets/monsters/characters/${filename}`);
                         
                         if (!displayImg && m.name_zh.includes(' ')) {
                             const spacePos = m.name_zh.lastIndexOf(' ');
                             const baseName = m.name_zh.substring(spacePos + 1);
                             const fallbackFilename = `${baseName}.webp`;
                             const fallbackImg = await getImg(`assets/monsters/characters/${fallbackFilename}`);
                             if (fallbackImg) { 
                                 displayImg = fallbackImg; 
                                 filename = fallbackFilename; 
                             }
                         }

                         // 2. Background Image
                         let bgFilename = filename;
                         if (m.name_zh === '绿洲守护神') bgFilename = '绿洲守护神_Day9.webp';
                         const displayImgBg = await getImg(`assets/monsters/backgrounds/${bgFilename}`);

                         processedData = { ...m, displayImg, displayImgBg };
                         
                         // 3. Skills - 从后端DB按需合并完整信息
                        if (m.skills) {
                             processedData.skills = await Promise.all(m.skills.map(async s => {
                                let imgPath = '';
                                const id = s.id || s.uuid; 
                                
                                // 从后端获取完整技能信息
                                const fullSkillInfo = await getItemInfoCached(id);
                                
                                // 合并技能信息
                                let mergedSkill = s;
                                if (fullSkillInfo) {
                                    mergedSkill = {
                                        ...s,
                                        skills: (fullSkillInfo.skills as any) || s.skills || [],
                                        name: fullSkillInfo.name_cn || fullSkillInfo.name || s.name,
                                        name_cn: fullSkillInfo.name_cn,
                                        name_en: fullSkillInfo.name,
                                        size: fullSkillInfo.size || s.size,
                                        starting_tier: (fullSkillInfo as any).starting_tier || fullSkillInfo.tier || s.starting_tier
                                    };
                                }
                                
                                // 获取技能图标
                                const art = id ? skillsArtMapRef.current[id] : undefined;
                                if (art) {
                                    const base = art.split('/').pop() || art;
                                    const nameNoExt = base.replace(/\.[^/.]+$/, "");
                                    imgPath = `images/skill/${nameNoExt}.webp`;
                                } else {
                                    imgPath = `images/${id || s.name}.webp`;
                                }
                                
                                return { ...mergedSkill, displayImg: await getImg(imgPath) };
                             }));
                         }
                         
                         // 4. Items - 从后端DB按需合并完整信息
                         if (m.items) {
                             processedData.items = await Promise.all(m.items.map(async i => {
                                 const id = i.id || i.uuid;
                                 let imgPath = `images/${id || i.name}.webp`;
                                 
                                 // 从后端DB中按 id 查找完整信息
                                 const fullItemInfo = await getItemInfoCached(id);
                                 
                                 // 合并：优先使用 items_db 的完整信息
                                 const merged = fullItemInfo ? 
                                     { 
                                         ...i,  // 先用 monsters_db 的基础信息
                                         ...fullItemInfo, // 覆盖为 items_db 的完整信息
                                         // 但保留 monsters_db 中的运行时字段
                                         current_tier: i.current_tier || fullItemInfo.current_tier,
                                         tier: i.tier || fullItemInfo.tier
                                     } : i;
                                 
                                 // 计算 CardFrame 路径
                                 let currentTier = (merged.current_tier || 'bronze').toLowerCase();
                                 const sizeStr = (merged.size || 'Medium / 中型').split(' / ')[0].toLowerCase().trim();
                                 
                                 const tierNameMap: Record<string, string> = {
                                     bronze: 'Bronze', silver: 'Silver', gold: 'Gold', diamond: 'Diamond', legendary: 'Legendary'
                                 };
                                 const sizeNameMap: Record<string, string> = {
                                     small: 'S', medium: 'M', large: 'L'
                                 };
                                 const tierName = tierNameMap[currentTier] || 'Bronze';
                                 const sizeName = sizeNameMap[sizeStr] || 'M';
                                const cardFramePath = `assets/gui/CardFrame_${tierName}_${sizeName}_TUI.webp`;
                                 
                                 const itemImg = await getImg(imgPath);
                                 const frameImg = await getImg(cardFramePath);
                                 
                                 console.log(`[CardFrame] ${merged.name} - itemImg: ${itemImg ? 'OK' : 'FAILED'}, frameImg: ${frameImg ? 'OK' : 'FAILED'}`);
                                 
                                 return { 
                                     ...merged, 
                                     displayImg: itemImg,
                                     cardFrameImg: frameImg
                                 };
                             }));
                         }

                    } else if (type === 'event') {
                         const e = rawData as EventData;
                         let displayImgBg = "";
                         let displayImgChar = "";
                         
                         if (e.image_paths?.bg) displayImgBg = await getImg(e.image_paths.bg);
                         if (e.image_paths?.char) displayImgChar = await getImg(e.image_paths.char);
                         
                         const processedChoices = e.choices ? await Promise.all(e.choices.map(async choice => {
                             const iconImg = choice.icon_path ? await getImg(choice.icon_path) : '';
                             return { ...choice, displayIcon: iconImg };
                         })) : [];

                         processedData = { ...e, displayImgBg, displayImgChar, choices: processedChoices };
                    }
                    
                    setData({ type, data: processedData });
                    setIsVisible(true);
                    
                    // 确保先设置为0，触发一次渲染
                    setScale(0);
                    
                    // 使用 requestAnimationFrame 确保下一帧动画生效
                    requestAnimationFrame(() => {
                        requestAnimationFrame(() => {
                            setScale(1);
                        });
                    });
                }
            );

            hideUnlisten = await listen("hide-detail-popup", async () => {
                console.log("[DetailPopup] Received hide-detail-popup event");
                
                if (hideTimeoutRef.current) {
                    clearTimeout(hideTimeoutRef.current);
                }

                setScale(0);
                
                hideTimeoutRef.current = setTimeout(async () => {
                    setIsVisible(false);
                    setData(null);
                    
                    // 只有在完全隐藏后才缩小窗口，避免动画过程中由于窗口缩小导致的闪烁
                    const currentWindow = getCurrentWindow();
                    try {
                        // 使用逻辑像素 1x1，或者至少 10x10 确保不会导致某些显卡驱动问题
                        await currentWindow.setSize(new PhysicalSize(1, 1));
                        console.log("[DetailPopup] Window size set to 1x1 after fade out");
                    } catch (e) {
                        console.error("[DetailPopup] Failed to resize window:", e);
                    }
                }, 300);
            });
        };

        setupListeners();

        return () => {
            console.log("[DetailPopup] Component unmounting, cleaning up listeners");
            if (showUnlisten) showUnlisten();
            if (hideUnlisten) hideUnlisten();
        };
    }, []); // 去掉 isVisible 依赖，只在加载时注册一次

    // 无论是否可见，都保持组件挂载，通过 isVisible 控制渲染
    // 这样 containerRef 始终是稳定的
    if (!isVisible || !data) {
        return (
            <div 
                ref={containerRef}
                style={{
                    width: '1px',
                    height: '1px',
                    opacity: 0,
                    pointerEvents: 'none',
                    overflow: 'hidden'
                }} 
            />
        );
    }

    // --- Render Helpers ---

    // Helper to merge description lines from all tiers
    const getUnifiedDescription = (item: MonsterSubItem): string[] => {
        if (!item || !item.tiers) return [];
        const order = ["bronze", "silver", "gold", "diamond", "legendary"];
        const tierKeys = order.filter(k => item.tiers![k]);
        if (tierKeys.length <= 1) return [];

        const baseTier = item.tiers![tierKeys[0]];
        if (!baseTier || !baseTier.description) return [];

        const unifiedLines: string[] = [];
        const splitRegex = /(\d+(?:\.\d+)?)/g;

        for (let i = 0; i < baseTier.description.length; i++) {
            const lines = tierKeys.map(k => item.tiers![k]?.description[i] || "");
            if (lines.some(l => l !== lines[0])) {
                try {
                    const partsList = lines.map(l => l.split(splitRegex));
                    const baseParts = partsList[0];
                    let consistent = true;

                    for (let j = 1; j < partsList.length; j++) {
                        if (partsList[j].length !== baseParts.length) {
                            consistent = false; break;
                        }
                        
                        for (let k = 0; k < baseParts.length; k += 2) {
                            if (partsList[j][k] !== baseParts[k]) {
                                consistent = false; break;
                            }
                        }
                    }

                    if (consistent) {
                        let mergedLine = "";
                        for (let k = 0; k < baseParts.length; k++) {
                            if (k % 2 === 0) {
                                mergedLine += baseParts[k];
                            } else {
                                const nums = partsList.map(p => p[k]);
                                if (nums.every(n => n === nums[0])) {
                                    mergedLine += nums[0];
                                } else {
                                    // Use custom markup or just slashes. 
                                    // To colorize, we might need to rely on renderText handling or inject span-like syntax if allowed.
                                    // For now, simple slash separation.
                                    mergedLine += nums.join("/");
                                }
                            }
                        }
                        unifiedLines.push(mergedLine);
                    } else {
                        return []; 
                    }
                } catch (e) { return []; }
            } else {
                unifiedLines.push(lines[0]);
            }
        }
        return unifiedLines;
    };

    const renderTierInfo = (item: MonsterSubItem, showUnified: boolean = false) => {
        if (!item) return null;
        let currentTier = "bronze";
        const tiers = item.tiers || {};
        if (item.current_tier) {
            currentTier = item.current_tier.toLowerCase();
        } else {
            const availableTiers = Object.keys(tiers);
            if (availableTiers.length > 0) {
                if (availableTiers.includes("bronze")) currentTier = "bronze";
                else if (availableTiers.includes("silver")) currentTier = "silver";
                else if (availableTiers.includes("gold")) currentTier = "gold";
                else currentTier = availableTiers[0];
            }
        }
        
        // Unified description only if requested (for expanded view)
        const unifiedDesc = showUnified ? getUnifiedDescription(item) : [];
        const tierData = tiers[currentTier] || Object.values(tiers).find(t => t !== null);
        
        const sizeClass = (item.size || 'Medium').split(' / ')[0].toLowerCase();
        const borderColorMap: Record<string, string> = {
            bronze: "#CD7F32", silver: "#C0C0C0", gold: "#FFD700", diamond: "#B9F2FF", legendary: "#FF4500",
        };
        const borderColor = borderColorMap[currentTier] || borderColorMap.bronze;

        const processText = (t: any) => {
            if (showUnified) return t;
            if (typeof t === 'string') return filterTextForTier(t, currentTier);
            if (t && typeof t === 'object') {
                // If it's an object (cn/en), map each property
                const newObj: any = { ...t };
                if (newObj.cn) newObj.cn = filterTextForTier(newObj.cn, currentTier);
                if (newObj.en) newObj.en = filterTextForTier(newObj.en, currentTier);
                return newObj;
            }
            return t;
        };

        return (
            <div key={item.name} className={`sub-item-card tier-${currentTier}`} style={{ 
                border: `1px solid ${borderColor}`,
                borderLeft: `5px solid ${borderColor}`, 
                marginBottom: '6px',
                boxShadow: `0 0 5px ${borderColor}40`
            }}>
                <div className="sub-item-header">
                    <div className={`image-box size-${sizeClass}`} style={{ border: `1px solid ${borderColor}`, alignSelf: 'center' }}>
                         {item.displayImg && <img src={item.displayImg} alt="" />}
                    </div>
                    <div className="sub-item-title-row">
                        <span className="sub-item-name">{item.name}</span>
                        {tierData && tierData.cd && <div className="sub-item-cd">⏳ {tierData.cd}s</div>}
                    </div>
                </div>
                <div className="sub-item-desc" style={{ padding: '4px 8px', fontSize: '13px', color: '#ccc' }}>
                    {/* 优先显示 items_db 中的 skills 字段 */}
                    {item.skills && item.skills.length > 0 ? (
                        item.skills.map((skill, i) => (
                            <div key={i} className="desc-line" style={{ 
                                marginTop: i > 0 ? '4px' : '0',
                                borderTop: i > 0 ? '1px solid rgba(255,255,255,0.1)' : 'none',
                                paddingTop: i > 0 ? '4px' : '0'
                            }}>
                                {renderText(processText(skill))}
                            </div>
                        ))
                    ) : (
                        // 如果没有 skills，则使用 tiers.description
                        <>
                            {unifiedDesc.length > 0 ? 
                                unifiedDesc.map((d, i) => <div key={i} className="desc-line">{renderText(d)}</div>)
                                : (tierData && tierData.description.map((d, i) => <div key={i} className="desc-line">{renderText(processText(d))}</div>))
                            }
                            {tierData && tierData.extra_description && tierData.extra_description.map((d, i) => <div key={`extra-${i}`} className="desc-line extra" style={{ color: '#aaa', fontStyle: 'italic' }}>{renderText(processText(d))}</div>)}
                        </>
                    )}
                </div>
            </div>
        );
    };

    const renderItemContent = (item: ItemData) => {
        return (
            <UnifiedItemCard
                item={item as any}
                allTags={allTags}
                isExpanded={true}
                isPinned={false}
                isRecognized={false}
                isTopMatch={false}
                showPin={false}
                showExpandChevron={false}
            />
        );
    };

    const renderMonsterContent = (m: MonsterData) => {
         return (
             <div className="monster-card-v2 expanded" style={{ border: 'none', boxShadow: 'none', background: 'transparent', margin: 0, padding: 0, overflow: 'visible' }}>
                <div className="monster-header-v2" style={{ marginBottom: '16px', background: 'rgba(255,255,255,0.03)', borderRadius: '8px' }}>
                    <div className="avatar-wrap">
                        <div className="monster-image-layers">
                            {m.displayImgBg && <img src={m.displayImgBg} className="monster-layer-bg" alt="" />}
                            {m.displayImg && <img src={m.displayImg} className="monster-layer-char" alt="" />}
                        </div>
                    </div>
                    <div className="monster-info-v2">
                        <div className="monster-name-zh">
                            {m.name_zh}
                        </div>
                        <div className="monster-health">❤️ {m.health?.toString() || m.health}</div>
                    </div>
                    <div className="monster-available-tag">
                        {m.available}
                    </div>
                </div>
                <div className="monster-assets-grid" style={{ gridTemplateColumns: '1fr', padding: 0, background: 'transparent' }}>
                    {m.skills && m.skills.length > 0 && (
                        <div className="assets-section">
                            <div className="section-title">技能 (Skills)</div>
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                {m.skills.map((s, idx) => {
                                    const isExpanded = expandedMonsterItem === s;
                                    return (
                                        <div 
                                            key={idx} 
                                            style={{ cursor: 'pointer' }} 
                                            onClick={() => {
                                                setExpandedMonsterItem(isExpanded ? null : s);
                                                setHoveredMonsterItem(null);
                                            }}
                                        >
                                            {renderTierInfo(s, isExpanded)}
                                        </div>
                                    );
                                })}
                            </div>
                        </div>
                    )}
                    {m.items && m.items.length > 0 && (
                        <div className="assets-section">
                            <div className="section-title" style={{ marginBottom: '8px' }}>物品 (Items)</div>
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                                <div style={{ display: 'flex', flexDirection: 'row', alignItems: 'flex-start', flexWrap: 'wrap', gap: '8px' }}>
                                    {m.items.map((it, idx) => {
                                        if (!it) return null;
                                        // Determine Tier & Color
                                        let currentTier = "bronze";
                                        const tiers = it.tiers || {};
                                        if (it.current_tier) {
                                            currentTier = it.current_tier.toLowerCase();
                                        } else {
                                             const availableTiers = Object.keys(tiers);
                                             if (availableTiers.length > 0) {
                                                if (availableTiers.includes("bronze")) currentTier = "bronze";
                                                else if (availableTiers.includes("silver")) currentTier = "silver";
                                                else if (availableTiers.includes("gold")) currentTier = "gold";
                                                else currentTier = availableTiers[0];
                                             }
                                        }
                                        
                                        // 从 size 字段提取尺寸 (如 "Small / 小型" -> "small")
                                        const sizeStr = (it.size || 'Medium / 中型').split(' / ')[0].toLowerCase().trim();
                                        
                                        const isHovered = hoveredMonsterItem === it;
                                        const isExpanded = expandedMonsterItem === it;
                                        
                                        const stats = getTierStatValue(it, currentTier);
                                        // Get CD (Simplified)
                                        let cdValue: string | null = null;
                                        if (it.cooldown_tiers && it.cooldown_tiers.trim()) {
                                            const cd = getTierValue(it.cooldown_tiers, currentTier, it.starting_tier);
                                            if (cd !== null && cd > 0) cdValue = (cd > 100 ? cd / 1000 : cd).toFixed(1);
                                        }
                                        if (!cdValue && it.cooldown && it.cooldown > 0) {
                                            cdValue = (it.cooldown > 100 ? it.cooldown / 1000 : it.cooldown).toFixed(1);
                                        }
                                        if (!cdValue && it.tiers && it.tiers[currentTier]) {
                                             const tierData = it.tiers[currentTier];
                                             if (tierData && tierData.cd && tierData.cd.trim()) cdValue = tierData.cd;
                                        }

                                        // const borderColorMap: Record<string, string> = {
                                        //     bronze: "#CD7F32", silver: "#C0C0C0", gold: "#FFD700", diamond: "#B9F2FF", legendary: "#FF4500",
                                        // };
                                        // const borderColor = borderColorMap[currentTier] || borderColorMap.bronze;
                                        
                                        // CardFrame 路径
                                        // const tierNameMap: Record<string, string> = {
                                        //     bronze: 'Bronze', silver: 'Silver', gold: 'Gold', diamond: 'Diamond', legendary: 'Legendary'
                                        // };
                                        // const sizeNameMap: Record<string, string> = {
                                        //     small: 'S', medium: 'M', large: 'L'
                                        // };
                                        // const tierName = tierNameMap[currentTier] || 'Bronze';
                                        // const sizeName = sizeNameMap[sizeStr] || 'M';
                                        // const cardFramePath = `assets/gui/CardFrame_${tierName}_${sizeName}_TUI.webp`;
                                        
                                        // 根据size确定宽度和高度 (再次放大)
                                        let boxWidth = 88;
                                        let boxHeight = 88;
                                        if (sizeStr === 'small') {
                                            boxWidth = 44;
                                            boxHeight = 88;
                                        } else if (sizeStr === 'medium') {
                                            boxWidth = 88;
                                            boxHeight = 88;
                                        } else if (sizeStr === 'large') {
                                            boxWidth = 132;
                                            boxHeight = 88;
                                        }
                                        
                                        return (
                                            <div 
                                                key={`${it.name}-${idx}`}
                                                className="monster-item-card"
                                                style={{
                                                    width: `${boxWidth}px`,
                                                    height: `${boxHeight}px`,
                                                    cursor: 'pointer',
                                                    position: 'relative',
                                                    display: 'flex',
                                                    alignItems: 'center',
                                                    justifyContent: 'center',
                                                    zIndex: isHovered ? 100 : 1
                                                }}
                                                onMouseEnter={() => {
                                                    setHoveredMonsterItem(it);
                                                    if (expandedMonsterItem && expandedMonsterItem !== it) {
                                                        setExpandedMonsterItem(null);
                                                    }
                                                }}
                                                onMouseLeave={() => setHoveredMonsterItem(null)}
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    setExpandedMonsterItem(isExpanded ? null : it);
                                                }}
                                            >
                                                {/* 第1层：物品图片（最底层）*/}
                                                {it.displayImg && (
                                                    <img 
                                                        src={it.displayImg} 
                                                        alt={it.name}
                                                        style={{
                                                            position: 'absolute',
                                                            top: 0,
                                                            left: 0,
                                                            width: '100%',
                                                            height: '100%',
                                                            objectFit: 'fill',
                                                            zIndex: 1
                                                        }}
                                                    />
                                                )}
                                                
                                                {/* 第2层：CardFrame 外边框 */}
                                                {it.cardFrameImg && (
                                                    <img 
                                                        src={it.cardFrameImg}
                                                        alt=""
                                                        style={{
                                                            position: 'absolute',
                                                            top: 0,
                                                            left: 0,
                                                            width: '100%',
                                                            height: '100%',
                                                            objectFit: 'fill',
                                                            pointerEvents: 'none',
                                                            zIndex: 2
                                                        }}
                                                    />
                                                )}
                                                
                                                {/* 第3层：Stats 标签（最上层）*/}
                                                
                                                {stats.length > 0 && (
                                                    <div style={{
                                                        position: 'absolute',
                                                        top: '-8px',
                                                        left: '50%',
                                                        transform: 'translateX(-50%)',
                                                        display: 'flex',
                                                        gap: '4px',
                                                        zIndex: 10
                                                    }}>
                                                        {stats.map((stat, i) => (
                                                            <div key={i} style={{
                                                                background: stat.color,
                                                                color: '#fff',
                                                                fontSize: '11px',
                                                                fontWeight: 'bold',
                                                                padding: '2px 6px',
                                                                borderRadius: '4px',
                                                                boxShadow: '0 2px 4px rgba(0,0,0,0.8)',
                                                                textShadow: '0 1px 2px rgba(0,0,0,0.8)'
                                                            }}>
                                                                {stat.value}
                                                            </div>
                                                        ))}
                                                    </div>
                                                )}
                                                
                                                {cdValue && (
                                                    <div style={{
                                                        position: 'absolute',
                                                        bottom: '4px',
                                                        left: '4px',
                                                        background: 'rgba(0,0,0,0.6)',
                                                        color: '#fff',
                                                        fontSize: '11px',
                                                        padding: '1px 3px',
                                                        borderRadius: '3px',
                                                        zIndex: 10
                                                    }}>
                                                        ⏳ {cdValue}
                                                    </div>
                                                )}

                                            </div>
                                        );
                                    })}
                                </div>
                                {/* Hovered Item details (always at the bottom of the section) */}
                                {hoveredMonsterItem && !expandedMonsterItem && (
                                     <div style={{ 
                                        marginTop: '4px', 
                                        animation: 'fadeIn 0.2s ease',
                                        background: 'rgba(0,0,0,0.6)',
                                        borderRadius: '8px',
                                        overflow: 'hidden',
                                        zIndex: 100,
                                        position: 'relative',
                                        border: '1px solid rgba(255, 205, 25, 0.3)'
                                    }}>
                                       <div className="section-title" style={{ padding: '8px 8px 0 8px', fontSize: '14px', color: '#ffd700' }}>
                                           {hoveredMonsterItem.name_cn || hoveredMonsterItem.name} 
                                           <span style={{ fontSize: '12px', color: '#888', marginLeft: '8px' }}>
                                               (点击查看所有等级 / Click for all tiers)
                                           </span>
                                       </div>
                                       {/* Only show the current tier info on hover */}
                                       {renderTierInfo(hoveredMonsterItem)}
                                    </div>
                                )}
                                {/* Expanded Monster Item - Shows ALL tiers or detailed info */}
                                {expandedMonsterItem && (!m.skills || !m.skills.includes(expandedMonsterItem)) && (
                                    <div style={{ 
                                        marginTop: '4px', 
                                        animation: 'fadeIn 0.2s ease',
                                        background: 'rgba(0,0,0,0.8)',
                                        borderRadius: '8px',
                                        overflow: 'hidden',
                                        border: '1px solid rgba(255, 205, 25, 0.5)',
                                        padding: '5px'
                                    }}>
                                       <div className="section-title" style={{ padding: '8px 8px 8px 8px', fontSize: '14px', color: '#ffd700', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>
                                           {expandedMonsterItem.name_cn || expandedMonsterItem.name}
                                           <span style={{ fontSize: '12px', color: '#aaa', marginLeft: '8px' }}>
                                               (所有等级详情 / All Tiers)
                                           </span>
                                       </div>
                                       
                                       {/* Render Unified Tier Info */}
                                       <div style={{ marginTop: '8px' }}>
                                            {renderTierInfo(expandedMonsterItem, true)}
                                       </div>
                                    </div>
                                )}
                            </div>
                        </div>
                    )}
                </div>
                {/* 
                  Add an outer border based on some logic if needed. 
                  Currently the detail popup has a fixed gold border. 
                  If we want it to match the monster rarity or something, we can dynamic style it.
                  User asked: "Outer level border". 
                  Assuming "Outer level border" means the popup border should reflect the tier/rarity if applicable, 
                  or just the gold border mentioned in css. Since Monster doesn't technically have a "Tier" like items,
                  maybe they mean the border around the items in the grid?
                  
                  Actually, user said "Monster view... outer level border". 
                  In OverlayApp, the monster card doesn't have a colored border like items.
                  But the user might be referring to the "Item Card" style border effectively being applied to the whole view.

                  Wait, "outer level border" might refer to the `sub-item-card` needing a border color.
                  We already have `borderLeft: 4px solid ...`.
                  Let's re-read the request. "External grade border" (最外层的等级边框). 
                  In the Item view, we have `item-card tier-X` which adds a border color. 
                  For Monster view, maybe they want a border around the whole popup? 
                  
                  Or maybe they mean this specific part: 
                  In `renderItemContent`, we have `<div className="item-card tier-${tierClass}">`.
                  In `renderMonsterContent`, we have `<div className="monster-card-v2 ...">`.
                  
                  If the use is talking about the popup window border, it's currently hardcoded gold.
                  If the user is talking about consistent styling with the overlay, maybe I should wrap the monster content in a border?
                  
                  Let's stick to the current plan: fix hover/click and ensure sub-items have their borders (which they do via `renderTierInfo`).
                  The "border around the image" in `renderTierInfo` is `outline: 2px solid ...`.
                  
                  Let's double check `renderTierInfo`.
                */}
             </div>
         );
    };

    const renderEventContent = (e: EventData) => {
        const eventTitle = e.name || e.name_en || e.InternalName || '';
        const eventDesc = e.desc || '';
        
        return (
            <div className="event-card-container" style={{ border: 'none', boxShadow: 'none', background: 'transparent', margin: 0, padding: 0 }}>
                {/* 事件标题区 */}
                <div style={{
                    marginBottom: '16px',
                    background: 'rgba(255,255,255,0.03)',
                    borderRadius: '8px',
                    padding: '12px',
                    position: 'relative'
                }}>
                    {e.displayImgBg && (
                        <div style={{
                            position: 'absolute',
                            top: 0,
                            left: 0,
                            right: 0,
                            bottom: 0,
                            borderRadius: '8px',
                            overflow: 'hidden',
                            opacity: 0.15,
                            zIndex: 0
                        }}>
                            <img src={e.displayImgBg} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />
                        </div>
                    )}
                    <div style={{ position: 'relative', zIndex: 1, display: 'flex', gap: '12px', alignItems: 'center' }}>
                        {e.displayImgChar && (
                            <img src={e.displayImgChar} alt={eventTitle} style={{ 
                                width: '80px', 
                                height: '80px', 
                                borderRadius: '8px',
                                border: '2px solid rgba(255,205,25,0.3)',
                                objectFit: 'cover'
                            }} />
                        )}
                        <div style={{ flex: 1 }}>
                            <div style={{ 
                                fontSize: '18px', 
                                fontWeight: 'bold', 
                                color: '#ffd700',
                                marginBottom: '4px'
                            }}>
                                {eventTitle}
                            </div>
                            {eventDesc && (
                                <div style={{ 
                                    fontSize: '13px', 
                                    color: 'rgba(255,255,255,0.7)',
                                    lineHeight: '1.4'
                                }}>
                                    {renderText(eventDesc)}
                                </div>
                            )}
                        </div>
                    </div>
                </div>

                {/* 事件选项 */}
                {e.choices && e.choices.length > 0 && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                        {e.choices.map((choice: any, idx) => (
                            <div key={idx} style={{
                                background: 'rgba(255,255,255,0.05)',
                                border: '1px solid rgba(255,205,25,0.2)',
                                borderRadius: '6px',
                                padding: '10px',
                                display: 'flex',
                                gap: '12px',
                                alignItems: 'center',
                            }}>
                                {choice.displayIcon && (
                                    <img src={choice.displayIcon} alt="" style={{
                                        width: '48px',
                                        height: '48px',
                                        borderRadius: '6px',
                                        border: '1px solid rgba(255,255,255,0.2)',
                                        objectFit: 'cover'
                                    }} />
                                )}
                                <div style={{ flex: 1 }}>
                                    <div style={{ 
                                        fontSize: '14px', 
                                        fontWeight: 'bold',
                                        color: '#ffd700',
                                        marginBottom: '4px'
                                    }}>
                                        {choice.name_zh || choice.name || ''}
                                    </div>
                                    {choice.description_zh && (
                                        <div style={{ 
                                            fontSize: '12px', 
                                            color: 'rgba(255,255,255,0.7)',
                                            lineHeight: '1.4'
                                        }}>
                                            {choice.description_zh}
                                        </div>
                                    )}
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        );
    };

    return (
        <div 
            ref={containerRef}
            onClick={(e) => {
                // 点击背景关闭（但拖动/调整大小时不关闭）
                if (!isDraggingRef.current && !resizeModeRef.current && !wasResizingRef.current &&
                    (e.target === e.currentTarget || (e.target as HTMLElement).classList.contains('detail-popup-container'))) {
                    invoke("hide_detail_popup");
                }
            }}
            onContextMenu={(e) => {
                e.preventDefault();
                // 右键点击背景也能关闭
                if (!isDraggingRef.current && !resizeModeRef.current && !wasResizingRef.current &&
                    (e.target === e.currentTarget || (e.target as HTMLElement).classList.contains('detail-popup-container'))) {
                    invoke("hide_detail_popup");
                }
            }}
            style={{
                width: '100vw',
                height: '100vh',
                display: 'flex',
                flexDirection: 'column',
                pointerEvents: 'auto',
                userSelect: 'none',
                overflow: 'hidden',
                background: 'linear-gradient(135deg, rgba(20, 18, 15, 0.98) 0%, rgba(30, 25, 20, 0.98) 100%)',
                border: '1px solid rgba(255, 205, 25, 0.3)',
                borderRadius: '8px',
                boxShadow: '0 8px 32px rgba(0, 0, 0, 0.8), 0 0 20px rgba(255, 205, 25, 0.2)',
                opacity: scale,
                transition: 'opacity 0.2s ease-out',
                position: 'relative'
            }}
        >
            {/* 拖动条 */}
            <div
                data-tauri-drag-region
                onMouseDown={() => {
                    setIsDragging(true);
                    isDraggingRef.current = true;
                }}
                onClick={(e) => e.stopPropagation()} 
                style={{
                    flexShrink: 0,
                    height: '20px',
                    cursor: 'move',
                    background: 'linear-gradient(to bottom, rgba(255, 205, 25, 0.25), rgba(255, 205, 25, 0.1))',
                    borderBottom: '1px solid rgba(255, 205, 25, 0.4)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    fontSize: '12px',
                    color: 'rgba(255, 205, 25, 0.9)',
                    letterSpacing: '3px',
                    fontWeight: 'bold',
                    textShadow: '0 1px 3px rgba(0, 0, 0, 0.8)'
                }}
            >
                ⋮⋮⋮
            </div>

            {/* Old Resize Handles for Window Size - Removed */}
            
            {/* 内容区域 */}
            <div
                className="custom-scrollbar"
                style={{
                    flex: 1,
                    overflow: 'auto',
                    padding: '10px'
                }}
            >
             <div style={{ zoom: contentScale, transformOrigin: 'top left' }}>
                {data.type === 'item' && renderItemContent(data.data as ItemData)}
                {data.type === 'monster' && renderMonsterContent(data.data as MonsterData)}
                {data.type === 'event' && renderEventContent(data.data as EventData)}
             </div>
            </div>

            {/* Manual Resize Handles using JS Logic */}
            {/* Edges */}
            {/* Native Resize Handles - Fixed zIndex and propagation */}
            <div 
                title="Resize Top" 
                onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); getCurrentWindow().startResizeDragging('North' as any); }} 
                style={{ position: 'absolute', top: 0, left: 15, right: 15, height: '10px', cursor: 'n-resize', zIndex: 9999, background: 'rgba(0,0,0,0)' }} 
            />
            <div 
                title="Resize Bottom" 
                onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); getCurrentWindow().startResizeDragging('South' as any); }} 
                style={{ position: 'absolute', bottom: 0, left: 15, right: 30, height: '10px', cursor: 's-resize', zIndex: 9999, background: 'rgba(0,0,0,0)' }} 
            />
            <div 
                title="Resize Left" 
                onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); getCurrentWindow().startResizeDragging('West' as any); }} 
                style={{ position: 'absolute', top: 15, bottom: 15, left: 0, width: '10px', cursor: 'w-resize', zIndex: 9999, background: 'rgba(0,0,0,0)' }} 
            />
            <div 
                title="Resize Right" 
                onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); getCurrentWindow().startResizeDragging('East' as any); }} 
                style={{ position: 'absolute', top: 15, bottom: 30, right: 0, width: '10px', cursor: 'e-resize', zIndex: 9999, background: 'rgba(0,0,0,0)' }} 
            />
            
            {/* Corners */}
            <div 
                title="Resize TopLeft" 
                onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); getCurrentWindow().startResizeDragging('NorthWest' as any); }} 
                style={{ position: 'absolute', top: 0, left: 0, width: '15px', height: '15px', cursor: 'nwse-resize', zIndex: 10000, background: 'rgba(0,0,0,0)' }} 
            />
            <div 
                title="Resize TopRight" 
                onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); getCurrentWindow().startResizeDragging('NorthEast' as any); }} 
                style={{ position: 'absolute', top: 0, right: 0, width: '15px', height: '15px', cursor: 'nesw-resize', zIndex: 10000, background: 'rgba(0,0,0,0)' }} 
            />
            <div 
                title="Resize BottomLeft" 
                onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); getCurrentWindow().startResizeDragging('SouthWest' as any); }} 
                style={{ position: 'absolute', bottom: 0, left: 0, width: '15px', height: '15px', cursor: 'nesw-resize', zIndex: 10000, background: 'rgba(0,0,0,0)' }} 
            />
            
            {/* Zoom Handle (Bottom Right) */}
            <div
                onMouseDown={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setResizeMode('zoom');
                    resizeModeRef.current = 'zoom';
                    resizeStartRef.current = {
                        x: e.clientX,
                        y: e.clientY,
                        width: contentScale, // Use width to store start scale
                        height: 0
                    };
                }}
                onClick={(e) => e.stopPropagation()}
                title="Drag up/down to Zoom"
                style={{
                    position: 'absolute',
                    bottom: 0,
                    right: 0,
                    width: '24px',
                    height: '24px',
                    cursor: 'nwse-resize', 
                    background: 'linear-gradient(135deg, transparent 0%, transparent 40%, #ffcc00 100%)',
                    borderRadius: '0 0 8px 0',
                    zIndex: 1000,
                    display: 'flex',
                    alignItems: 'flex-end',
                    justifyContent: 'flex-end',
                    paddingRight: '3px',
                    paddingBottom: '3px'
                }}
            >
                <div style={{ transform: 'rotate(-45deg)', fontSize: '10px' }}>🔍</div>
            </div>
        </div>
    );
}
