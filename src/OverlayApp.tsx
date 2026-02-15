import * as React from "react";
import { useEffect, useState, useRef, Fragment } from "react";
import { listen, emit } from "@tauri-apps/api/event";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import "./App.css";

interface SkillText {
    en: string;
    cn: string;
}

interface SkillDbData {
    id: string;
    name_en: string;
    name_cn: string;
    description_en: string;
    description_cn: string;
    size?: string;
    starting_tier?: string;
    available_tiers?: string;
    heroes?: string;
    tags?: string;
    hidden_tags?: string;
    descriptions: SkillText[];
}

interface ItemData {
    uuid: string;
    instance_id?: string;
    name: string;
    name_cn: string;
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
    enchantments: string[];
    description: string;
    image: string;
    displayImg?: string;
    id?: string;
    name_en?: string;
    starting_tier?: string;
    current_tier?: string;
    tiers?: Record<string, TierInfo | null>;
}

interface TierInfo {
    description: string[];
    extra_description: string[];
    cd: string | null;
}

interface MonsterSubItem {
    id?: string;
    name: string;
    name_cn?: string;
    name_en?: string;
    tier?: string;
    current_tier?: string;
    starting_tier?: string;
    tags?: string[] | string;
    tiers: Record<string, TierInfo | null>;
    image: string;
    damage?: number;
    heal?: number;
    shield?: number;
    burn?: number;
    poison?: number;
    regen?: number;
    lifesteal?: number;
    ammo?: number;
    multicast?: number;
    displayImg?: string;
    cardFrameImg?: string;
    size?: string;
    damage_tiers?: string;
    heal_tiers?: string;
    shield_tiers?: string;
    ammo_tiers?: string;
    burn_tiers?: string;
    poison_tiers?: string;
    regen_tiers?: string;
    lifesteal_tiers?: string;
    multicast_tiers?: string;
    cooldown?: number;
    cooldown_tiers?: string;
    skills?: SkillText[];
}

interface MonsterData {
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

interface EventChoice {
    name?: string;              // 英文名
    name_zh?: string;           // 中文名
    icon?: string;              // 图标文件名
    icon_url?: string;          // 图标URL
    icon_path?: string;         // 本地图标路径
    url?: string;
    description?: string;       // 英文描述
    description_zh?: string;    // 中文描述
    text_key?: string;
    display_text?: string;
    displayIcon?: string;       // 加载后的本地图片路径
}

interface EventData {
    Id: string;
    InternalName?: string;
    name?: string;          // 中文名称
    name_en?: string;       // 英文名称
    url?: string;
    choices?: EventChoice[];
    image_paths?: {
        char?: string;
        bg?: string;
    };
    displayImgChar?: string;
    displayImgBg?: string;
}

const ENCHANT_COLORS: Record<string, string> = {
    "黄金": "var(--c-gold)",
    "沉重": "var(--c-slow)",
    "寒冰": "var(--c-freeze)",
    "疾速": "var(--c-haste)",
    "护盾": "var(--c-shield)",
    "回复": "var(--c-heal)",
    "毒素": "var(--c-poison)",
    "炽焰": "var(--c-burn)",
    "闪亮": "#98a8fe",
    "致命": "var(--c-damage)",
    "辉耀": "#98a8fe",
    "黑曜石": "#9d4a6f"
};

const TIER_COLORS = ["#cd7f32", "#c0c0c0", "#ffd700", "#b9f2ff", "#e298ff"];

const KEYWORD_COLORS: Record<string, string> = {
    "弹药": "#ff8e00", "灼烧": "#ff9f45", "充能": "#00ecc3", "冷却": "#00ecc3",
    "加速": "#00ecc3", "暴击率": "#f5503d", "伤害": "#f5503d", "飞行": "#f4cf20",
    "冻结": "#00ccff", "金币": "#ffd700", "治疗": "#8eea31", "生命值": "#8eea31",
    "最大生命值": "#8eea31", "收入": "#ffcd19", "吸血": "#9d4a6f", "剧毒": "#0ebe4f",
    "生命再生": "#8eea31", "护盾": "#f4cf20", "减速": "#cb9f6e", "价值": "#ffcd19"
};

// 获取卡牌背景框图片路径
const getCardFramePath = (tier: string, size: string): string => {
    // 标准化等级名称
    const tierMap: Record<string, string> = {
        'bronze': 'Bronze',
        'silver': 'Silver',
        'gold': 'Gold',
        'diamond': 'Diamond',
        'legendary': 'Legendary'
    };
    const normalizedTier = tierMap[tier.toLowerCase()] || 'Bronze';
    
    // 标准化尺寸：Small->S, Medium->M, Large->L
    const sizeMap: Record<string, string> = {
        'small': 'S',
        'medium': 'M',
        'large': 'L'
    };
    const normalizedSize = sizeMap[size.toLowerCase()] || 'M';
    
    return `assets/gui/CardFrame_${normalizedTier}_${normalizedSize}_TUI.webp`;
};

// 提取到外部的组件，避免因为父组件重绘导致被重新创建而闪烁
const CardBackground = React.memo(({ framePath, getImg }: { framePath: string, getImg: (path: string) => Promise<string> }) => {
    const [frameUrl, setFrameUrl] = useState('');
    
    useEffect(() => {
        let isMounted = true;
        getImg(framePath).then(url => {
            if (isMounted && url) setFrameUrl(url);
        });
        return () => { isMounted = false; };
    }, [framePath, getImg]); // getImg should be stable (useCallback or outside definition)
    
    return (
        <div style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: '100%',
            backgroundImage: frameUrl ? `url(${frameUrl})` : 'none',
            backgroundSize: '100% 100%',
            backgroundPosition: 'center',
            backgroundRepeat: 'no-repeat',
            pointerEvents: 'none',
            zIndex: 2,
            transform: 'translateZ(0)',
            backfaceVisibility: 'hidden'
        }} />
    );
});

export default function OverlayApp() {
    const [yoloResult, setYoloResult] = useState<{type: 'item' | 'monster' | 'event', data: ItemData | MonsterData | EventData} | null>(null);
    const [, setIdentifying] = useState(false);
    const [showYoloMonitor, setShowYoloMonitor] = useState(() => {
        const saved = localStorage.getItem("show-yolo-monitor");
        if (saved === null) {
            localStorage.setItem("show-yolo-monitor", "true");
            return true;
        }
        return saved === "true";
    });
    
    // 打印DPI信息用于调试
    useEffect(() => {
        const dpi = window.devicePixelRatio || 1;
        const screenW = window.screen.width;
        const screenH = window.screen.height;
        console.log(`[DPI Info] devicePixelRatio=${dpi}, screen=${screenW}x${screenH}, inner=${window.innerWidth}x${window.innerHeight}`);
    }, []);

    // 从后端读取持久化的 show-yolo-monitor 设置，确保不同窗口之间一致
    useEffect(() => {
        try {
            invoke('get_show_yolo_monitor').then((res) => {
                const val = !!(res as boolean);
                try {
                    localStorage.setItem('show-yolo-monitor', val.toString());
                } catch {
                    // ignore
                }
                setShowYoloMonitor(val);
                console.log('[Overlay] Synced show-yolo-monitor from backend:', val);
            }).catch(() => {
                // ignore
            });
        } catch {
            // ignore
        }
    }, []);
    const containerRef = useRef<HTMLDivElement>(null);
    const yoloMonitorRef = useRef<HTMLDivElement>(null);
    const yoloResultRef = useRef(yoloResult);
    const imgCache = useRef<Map<string, string>>(new Map());
    const MAX_CACHE_SIZE = 200; // 限制缓存最大200张图片

    // Load items database for merging monster item details
    const [itemsDb, setItemsDb] = useState<Map<string, ItemData>>(new Map());
    const itemsDbRef = useRef<Map<string, ItemData>>(new Map());
    const [skillsDb, setSkillsDb] = useState<Map<string, SkillDbData>>(new Map());
    const skillsDbRef = useRef<Map<string, SkillDbData>>(new Map());
    
    useEffect(() => {
        itemsDbRef.current = itemsDb;
    }, [itemsDb]);
    
    useEffect(() => {
        skillsDbRef.current = skillsDb;
    }, [skillsDb]);

    useEffect(() => {
        (async () => {
            try {
                const path = await resolveResource('resources/items_db.json');
                const response = await fetch(convertFileSrc(path));
                const items: ItemData[] = await response.json();
                const map = new Map<string, ItemData>();
                items.forEach(item => {
                    if (item.id) map.set(item.id, item);
                    if (item.name_cn) map.set(item.name_cn, item);
                    if (item.name_en) map.set(item.name_en, item);
                });
                setItemsDb(map);
                console.log('[Overlay] Loaded items_db:', map.size, 'items');
            } catch (e) {
                console.error('[Overlay] Failed to load items_db:', e);
            }
        })();
    }, []);
    
    // Load skills database
    useEffect(() => {
        (async () => {
            try {
                const path = await resolveResource('resources/skills_db.json');
                const response = await fetch(convertFileSrc(path));
                const skills: SkillDbData[] = await response.json();
                const map = new Map<string, SkillDbData>();
                skills.forEach(skill => {
                    if (skill.id) map.set(skill.id, skill);
                    if (skill.name_cn) map.set(skill.name_cn, skill);
                    if (skill.name_en) map.set(skill.name_en, skill);
                });
                setSkillsDb(map);
                console.log('[Overlay] Loaded skills_db:', map.size, 'skills');
            } catch (e) {
                console.error('[Overlay] Failed to load skills_db:', e);
            }
        })();
    }, []);

    // 移除了拖动功能，使用设置中的位置控制
    const [isResizing, setIsResizing] = useState(false);
    const isResizingRef = useRef(false);
    
    // Overlay详情页面位置、缩放和尺寸设置
    const [detailPosition, setDetailPosition] = useState(() => {
        const x = localStorage.getItem('overlay-detail-x');
        const y = localStorage.getItem('overlay-detail-y');
        const scale = localStorage.getItem('overlay-detail-scale');
        const width = localStorage.getItem('overlay-detail-width');
        const height = localStorage.getItem('overlay-detail-height');
        return {
            x: x ? parseInt(x) : 50,
            y: y ? parseInt(y) : 50,
            scale: scale ? parseInt(scale) : 100,
            width: width ? parseInt(width) : 420,
            height: height ? parseInt(height) : 600
        };
    });

    useEffect(() => { isResizingRef.current = isResizing; }, [isResizing]);

    // 主动同步机制：
    // 每当 Overlay 显示结果时，请求 App 主窗口再次广播一次位置配置
    // 确保即使 localStorage 同步失败，也能通过事件总线获取到正确的配置
    useEffect(() => {
        if (yoloResult) {
             console.log("[Overlay] Requesting config sync from App...");
             emit("request-sync-overlay-settings", {}).catch(console.error);
        }
    }, [yoloResult]);

    // 当详情卡片首次显示时，强制从localStorage重新读取位置
    // 以防止因为App.tsx状态未同步导致的错位
    useEffect(() => {
        if (yoloResult) {
            const x = localStorage.getItem('overlay-detail-x');
            const y = localStorage.getItem('overlay-detail-y');
            const scale = localStorage.getItem('overlay-detail-scale');
            const width = localStorage.getItem('overlay-detail-width');
            const height = localStorage.getItem('overlay-detail-height');
            
            console.log("[Overlay] Opened, forcing position reload from storage:", { x, y, scale, width, height });
            
            if (x || y || scale || width || height) {
                setDetailPosition(prev => ({
                    x: x ? parseInt(x) : prev.x,
                    y: y ? parseInt(y) : prev.y,
                    scale: scale ? parseInt(scale) : prev.scale,
                    width: width ? parseInt(width) : prev.width,
                    height: height ? parseInt(height) : prev.height
                }));
            }
        }
    }, [yoloResult]);
    
    // Debug: 检查YOLO Monitor ref
    useEffect(() => {
        console.log('[DEBUG] showYoloMonitor:', showYoloMonitor, 'yoloMonitorRef.current:', yoloMonitorRef.current);
        if (yoloMonitorRef.current) {
            const rect = yoloMonitorRef.current.getBoundingClientRect();
            console.log('[DEBUG] YOLO Monitor rect:', { x: rect.left, y: rect.top, w: rect.width, h: rect.height });
        }
    }, [showYoloMonitor]);
    
    const [realtimeList, setRealtimeList] = useState<string[]>([]);
    const [isPolling, setIsPolling] = useState(false);
    const [hoveredMonsterItem, setHoveredMonsterItem] = useState<MonsterSubItem | null>(null);
    const [expandedMonsterItem, setExpandedMonsterItem] = useState<MonsterSubItem | null>(null);
    const [yoloStats, setYoloStats] = useState<{total: number, items: number, events: number, monsters: number, skills: number} | null>(null);
    
    const winMousePos = useRef({ x: 0, y: 0 });
    useEffect(() => {
        const track = (e: MouseEvent) => { winMousePos.current = { x: e.clientX, y: e.clientY }; };
        window.addEventListener('mousemove', track);
        return () => window.removeEventListener('mousemove', track);
    }, []);

    useEffect(() => {
        // const startUnlisten = listen("yolo-scan-start", () => setIdentifying(true));
        // const endUnlisten = listen("yolo-scan-end", () => setIdentifying(false));
        const startUnlisten = Promise.resolve(() => {});
        const endUnlisten = Promise.resolve(() => {});
        
        // 监听主窗口关闭事件
        listen("main-window-closing", () => {
            console.log('[Overlay] Main window closing, hiding overlay');
            setYoloResult(null);
            setShowYoloMonitor(false);
        });
        
        // 监听来自App的YOLO统计更新
        const statsUnlisten = listen("yolo-stats-updated", (event: any) => {
            // 只在Stats Monitor显示时才更新状态，避免不必要的全局重绘（如果Stats没在主UI显示）
            if (showYoloMonitor) {
                // 做一个简单的比较，防止频繁更新相同的数据
                setYoloStats(prev => {
                   if (JSON.stringify(prev) === JSON.stringify(event.payload)) return prev;
                   return event.payload;
                });
                
                // 仅当Polling指示灯需要闪烁时才设置
                // setIsPolling(true);
                // setTimeout(() => setIsPolling(false), 800);
            }
        });
        
        // 监听详情页面位置更新
        const positionUnlisten = listen("update-overlay-detail-position", (event: any) => {
            console.log("[Overlay] Received position update and saving to storage:", event.payload);
            const { x, y, scale, width, height } = event.payload;
            
            // Persist to localStorage so it survives reload
            if (x !== undefined) localStorage.setItem('overlay-detail-x', x.toString());
            if (y !== undefined) localStorage.setItem('overlay-detail-y', y.toString());
            if (scale !== undefined) localStorage.setItem('overlay-detail-scale', scale.toString());
            if (width !== undefined) localStorage.setItem('overlay-detail-width', width.toString());
            if (height !== undefined) localStorage.setItem('overlay-detail-height', height.toString());

            setDetailPosition(prev => ({ 
                x: x ?? prev.x, 
                y: y ?? prev.y, 
                scale: scale ?? prev.scale,
                width: width ?? prev.width,
                height: height ?? prev.height
            }));
        });

        return () => {
            startUnlisten.then(u => u());
            endUnlisten.then(u => u());
            statsUnlisten.then(u => u());
            positionUnlisten.then(u => u());
            // 清理图片缓存
            imgCache.current.clear();
        };
    }, []);

    // Listen for backend updates (triggered by log events or manual scans)
    useEffect(() => {
        const unlistenPromise = listen<string[]>("overlay-update-monsters", async (event) => {
            const results = event.payload;
            setIsPolling(true);
            if (results && results.length > 0) {
                setRealtimeList(results.slice(0, 5));
            } else {
                setRealtimeList([]);
            }
            
            // 获取YOLO统计信息
            try {
                const stats = await invoke('get_yolo_stats');
                setYoloStats(stats as any);
            } catch (e) {
                console.error('Failed to get YOLO stats:', e);
            }
            
            // Visual feedback loop
            setTimeout(() => setIsPolling(false), 800);
        });
        return () => { unlistenPromise.then(u => u()); };
    }, []);


    useEffect(() => {
        const move = (_e: MouseEvent) => {
             if (isResizingRef.current) {
                // Resizing is now controlled via detailPosition state from backend
            }
        };
        const stop = () => {
            if (isResizingRef.current) {
                setIsResizing(false);
                // After drag/resize stop, check mouse position to decide ignore state
                if (containerRef.current) {
                    const rect = containerRef.current.getBoundingClientRect();
                    const isOutside = (winMousePos.current.x < rect.left || winMousePos.current.x > rect.right ||
                                      winMousePos.current.y < rect.top || winMousePos.current.y > rect.bottom);
                    if (isOutside) {
                        invoke("set_overlay_ignore_cursor", { ignore: true }).catch(() => {});
                        invoke("restore_game_focus").catch(() => {});
                    }
                }
            }
        };
        window.addEventListener('mousemove', move);
        window.addEventListener('mouseup', stop);
        return () => {
            window.removeEventListener('mousemove', move);
            window.removeEventListener('mouseup', stop);
        };
    }, []);

    useEffect(() => {
        invoke("set_overlay_ignore_cursor", { ignore: true }).catch(console.error);
    }, []);

    useEffect(() => { yoloResultRef.current = yoloResult; }, [yoloResult]);

    // 监听 localStorage 变化以实时更新 showYoloMonitor
    useEffect(() => {
        const handleStorageChange = () => {
            const saved = localStorage.getItem("show-yolo-monitor");
            const shouldShow = saved === "true";
            console.log("[Overlay] Storage check - raw value:", saved, "shouldShow:", shouldShow, "current state:", showYoloMonitor);
            if (shouldShow !== showYoloMonitor) {
                console.log("[Overlay] Updating showYoloMonitor to:", shouldShow);
                setShowYoloMonitor(shouldShow);
            }
        };

        // Also listen to backend-emitted event for cross-window updates
        let unlistenPromise: Promise<() => void> | null = null;
        try {
            unlistenPromise = listen<boolean>("set-show-yolo-monitor", (event) => {
                console.log("[Overlay] Received set-show-yolo-monitor event:", event.payload);
                const shouldShow = !!event.payload;
                // Keep this window's localStorage in sync so the polling/storage-check won't revert the value
                try {
                    localStorage.setItem("show-yolo-monitor", shouldShow.toString());
                } catch (e) {
                    console.error('[Overlay] Failed to write localStorage for show-yolo-monitor:', e);
                }
                setShowYoloMonitor(shouldShow);
            });
        } catch (e) {
            console.error("[Overlay] Failed to attach backend listener:", e);
        }

        window.addEventListener('storage', handleStorageChange);
        // 定期检查（因为同窗口的localStorage变化不触发storage事件）
        const interval = setInterval(handleStorageChange, 500);
        console.log("[Overlay] Storage listener initialized");
        return () => {
            window.removeEventListener('storage', handleStorageChange);
            clearInterval(interval);
            if (unlistenPromise) {
                unlistenPromise.then(u => u && u());
            }
        };
    }, [showYoloMonitor]);

    const getImg = async (path: string | null | undefined) => {
        if (!path) return "";
        if (imgCache.current.has(path)) {
            // LRU: 将访问的项移到最后
            const value = imgCache.current.get(path)!;
            imgCache.current.delete(path);
            imgCache.current.set(path, value);
            return value;
        }
        try {
            const fullPath = await resolveResource(`resources/${path}`);
            const assetUrl = convertFileSrc(fullPath);
            
            // 检查缓存大小，如果超过限制则删除最老的项
            if (imgCache.current.size >= MAX_CACHE_SIZE) {
                const firstKey = imgCache.current.keys().next().value;
                if (firstKey) {
                    imgCache.current.delete(firstKey);
                }
            }
            
            imgCache.current.set(path, assetUrl);
            return assetUrl;
        } catch { return ""; }
    };

        // Load skills_db.json mapping (id -> art_key basename)
        const [skillsArtMap, setSkillsArtMap] = useState<Record<string, string>>({});
        useEffect(() => {
            (async () => {
                try {
                    const resPath = await resolveResource('resources/skills_db.json');
                    const url = convertFileSrc(resPath);
                    const resp = await fetch(url);
                    const data = await resp.json();
                    const map: Record<string, string> = {};
                    for (const entry of data) {
                        if (entry.id && entry.art_key) {
                            const basename = entry.art_key.split('/').pop();
                            map[entry.id] = basename;
                        }
                    }
                    setSkillsArtMap(map);
                } catch (e) { console.warn('Overlay: Failed to load skills_db.json', e); }
            })();
        }, []);

    const processMonsterImages = async (m: MonsterData) => {
        let filename = m.image ? m.image.split('/').pop() || `${m.name_zh}.webp` : `${m.name_zh}.webp`;
        let displayImg = await getImg(`assets/monsters/characters/${filename}`);
        if (!displayImg && m.name_zh.includes(' ')) {
            const spacePos = m.name_zh.lastIndexOf(' ');
            const baseName = m.name_zh.substring(spacePos + 1);
            const fallbackFilename = `${baseName}.webp`;
            const fallbackImg = await getImg(`assets/monsters/characters/${fallbackFilename}`);
            if (fallbackImg) { displayImg = fallbackImg; filename = fallbackFilename; }
        }
        let bgFilename = filename;
        if (m.name_zh === '绿洲守护神') bgFilename = '绿洲守护神_Day9.webp';
        const displayImgBg = await getImg(`assets/monsters/backgrounds/${bgFilename}`);

        return {
            ...m,
            displayImg,
            displayImgBg,
            skills: m.skills ? await Promise.all(m.skills.map(async s => {
                let imgPath = '';
                const id = s.id || s.name;
                
                // 从 skills_db 获取完整技能信息
                const fullSkillInfo = id ? skillsDbRef.current.get(id) : null;
                
                // 合并技能信息，将 descriptions 字段映射到 skills
                let mergedSkill = s;
                if (fullSkillInfo) {
                    mergedSkill = {
                        ...s,
                        // 使用 skills_db 中的 descriptions 作为 skills 字段
                        skills: fullSkillInfo.descriptions || s.skills || [],
                        name: fullSkillInfo.name_cn || s.name,
                        name_cn: fullSkillInfo.name_cn,
                        name_en: fullSkillInfo.name_en,
                        size: fullSkillInfo.size || s.size,
                        starting_tier: fullSkillInfo.starting_tier || s.starting_tier
                    };
                }
                
                // 获取技能图标
                try {
                    const art = id ? skillsArtMap[id] : undefined;
                    if (art) {
                        const base = art.split('/').pop() || art;
                        const nameNoExt = base.replace(/\.[^/.]+$/, '');
                        imgPath = `images/skill/${nameNoExt}.webp`;
                    } else {
                        imgPath = `images/${id || s.name}.webp`;
                    }
                } catch (e) { imgPath = `images/${id || s.name}.webp`; }
                
                return { ...mergedSkill, displayImg: await getImg(imgPath) };
            })) : [],
            items: m.items ? await Promise.all(m.items.map(async i => {
                const id = i.id || i.name;
                // Merge detailed info from items_db if available
                let fullItemInfo = null;
                if (id) {
                    fullItemInfo = itemsDbRef.current.get(id);
                    // Try case-insensitive lookup if direct match fails
                    if (!fullItemInfo) {
                        const lowerId = id.toLowerCase();
                        for (const [key, val] of itemsDbRef.current.entries()) {
                            if (key.toLowerCase() === lowerId) {
                                fullItemInfo = val;
                                break;
                            }
                        }
                    }
                }
                
                // If not found by ID, try name
                if (!fullItemInfo && i.name) {
                    fullItemInfo = itemsDbRef.current.get(i.name);
                }
                
                // Merge: prefer item info from DB, but keep existing runtime info if needed. 
                const merged = fullItemInfo ? 
                    { 
                        ...i,  // 先用 monsters_db 的基础信息
                        ...fullItemInfo, // 覆盖为 items_db 的完整信息
                        // 但保留 monsters_db 中的运行时字段
                        current_tier: i.current_tier || fullItemInfo.current_tier,
                        tier: i.tier || fullItemInfo.tier,
                        // 确保 skills 来自 items_db，合并 skills 和 skills_passive
                        skills: [
                            ...(fullItemInfo.skills || []),
                            ...(fullItemInfo.skills_passive || [])
                        ]
                    } : i;

                // Determine Tier & Color Logic
                // Priority: current_tier > tier (from DB) > bronze
                let currentTier = (merged.current_tier || merged.tier || 'bronze').toLowerCase();
                // Handle complex tier strings like "Gold / 黄金" -> "gold"
                currentTier = currentTier.split(' / ')[0].trim().toLowerCase();
                
                // Determine Size
                const sizeStr = (merged.size || 'Medium / 中型').split(' / ')[0].toLowerCase().trim();
                
                // Calculate Card Frame Path
                const cardFramePath = getCardFramePath(currentTier, sizeStr);
                const cardFrameImg = await getImg(cardFramePath);
                
                // Debug log
                // console.log(`[MonsterItem] ${merged.name}: tier=${currentTier}, size=${sizeStr} -> frame=${cardFramePath} (${cardFrameImg ? 'OK' : 'FAIL'})`);

                return { 
                    ...merged, 
                    displayImg: await getImg(`images/${merged.id || merged.name}.webp`),
                    cardFrameImg,
                    // Ensure merged has correct tier/size for rendering usage
                    current_tier: currentTier,
                    size: sizeStr
                };
            })) : []
        };
    };

    const processEventImages = async (e: EventData) => {
        // 加载事件图片
        let displayImgChar = '';
        let displayImgBg = '';
        
        if (e.image_paths?.char) {
            displayImgChar = await getImg(e.image_paths.char);
        }
        if (e.image_paths?.bg) {
            displayImgBg = await getImg(e.image_paths.bg);
        }

        // 加载choices的图标
        const processedChoices = e.choices ? await Promise.all(e.choices.map(async choice => {
            const iconImg = choice.icon_path ? await getImg(choice.icon_path) : '';
            return {
                ...choice,
                displayIcon: iconImg
            };
        })) : [];

        return {
            ...e,
            displayImgChar,
            displayImgBg,
            choices: processedChoices as any
        };
    };

    // Removed duplicate trigger_yolo_scan listener - now handled in App.tsx with enable-yolo-auto check

    useEffect(() => {
        const unlistenPromise = listen<{x: number, y: number}>("global-right-click", async (event) => {
            const coords = event.payload;
            // 优化流程：无论当前是否已显示卡牌，只要右键点击了有效区域，就直接尝试识别并切换
            // 只有当右键点击了"空白"区域且当前有卡牌显示时，才关闭
            try {
                // 停止正在进行的 YOLO 扫描，并立马响应右键点击
                await invoke("abort_yolo_scan").catch(() => {});
                
                const res = await invoke<{type: 'item' | 'monster' | 'event', data: any} | null>("handle_overlay_right_click", { 
                    x: Math.round(coords.x), 
                    y: Math.round(coords.y) 
                });
                
                if (res) {
                    // 命中了新的目标：直接更新内容/切换显示
                    if (res.type === 'item') {
                        const item = res.data as ItemData;
                        item.displayImg = await getImg(`images/${item.uuid || item.name}.webp`);
                        setYoloResult({ type: 'item', data: item });
                        // Update draggable position to mouse position if it's a fresh open (optional, but keep consistent with user intent)
                        // If user wants manual drag, we shouldn't force move it unless it was closed.
                        // But since we are switching, keeping position is better.
                    } else if (res.type === 'monster') {
                        const monster = await processMonsterImages(res.data as MonsterData);
                        setYoloResult({ type: 'monster', data: monster });
                        // 发送事件到主窗口更新野怪匹配状态
                        try {
                            await invoke('emit_to_main', { 
                                event: 'monster-matched', 
                                payload: { name: monster.name, name_zh: monster.name_zh } 
                            });
                        } catch (e) {
                            console.error('Failed to emit monster-matched event:', e);
                        }
                    } else if (res.type === 'event') {
                        const eventData = await processEventImages(res.data as EventData);
                        setYoloResult({ type: 'event', data: eventData });
                    }
                    // 注意：这里不重置 pos，保留用户上次拖拽的位置（或者保持默认相对位置）
                    // 也不修改 ignore_cursor，用户可能正在操作
                } else {
                    // 未命中任何目标
                    // 如果当前有显示的卡牌，则认为是用户的"关闭"意图
                    if (yoloResultRef.current) {
                        setYoloResult(null);
                        // Hide offscreen if needed, but draggablePos controls it now
                        invoke("set_overlay_ignore_cursor", { ignore: true }).catch(() => {});
                    }
                }
            } catch (err) {
                console.error("Right-click handle error:", err);
            }
        });
        return () => { unlistenPromise.then(u => u()); };
    }, []);

    // Reporting bounds for "Auto-Sensing Door"
    useEffect(() => {
        const timer = setInterval(() => {
            const boundsArray = [];
            const scale = window.devicePixelRatio || 1;
            
            // 主卡片区域
            if (containerRef.current && yoloResult) {
                const rect = containerRef.current.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) {
                    // 应用DPI缩放修正
                    boundsArray.push({ 
                        x: Math.round(rect.left * scale), 
                        y: Math.round(rect.top * scale), 
                        w: Math.round(rect.width * scale), 
                        h: Math.round(rect.height * scale) 
                    });
                }
            }
            
            // YOLO Monitor区域
            if (yoloMonitorRef.current && showYoloMonitor) {
                const rect = yoloMonitorRef.current.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) {
                    // 应用DPI缩放修正
                    boundsArray.push({ 
                        x: Math.round(rect.left * scale), 
                        y: Math.round(rect.top * scale), 
                        w: Math.round(rect.width * scale), 
                        h: Math.round(rect.height * scale) 
                    });
                }
            }
            
            invoke("update_overlay_bounds", { bounds: boundsArray });
        }, 200);
        return () => clearInterval(timer);
    }, [yoloResult, showYoloMonitor]);

    const handleMouseEnter = () => {
        invoke("set_overlay_ignore_cursor", { ignore: false }).catch(console.error);
    };

    const handleMouseLeave = () => {
        if (!isResizingRef.current) {
            invoke("set_overlay_ignore_cursor", { ignore: true }).catch(console.error);
            invoke("restore_game_focus").catch(console.error);
        }
    };

    const renderTextContent = (text: string) => {
        if (!text) return null;
        
        // 处理 3/6/9/12 序列
        const parts = text.split(/(\d+(?:\/\d+)+)/g);
        
        return parts.map((part, i) => {
            if (part.includes('/')) {
                const nums = part.split('/');
                return (
                    <span key={i} className="progression-nums" style={{ display: 'inline-flex', gap: '2px', alignItems: 'center' }}>
                        {nums.map((n, idx) => {
                            let colorIdx = idx;
                            if (nums.length === 2) colorIdx = idx + 2;
                            else if (nums.length === 3) colorIdx = idx + 1;
                            
                            return (
                                <Fragment key={idx}>
                                    <span style={{ 
                                        color: TIER_COLORS[colorIdx] || '#fff', 
                                        fontWeight: 'bold',
                                        textShadow: '0 0 4px rgba(0,0,0,0.5)'
                                    }}>{n}</span>
                                    {idx < nums.length - 1 && <span style={{ color: '#fff', opacity: 0.3 }}>/</span>}
                                </Fragment>
                            );
                        })}
                    </span>
                );
            }

            // 处理关键词变色
            let resultParts: any[] = [part];
            Object.entries(KEYWORD_COLORS).forEach(([keyword, color]) => {
                const newResultParts: any[] = [];
                resultParts.forEach(segment => {
                    if (typeof segment !== 'string') {
                        newResultParts.push(segment);
                    } else {
                        const subSegments = segment.split(new RegExp(`(${keyword})`, 'g'));
                        subSegments.forEach((sub, subIdx) => {
                            if (sub === keyword) {
                                newResultParts.push(<span key={`${keyword}-${subIdx}`} style={{ color, fontWeight: 'bold' }}>{sub}</span>);
                            } else if (sub) {
                                newResultParts.push(sub);
                            }
                        });
                    }
                });
                resultParts = newResultParts;
            });

            return <Fragment key={i}>{resultParts}</Fragment>;
        });
    };

    const formatDescription = (text: string) => {
        if (!text) return null;
        const parts = text.split(/(\[Locked\]|Quest:)/g);
        return parts.map((part, i) => {
            if (part === "[Locked]") return <span key={i} className="icon-locked" title="Locked" style={{ color: '#f5503d', fontSize: '1.1em' }}>🔒</span>;
            if (part === "Quest:") return <span key={i} className="icon-quest" title="Quest" style={{ color: '#ffd700', fontSize: '1.1em' }}>📜</span>;
            return renderTextContent(part);
        });
    };

    // 专门用于渲染附魔文本的函数，保留除以1000逻辑
    const formatEnchantDescription = (text: string) => {
        if (!text) return null;
        
        const parts = text.split(/(\d+(?:\/\d+)+)/g);
        
        return parts.map((part, i) => {
            if (part.includes('/')) {
                const nums = part.split('/');
                return (
                    <span key={i} className="progression-nums" style={{ display: 'inline-flex', gap: '2px', alignItems: 'center' }}>
                        {nums.map((n, idx) => {
                            let colorIdx = idx;
                            if (nums.length === 2) colorIdx = idx + 2;
                            else if (nums.length === 3) colorIdx = idx + 1;
                            
                            const val = parseFloat(n);
                            const displayVal = (!isNaN(val) && val > 100) ? (val / 1000).toFixed(1) : n;
                            
                            return (
                                <Fragment key={idx}>
                                    <span style={{ 
                                        color: TIER_COLORS[colorIdx] || '#fff', 
                                        fontWeight: 'bold',
                                        textShadow: '0 0 4px rgba(0,0,0,0.5)'
                                    }}>{displayVal}</span>
                                    {idx < nums.length - 1 && <span style={{ color: '#fff', opacity: 0.3 }}>/</span>}
                                </Fragment>
                            );
                        })}
                    </span>
                );
            }

            // 处理单个大数值 (ms -> s)
            let processedPart = part;
            processedPart = processedPart.replace(/\b(\d{3,})\b/g, (match) => {
                const val = parseInt(match, 10);
                return val > 100 ? (val / 1000).toFixed(1) : match;
            });

            // 处理关键词变色
            let resultParts: any[] = [processedPart];
            Object.entries(KEYWORD_COLORS).forEach(([keyword, color]) => {
                const newResultParts: any[] = [];
                resultParts.forEach(segment => {
                    if (typeof segment !== 'string') {
                        newResultParts.push(segment);
                    } else {
                        const subSegments = segment.split(new RegExp(`(${keyword})`, 'g'));
                        subSegments.forEach((sub, subIdx) => {
                            if (sub === keyword) {
                                newResultParts.push(<span key={`${keyword}-${subIdx}`} style={{ color, fontWeight: 'bold' }}>{sub}</span>);
                            } else if (sub) {
                                newResultParts.push(sub);
                            }
                        });
                    }
                });
                resultParts = newResultParts;
            });

            return <Fragment key={i}>{resultParts}</Fragment>;
        });
    };

    // 从tier字符串中提取当前等级的数值
    const getTierValue = (tierString: string, tier: string, startingTier?: string): number | null => {
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

    // 计算派生属性值（基于其他属性的百分比或倍数）
    const calculateDerivedStats = (descriptions: string[], baseStats: {damage?: number, burn?: number, heal?: number, shield?: number, poison?: number}) => {
        const derived: {damage?: number, burn?: number, heal?: number, shield?: number, poison?: number} = {};
        
        if (!descriptions || !Array.isArray(descriptions)) return derived;
        
        for (const desc of descriptions) {
            // "造成灼烧，等量于此物品伤害" 或 "造成灼烧，等量于此物品伤害的X%"
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
            
            // "造成剧毒，等量于此物品伤害的X%"
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
            
            // "获得护盾，等量于此物品伤害量" 或 "获得护盾，等量于此物品伤害"
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
            
            // "获得治疗，等量于此物品伤害"
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
            
            // "造成伤害，等量于此物品灼烧的X倍"
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
        
        // 吸血特殊处理：如果lifesteal=1，用吸血颜色显示伤害数值
        if (lifesteal === 1 && damage) {
            stats.push({ type: 'lifesteal', value: damage, color: '#9d4a6f' });
        } else {
            if (damage) stats.push({ type: 'damage', value: damage, color: '#f5503d' });
            if (lifesteal && lifesteal !== 1) stats.push({ type: 'lifesteal', value: lifesteal, color: '#9d4a6f' });
        }
        
        if (heal) stats.push({ type: 'heal', value: heal, color: '#8eea31' });
        if (shield) stats.push({ type: 'shield', value: shield, color: '#f4cf20' });
        if (burn) stats.push({ type: 'burn', value: burn, color: '#ff9f45' });
        if (poison) stats.push({ type: 'poison', value: poison, color: '#5DB75C' });
        if (regen) stats.push({ type: 'regen', value: regen, color: '#8eea31' });

        return stats;
    };

    const renderStatTiers = (item: ItemData) => {
        const stats: {label: string, value: string}[] = [];
        const check = (val: any, label: string) => {
            if (val && typeof val === 'string' && val.includes('/')) {
                stats.push({ label, value: val });
            }
        };

        check(item.damage_tiers, '伤害');
        check(item.heal_tiers, '治疗');
        check(item.shield_tiers, '护盾');
        if (item.cooldown_tiers && item.cooldown_tiers.includes('/')) {
            const cdSecs = item.cooldown_tiers.split('/').map(v => (parseFloat(v)/1000).toFixed(1)).join('/');
            stats.push({ label: 'CD', value: cdSecs + 's' });
        }
        check(item.ammo_tiers, '弹药');
        check(item.burn_tiers, '灼烧');
        check(item.poison_tiers, '中毒');
        check(item.multicast_tiers, '多重');
        check(item.regen_tiers, '回复');

        if (stats.length === 0) return null;

        return (
            <div style={{ marginTop: '10px', background: 'rgba(0,0,0,0.3)', padding: '10px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.05)' }}>
                <div style={{ fontSize: '11px', color: 'var(--c-golden)', marginBottom: '8px', borderBottom: '1px solid rgba(255,205,25,0.2)', paddingBottom: '4px' }}>属性成长 (Tier Progression)</div>
                {stats.map(s => (
                    <div key={s.label} style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px', marginBottom: '4px' }}>
                        <span style={{ color: '#888' }}>{s.label}</span>
                        <span style={{ color: '#ddd' }}>{renderTextContent(s.value)}</span>
                    </div>
                ))}
            </div>
        );
    };

    const renderTierInfo = (item: MonsterSubItem) => {
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
        const tierData = tiers[currentTier] || Object.values(tiers).find(t => t !== null);
        if (!tierData) return null;

        const sizeClass = (item.size || 'Medium').split(' / ')[0].toLowerCase();
        const borderColorMap: Record<string, string> = {
            bronze: "#CD7F32", silver: "#C0C0C0", gold: "#FFD700", diamond: "#B9F2FF", legendary: "#FF4500",
        };
        const borderColor = borderColorMap[currentTier] || borderColorMap.bronze;

        return (
            <div key={item.name} className={`sub-item-card tier-${currentTier}`} style={{ borderLeft: `4px solid ${borderColor}`, marginBottom: '6px' }}>
                <div className="sub-item-header">
                    <div className={`sub-item-img-wrap size-${sizeClass}`} style={{ outline: `2px solid ${borderColor}` }}>
                        <img src={item.displayImg} className="sub-item-img" />
                    </div>
                    <div className="sub-item-title-row">
                        <span className="sub-item-name">{item.name}</span>
                        {tierData.cd && <div className="sub-item-cd">⏳ {tierData.cd}s</div>}
                    </div>
                </div>
                <div className="sub-item-desc" style={{ padding: '4px 8px', fontSize: '13px', color: '#ccc' }}>
                    {tierData.description.map((d, i) => <div key={i} className="desc-line">{formatDescription(d)}</div>)}
                    {tierData.extra_description && tierData.extra_description.map((d, i) => <div key={`extra-${i}`} className="desc-line extra" style={{ color: '#aaa', fontStyle: 'italic' }}>{formatDescription(d)}</div>)}
                </div>
            </div>
        );
    };

    return (
        <div 
            style={{ 
                width: '100vw', 
                height: '100vh', 
                position: 'relative', 
                pointerEvents: yoloResult ? 'auto' : 'none',
                overflow: 'hidden'
            }}
            onClick={(e) => {
                // 左键点击空白处关闭详情页
                if (yoloResult && e.target === e.currentTarget) {
                    setYoloResult(null);
                    invoke("set_overlay_ignore_cursor", { ignore: true }).catch(() => {});
                }
            }}
        >
            {yoloResult && (
                <div 
                    ref={containerRef}
                    onMouseEnter={handleMouseEnter}
                    onMouseLeave={handleMouseLeave}
                    style={{
                        position: 'absolute',
                        left: `${detailPosition.x}%`,
                        top: `${detailPosition.y}%`,
                        transform: `translate(-50%, -50%) scale(${detailPosition.scale / 100})`,
                        width: `${detailPosition.width}px`,
                        height: `${detailPosition.height}px`,
                        pointerEvents: 'auto',
                        zIndex: 999,
                        overflow: 'visible',
                        minWidth: '200px',
                        background: 'rgba(20,15,10,0.98)',
                        border: '1px solid var(--c-golden)',
                        borderRadius: '12px',
                        boxShadow: '0 15px 45px rgba(0,0,0,0.9), 0 0 20px rgba(255,205,25,0.1)',
                        display: 'flex',
                        flexDirection: 'column'
                    }}
                >
                    {/* 标题栏（不可拖动） */}
                    <div 
                        style={{
                            height: '32px',
                            background: 'linear-gradient(90deg, rgba(255, 205, 25, 0.1), rgba(255, 205, 25, 0.2), rgba(255, 205, 25, 0.1))',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            padding: '0 8px',
                            flexShrink: 0,
                            borderBottom: '1px solid rgba(255,205,25,0.2)',
                            userSelect: 'none'
                        }}
                    >
                        <div style={{
                            fontSize: '10px',
                            color: 'rgba(255,205,25,0.8)',
                            letterSpacing: '3px',
                            fontWeight: 'bold',
                            textShadow: '0 0 5px rgba(0,0,0,0.5)'
                        }}>
                           ::: 识别结果 :::
                        </div>
                    </div>

                    <div style={{ flex: 1, padding: '16px', overflowY: 'auto', overflowX: 'hidden' }}>
                        {yoloResult.type === 'item' ? (
                            <div className="item-card-container expanded" style={{ border: 'none', boxShadow: 'none', background: 'transparent', margin: 0, padding: 0 }}>
                                {(() => {
                                    const item = yoloResult.data as ItemData;
                                    const tierClass = item.tier.split(' / ')[0].toLowerCase();
                                    const tierNameZh = {
                                        'bronze': '青铜+', 'silver': '白银+', 'gold': '黄金+', 'diamond': '钻石+'
                                    }[tierClass] || tierClass;
                                    const heroZh = item.heroes[0]?.split(' / ')[1] || item.heroes[0] || "通用";
                                    const sizeClass = item.size?.split(' / ')[0].toLowerCase() || 'medium';

                                    return (
                                        <Fragment>
                                            <div className={`item-card tier-${tierClass}`} style={{ marginBottom: '16px' }}>
                                                <div className="card-left">
                                                    <div className={`image-box size-${sizeClass}`} style={{ position: 'relative' }}>
                                                        <img src={item.displayImg} alt={item.name} />
                                                        {/* {renderStatBadges(item, tierClass)} */}
                                                    </div>
                                                </div>
                                                <div className="card-center">
                                                    <div className="name-line">
                                                        <span className="name-cn">{item.name_cn}</span>
                                                        <span className="id-badge">MATCH</span>
                                                        <span className={`tier-label tier-${tierClass}`}>{tierNameZh}</span>
                                                    </div>
                                                    <div className="tags-line" style={{ marginTop: '4px' }}>
                                                        {item.processed_tags.slice(0, 3).map(t => (
                                                            <span key={t} className="tag-badge">{t}</span>
                                                        ))}
                                                    </div>
                                                </div>
                                                <div className="card-right">
                                                    <span className="hero-badge">{heroZh}</span>
                                                </div>
                                            </div>

                                            {/* Skills Section */}
                                            {item.skills && item.skills.length > 0 && (
                                                <div className="details-right" style={{ marginBottom: '16px', gap: '8px' }}>
                                                    {item.skills.map((s, idx) => (
                                                        <div key={idx} className="skill-item">
                                                            {formatDescription(s.cn || s.en)}
                                                        </div>
                                                    ))}
                                                </div>
                                            )}

                                            {/* Enchantments Section */}
                                            {item.enchantments && item.enchantments.length > 0 && (
                                                <div className="item-enchantments-row" style={{ margin: '0 0 16px 0', borderTop: '1px solid rgba(255,255,255,0.05)', paddingTop: '12px' }}>
                                                    {item.enchantments.map((enc, idx) => {
                                                        const parts = enc.split('|');
                                                        if (parts.length > 1) {
                                                            const name = parts[0];
                                                            const effect = parts[1];
                                                            const color = ENCHANT_COLORS[name] || '#ffcd19';
                                                            return (
                                                                <div key={idx} className="enchant-item">
                                                                    <span className="enchant-badge" style={{ 
                                                                        '--enc-clr': color
                                                                    } as React.CSSProperties}>{name}</span>
                                                                    <span className="enchant-effect">{formatEnchantDescription(effect)}</span>
                                                                </div>
                                                            );
                                                        }
                                                        return <div key={idx} className="enchant-item">{formatEnchantDescription(enc)}</div>;
                                                    })}
                                                </div>
                                            )}

                                            {/* Tier Stats Section */}
                                            {renderStatTiers(item)}

                                            {/* Description Section */}
                                            <div className="item-description-row" style={{ margin: '16px 0 0 0', borderTop: 'none' }}>
                                                <div className="description-text" style={{ fontSize: '14px', borderTop: '1px dotted rgba(255,255,255,0.1)', paddingTop: '12px' }}>
                                                    {formatDescription(item.description)}
                                                </div>
                                            </div>
                                        </Fragment>
                                    );
                                })()}
                            </div>
                        ) : yoloResult.type === 'event' ? (
                            <div className="event-card-container" style={{ border: 'none', boxShadow: 'none', background: 'transparent', margin: 0, padding: 0 }}>
                                {(() => {
                                    const e = yoloResult.data as EventData;
                                    const eventTitle = e.name || e.name_en || e.InternalName || '';
                                    const eventDesc = '';  // 事件本身没有描述，只有choices有描述
                                    
                                    return (
                                        <Fragment>
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
                                                            <span className="id-badge" style={{ marginLeft: '8px' }}>EVENT</span>
                                                        </div>
                                                        {eventDesc && (
                                                            <div style={{ 
                                                                fontSize: '13px', 
                                                                color: 'rgba(255,255,255,0.7)',
                                                                lineHeight: '1.4'
                                                            }}>
                                                                {formatDescription(eventDesc)}
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
                                                            transition: 'all 0.2s ease',
                                                            cursor: 'pointer'
                                                        }}
                                                        onMouseEnter={(e) => {
                                                            e.currentTarget.style.background = 'rgba(255,205,25,0.1)';
                                                            e.currentTarget.style.borderColor = 'rgba(255,205,25,0.5)';
                                                        }}
                                                        onMouseLeave={(e) => {
                                                            e.currentTarget.style.background = 'rgba(255,255,255,0.05)';
                                                            e.currentTarget.style.borderColor = 'rgba(255,205,25,0.2)';
                                                        }}
                                                        >
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
                                        </Fragment>
                                    );
                                                })()}
                            </div>
                        ) : (
                                            <div className="monster-card-v2 expanded" style={{ border: 'none', boxShadow: 'none', background: 'transparent', margin: 0, padding: 0, overflow: 'visible' }}>
                                {(() => {
                                    const m = yoloResult.data as MonsterData;
                                    return (
                                        <Fragment>
                                            <div className="monster-header-v2" style={{ marginBottom: '16px', background: 'rgba(255,255,255,0.03)', borderRadius: '8px' }}>
                                                <div className="avatar-wrap">
                                                    <div className="monster-image-layers">
                                                        <img src={m.displayImgBg} className="monster-layer-bg" alt="" />
                                                        <img src={m.displayImg} className="monster-layer-char" alt="" />
                                                    </div>
                                                </div>
                                                <div className="monster-info-v2">
                                                    <div className="monster-name-zh">
                                                        {m.name_zh}
                                                        <span className="id-badge">MATCH</span>
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
                                                        {m.skills.map((s) => renderTierInfo(s))}
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
                                                                         // Fallback logic
                                                                         const availableTiers = Object.keys(tiers);
                                                                         if (availableTiers.length > 0) {
                                                                            if (availableTiers.includes("bronze")) currentTier = "bronze";
                                                                            else if (availableTiers.includes("silver")) currentTier = "silver";
                                                                            else if (availableTiers.includes("gold")) currentTier = "gold";
                                                                            else currentTier = availableTiers[0];
                                                                         }
                                                                    }
                                                                    
                                                                    // Tier border colors
                                                                    /*
                                                                    const tierBorderColors: Record<string, string> = {
                                                                        bronze: "#CD7F32",
                                                                        silver: "#C0C0C0", 
                                                                        gold: "#FFD700",
                                                                        diamond: "#B9F2FF",
                                                                        legendary: "#FF4500"
                                                                    };
                                                                    const borderColor = tierBorderColors[currentTier] || tierBorderColors.bronze;
                                                                    */
                                                                    
                                                                    // Size Logic (1:2:3)
                                                                    const sizeStr = (it.size || 'Medium').split(' / ')[0].toLowerCase();
                                                                    let widthUnit = 50;
                                                                    let widthMult = 2; // Medium
                                                                    if (sizeStr === 'small') widthMult = 1;
                                                                    if (sizeStr === 'large') widthMult = 3;
                                                                    
                                                                    const finalWidth = widthUnit * widthMult;
                                                                    const isHovered = hoveredMonsterItem === it;
                                                                    const isExpanded = expandedMonsterItem === it;
                                                                    
                                                                    // 获取当前tier的数值
                                                                    const stats = getTierStatValue(it, currentTier);
                                                                    const ammo = getTierValue(it.ammo_tiers || '', currentTier, it.starting_tier);
                                                                    const multicast = getTierValue(it.multicast_tiers || '', currentTier, it.starting_tier);
                                                                    
                                                                    // 获取CD - 尝试所有可能的来源
                                                                    let cdValue: string | null = null;
                                                                    
                                                                    // 1. 尝试从 cooldown_tiers 获取（多等级CD）
                                                                    if (it.cooldown_tiers && it.cooldown_tiers.trim()) {
                                                                        const cd = getTierValue(it.cooldown_tiers, currentTier, it.starting_tier);
                                                                        if (cd !== null && cd > 0) {
                                                                            cdValue = (cd > 100 ? cd / 1000 : cd).toFixed(1);
                                                                        }
                                                                    }
                                                                    
                                                                    // 2. 如果没有，尝试从 cooldown 获取（固定CD）
                                                                    if (!cdValue && it.cooldown && it.cooldown > 0) {
                                                                        cdValue = (it.cooldown > 100 ? it.cooldown / 1000 : it.cooldown).toFixed(1);
                                                                    }
                                                                    
                                                                    // 3. 如果还没有，尝试从 tiers[currentTier].cd 获取
                                                                    if (!cdValue && it.tiers && it.tiers[currentTier]) {
                                                                        const tierData = it.tiers[currentTier];
                                                                        if (tierData && tierData.cd && tierData.cd.trim()) {
                                                                            cdValue = tierData.cd;
                                                                        }
                                                                    }

                                                                    // 获取卡牌背景图路径
                                                                    const cardFramePath = getCardFramePath(currentTier, sizeStr);
                                                                    
                                                                    return (
                                                                        <div 
                                                                            key={`${it.name}-${idx}`}
                                                                            style={{
                                                                                width: `${finalWidth}px`,
                                                                                height: '95px',
                                                                                display: 'flex',
                                                                                alignItems: 'center',
                                                                                justifyContent: 'center',
                                                                                boxSizing: 'border-box',
                                                                                overflow: 'visible',
                                                                                cursor: 'pointer',
                                                                                position: 'relative',
                                                                                padding: '0',
                                                                                border: 'none',
                                                                                borderRadius: '0',
                                                                                filter: isHovered ? 'brightness(1.2) drop-shadow(0 0 10px rgba(255,255,255,0.5))' : 'none',
                                                                                transform: isHovered ? 'scale(1.08)' : 'scale(1)',
                                                                                transition: 'all 0.3s ease',
                                                                                zIndex: isHovered ? 100 : 1
                                                                            }}
                                                                            onMouseEnter={() => setHoveredMonsterItem(it)}
                                                                            onMouseLeave={() => setHoveredMonsterItem(null)}
                                                                            onClick={(e) => {
                                                                                e.stopPropagation();
                                                                                setExpandedMonsterItem(isExpanded ? null : it);
                                                                            }}
                                                                        >
                                                                            {/* 卡牌内容容器 - 负责剪裁溢出部分 */}
                                                                            <div style={{
                                                                                position: 'absolute',
                                                                                top: 0,
                                                                                left: 0,
                                                                                width: '100%',
                                                                                height: '100%',
                                                                                borderRadius: '6px',
                                                                                overflow: 'hidden',
                                                                                zIndex: 0
                                                                            }}>
                                                                                {/* 物品图片 (底层) */}
                                                                                {it.displayImg && (
                                                                                    <img 
                                                                                        src={it.displayImg} 
                                                                                        alt={it.name}
                                                                                        style={{ 
                                                                                            position: 'absolute',
                                                                                            top: '2%',
                                                                                            left: '2%',
                                                                                            width: '96%',
                                                                                            height: '96%',
                                                                                            objectFit: 'fill',
                                                                                            pointerEvents: 'none',
                                                                                            zIndex: 1,
                                                                                            borderRadius: '4px',
                                                                                        }} 
                                                                                    />
                                                                                )}

                                                                                {/* 卡牌边框 (顶层) */}
                                                                                {it.cardFrameImg ? (
                                                                                     <img 
                                                                                        src={it.cardFrameImg}
                                                                                        alt="Frame"
                                                                                        style={{
                                                                                            position: 'absolute',
                                                                                            top: 0,
                                                                                            left: 0,
                                                                                            width: '100%',
                                                                                            height: '100%',
                                                                                            zIndex: 2,
                                                                                            pointerEvents: 'none'
                                                                                        }}
                                                                                     />
                                                                                ) : (
                                                                                    <div style={{ position: 'absolute', inset: 0, zIndex: 2, pointerEvents: 'none' }}>
                                                                                         <CardBackground framePath={cardFramePath} getImg={getImg} />
                                                                                    </div>
                                                                                )}
                                                                            </div>
                                                                            
                                                                            {/* 数值标记（上方）*/}
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
                                                                            
                                                                            {/* CD显示（左下角）*/}
                                                                            {cdValue && (
                                                                                <div style={{
                                                                                    position: 'absolute',
                                                                                    bottom: '4px',
                                                                                    left: '4px',
                                                                                    background: 'rgba(0, 236, 195, 0.9)',
                                                                                    color: '#fff',
                                                                                    fontSize: '10px',
                                                                                    fontWeight: 'bold',
                                                                                    padding: '2px 4px',
                                                                                    borderRadius: '3px',
                                                                                    boxShadow: '0 2px 4px rgba(0,0,0,0.8)',
                                                                                    textShadow: '0 1px 2px rgba(0,0,0,0.8)',
                                                                                    zIndex: 5
                                                                                }}>
                                                                                    {cdValue}
                                                                                </div>
                                                                            )}
                                                                            
                                                                            {/* 多重攻击（中间）*/}
                                                                            {multicast && multicast >= 2 && (
                                                                                <div style={{
                                                                                    position: 'absolute',
                                                                                    top: '50%',
                                                                                    left: '50%',
                                                                                    transform: 'translate(-50%, -50%)',
                                                                                    background: 'rgba(255, 142, 0, 0.95)',
                                                                                    color: '#fff',
                                                                                    fontSize: '16px',
                                                                                    fontWeight: 'bold',
                                                                                    padding: '4px 8px',
                                                                                    borderRadius: '4px',
                                                                                    boxShadow: '0 3px 6px rgba(0,0,0,0.9)',
                                                                                    textShadow: '0 2px 4px rgba(0,0,0,0.9)',
                                                                                    zIndex: 5,
                                                                                    border: '2px solid rgba(255, 255, 255, 0.3)'
                                                                                }}>
                                                                                    ×{multicast}
                                                                                </div>
                                                                            )}
                                                                            
                                                                            
                                                                            
                                                                            {/* 弹药圆点（下方）*/}
                                                                            {ammo && ammo > 0 && (
                                                                                <div style={{
                                                                                    position: 'absolute',
                                                                                    bottom: '-6px',
                                                                                    left: '50%',
                                                                                    transform: 'translateX(-50%)',
                                                                                    display: 'flex',
                                                                                    gap: '3px',
                                                                                    zIndex: 10
                                                                                }}>
                                                                                    {Array.from({ length: Math.min(ammo, 10) }).map((_, i) => (
                                                                                        <div key={i} style={{
                                                                                            width: '8px',
                                                                                            height: '8px',
                                                                                            borderRadius: '50%',
                                                                                            background: '#ff8e00',
                                                                                            boxShadow: '0 0 6px rgba(255, 142, 0, 0.8), inset 0 1px 2px rgba(255,255,255,0.4)'
                                                                                        }} />
                                                                                    ))}
                                                                                </div>
                                                                            )}
                                                                        </div>
                                                                    );
                                                                })}
                                                            </div>
                                                            
                                                            {/* 悬浮详细信息 */}
                                                            {hoveredMonsterItem && (() => {
                                                                const tierNameMap: Record<string, string> = {
                                                                    bronze: '青铜',
                                                                    silver: '白银',
                                                                    gold: '黄金',
                                                                    diamond: '钻石',
                                                                    legendary: '传说'
                                                                };
                                                                const tierColorMap: Record<string, string> = {
                                                                    bronze: '#cd7f32',
                                                                    silver: '#c0c0c0',
                                                                    gold: '#ffd700',
                                                                    diamond: '#b9f2ff',
                                                                    legendary: '#e298ff'
                                                                };
                                                                const currentTier = (hoveredMonsterItem.current_tier || 'bronze').toLowerCase();
                                                                const isExpanded = expandedMonsterItem === hoveredMonsterItem;
                                                                
                                                                // CD显示：hover时只显示当前tier，点击后显示所有tier
                                                                let cdToShow: string | null = null;
                                                                let cdTiers: string[] = []; // 记录每个CD值对应的tier
                                                                if (!isExpanded) {
                                                                    // 未展开：只显示当前tier的CD
                                                                    let cdValue: number | null = null;
                                                                    
                                                                    // 1. 尝试从cooldown_tiers获取
                                                                    if (hoveredMonsterItem.cooldown_tiers) {
                                                                        cdValue = getTierValue(hoveredMonsterItem.cooldown_tiers, currentTier, hoveredMonsterItem.starting_tier);
                                                                    }
                                                                    
                                                                    // 2. 如果没有找到（例如tier只有1个值但当前是高等级），尝试从基础cooldown获取
                                                                    if (cdValue === null && hoveredMonsterItem.cooldown !== undefined && hoveredMonsterItem.cooldown > 0) {
                                                                        cdValue = hoveredMonsterItem.cooldown;
                                                                    }

                                                                    if (cdValue !== null) {
                                                                        cdToShow = (cdValue > 100 ? cdValue / 1000 : cdValue).toFixed(1);
                                                                        cdTiers = [currentTier];
                                                                    } else if (hoveredMonsterItem.tiers) {
                                                                        const currentTierInfo = hoveredMonsterItem.tiers[currentTier];
                                                                        if (currentTierInfo && currentTierInfo.cd) {
                                                                            cdToShow = currentTierInfo.cd;
                                                                            cdTiers = [currentTier];
                                                                        }
                                                                    }
                                                                } else {
                                                                    // 已展开：显示所有tier的CD
                                                                    if (hoveredMonsterItem.cooldown_tiers) {
                                                                        const tierOrder = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
                                                                        // 获取物品的可用tier，确定起始等级
                                                                        const availableTiers = hoveredMonsterItem.tiers ? Object.keys(hoveredMonsterItem.tiers).filter(t => hoveredMonsterItem.tiers[t]) : tierOrder;
                                                                        const startTierIndex = availableTiers.length > 0 ? tierOrder.indexOf(availableTiers[0]) : 0;
                                                                        
                                                                        const rawCDs = hoveredMonsterItem.cooldown_tiers.split('/');
                                                                        const cdValuesWithTier: {cd: string, tier: string}[] = [];
                                                                        rawCDs.forEach((v, i) => {
                                                                            const val = parseFloat(v.trim());
                                                                            const cd = (val > 100 ? val / 1000 : val).toFixed(1);
                                                                            // 使用起始等级偏移
                                                                            const tierIndex = startTierIndex + i;
                                                                            cdValuesWithTier.push({ cd, tier: tierOrder[tierIndex] || 'bronze' });
                                                                        });
                                                                        // 去重：保留第一次出现的CD和对应的tier
                                                                        const seen = new Set<string>();
                                                                        const unique = cdValuesWithTier.filter(item => {
                                                                            if (seen.has(item.cd)) return false;
                                                                            seen.add(item.cd);
                                                                            return true;
                                                                        });
                                                                        cdToShow = unique.map(u => u.cd).join('/');
                                                                        cdTiers = unique.map(u => u.tier);
                                                                    } else if (hoveredMonsterItem.cooldown) {
                                                                        const val = hoveredMonsterItem.cooldown;
                                                                        cdToShow = (val > 100 ? val / 1000 : val).toFixed(1);
                                                                        cdTiers = [currentTier];
                                                                    } else if (hoveredMonsterItem.tiers) {
                                                                        // 从所有可用的tier中收集CD
                                                                        const tierOrder = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
                                                                        const cdValuesWithTier: {cd: string, tier: string}[] = [];
                                                                        tierOrder.forEach(t => {
                                                                            if (hoveredMonsterItem.tiers[t] && hoveredMonsterItem.tiers[t].cd) {
                                                                                cdValuesWithTier.push({ cd: hoveredMonsterItem.tiers[t].cd, tier: t });
                                                                            }
                                                                        });
                                                                        // 去重
                                                                        const seen = new Set<string>();
                                                                        const unique = cdValuesWithTier.filter(item => {
                                                                            if (seen.has(item.cd)) return false;
                                                                            seen.add(item.cd);
                                                                            return true;
                                                                        });
                                                                        if (unique.length > 0) {
                                                                            cdToShow = unique.map(u => u.cd).join('/');
                                                                            cdTiers = unique.map(u => u.tier);
                                                                        }
                                                                    }
                                                                }

                                                                return (
                                                                    <div 
                                                                        style={{
                                                                            position: 'fixed',
                                                                            left: (winMousePos.current.x + 470 > window.innerWidth) 
                                                                                ? Math.max(10, winMousePos.current.x - 470) // 显示在左侧
                                                                                : winMousePos.current.x + 20, // 显示在右侧
                                                                            top: Math.min(winMousePos.current.y - 50, window.innerHeight - 300), // 鼠标上方50px，防止溢出
                                                                            width: '450px',
                                                                            maxWidth: '90vw',
                                                                            maxHeight: '80vh', // 限制最大高度
                                                                            overflow: 'auto', // 允许滚动
                                                                            zIndex: 9999,
                                                                            pointerEvents: 'auto',
                                                                            background: 'rgba(15, 15, 15, 0.98)',
                                                                            border: '2px solid rgba(255, 205, 25, 0.4)',
                                                                            borderRadius: '8px',
                                                                            padding: '16px',
                                                                            boxShadow: '0 8px 32px rgba(0,0,0,0.9), inset 0 0 20px rgba(255,205,25,0.05)',
                                                                            animation: 'fadeIn 0.2s ease-in',
                                                                            cursor: 'pointer'
                                                                        }}
                                                                        onClick={(e) => {
                                                                            e.stopPropagation();
                                                                            setExpandedMonsterItem(isExpanded ? null : hoveredMonsterItem);
                                                                        }}
                                                                    >
                                                                        <div style={{ fontSize: '16px', fontWeight: 'bold', color: '#ffcd19', marginBottom: '8px' }}>
                                                                            {hoveredMonsterItem.name}
                                                                        </div>
                                                                        <div style={{ fontSize: '12px', color: '#888', marginBottom: '12px' }}>
                                                                            尺寸: {hoveredMonsterItem.size || 'Medium'} | 等级: {tierNameMap[currentTier] || '青铜'}
                                                                            {cdToShow && <span> | CD: {cdToShow}</span>}
                                                                        </div>
                                                                        {!isExpanded && !hoveredMonsterItem.skills && (
                                                                            <div style={{ fontSize: '11px', color: '#666', marginBottom: '8px', fontStyle: 'italic' }}>
                                                                                点击查看升级信息
                                                                            </div>
                                                                        )}
                                                                        
                                                                        {/* CD和描述左右布局 */}
                                                                        <div style={{ display: 'flex', gap: '16px', alignItems: 'flex-start' }}>
                                                                            {/* 左侧CD */}
                                                                            {cdToShow && (
                                                                                <div style={{ 
                                                                                    minWidth: '80px',
                                                                                    flexShrink: 0,
                                                                                    padding: '8px 12px',
                                                                                    background: 'rgba(0, 236, 195, 0.08)',
                                                                                    borderRadius: '6px',
                                                                                    border: '1px solid rgba(0, 236, 195, 0.2)',
                                                                                    textAlign: 'center'
                                                                                }}>
                                                                                    <div style={{ 
                                                                                        fontSize: '13px', 
                                                                                        fontWeight: 'bold',
                                                                                        marginBottom: '4px'
                                                                                    }}>
                                                                                        {cdToShow && cdToShow.split('/').map((cd, idx) => (
                                                                                            <Fragment key={idx}>
                                                                                                <span style={{ 
                                                                                                    color: tierColorMap[cdTiers[idx]] || '#fff',
                                                                                                    textShadow: '0 0 4px rgba(0,0,0,0.5)'
                                                                                                }}>
                                                                                                    {cd}
                                                                                                </span>
                                                                                                {idx < cdToShow.split('/').length - 1 && <span style={{ color: '#666' }}>/</span>}
                                                                                            </Fragment>
                                                                                        ))}
                                                                                    </div>
                                                                                    <div style={{ fontSize: '10px', color: '#aaa' }}>
                                                                                        冷却
                                                                                    </div>
                                                                                </div>
                                                                            )}
                                                                            
                                                                            {/* 右侧描述 - 直接检查skills字段并逐条渲染，和物品一览完全相同 */}
                                                                            <div style={{ flex: 1 }}>
                                                                                {/* 1. 优先显示skills字段（物品自带的skill列表） */}
                                                                                {hoveredMonsterItem.skills && hoveredMonsterItem.skills.length > 0 && hoveredMonsterItem.skills.map((skill, i) => (
                                                                                    <div key={i} style={{ fontSize: '13px', color: '#ddd', marginBottom: '6px', lineHeight: '1.4' }}>
                                                                                        {(() => {
                                                                                            const text = skill.cn || skill.en;
                                                                                            // Calculate usage of tier
                                                                                            const isExpanded = expandedMonsterItem === hoveredMonsterItem;
                                                                                            
                                                                                            if (isExpanded) {
                                                                                                // Clicked state: Show full info
                                                                                                return formatDescription(text);
                                                                                            } else {
                                                                                                // Hover state: Show only current tier info
                                                                                                const tierMap: {[key: string]: number} = { 'bronze': 0, 'silver': 1, 'gold': 2, 'diamond': 3, 'legendary': 4 };
                                                                                                let startTierIndex = 0;
                                                                                                if (hoveredMonsterItem.starting_tier) {
                                                                                                    const startLower = hoveredMonsterItem.starting_tier.toLowerCase();
                                                                                                    if (startLower.includes('silver')) startTierIndex = 1;
                                                                                                    else if (startLower.includes('gold')) startTierIndex = 2;
                                                                                                    else if (startLower.includes('diamond')) startTierIndex = 3;
                                                                                                    else if (startLower.includes('legendary')) startTierIndex = 4;
                                                                                                }
                                                                                                const targetTierIndex = tierMap[currentTier] ?? 0;
                                                                                                const relativeIndex = Math.max(0, targetTierIndex - startTierIndex);
                                                                                                
                                                                                                const processed = text.replace(/(\d+(?:\/\d+)+)/g, (match) => {
                                                                                                    const nums = match.split('/');
                                                                                                    return nums[Math.min(relativeIndex, nums.length - 1)];
                                                                                                });
                                                                                                return formatDescription(processed);
                                                                                            }
                                                                                        })()}
                                                                                    </div>
                                                                                ))}
                                                                                
                                                                                {/* 2. 如果没有skills，回退到tiers字段的description和extra_description */}
                                                                                {(!hoveredMonsterItem.skills || hoveredMonsterItem.skills.length === 0) && hoveredMonsterItem.tiers && (() => {
                                                                                    const tierOrder = ['bronze', 'silver', 'gold', 'diamond'];
                                                                                    const isExpanded = expandedMonsterItem === hoveredMonsterItem;
                                                                                    
                                                                                    if (!isExpanded) {
                                                                                        // 未展开：只显示当前tier的描述
                                                                                        let currentTierInfo = hoveredMonsterItem.tiers[currentTier];
                                                                                        if (!currentTierInfo || !currentTierInfo.description || currentTierInfo.description.length === 0) {
                                                                                            for (const tierKey of tierOrder) {
                                                                                                const tierInfo = hoveredMonsterItem.tiers![tierKey];
                                                                                                if (tierInfo && tierInfo.description && tierInfo.description.length > 0) {
                                                                                                    currentTierInfo = tierInfo;
                                                                                                    break;
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        if (!currentTierInfo) return null;
                                                                                        
                                                                                        return (
                                                                                            <>
                                                                                                {currentTierInfo.description && currentTierInfo.description.map((desc, i) => (
                                                                                                    <div key={i} style={{ fontSize: '13px', color: '#ddd', marginBottom: '6px', lineHeight: '1.4' }}>
                                                                                                        {formatDescription(desc)}
                                                                                                    </div>
                                                                                                ))}
                                                                                                {currentTierInfo.extra_description && currentTierInfo.extra_description.map((desc, i) => (
                                                                                                    <div key={`extra-${i}`} style={{ fontSize: '13px', color: '#aaa', marginBottom: '6px', lineHeight: '1.4', fontStyle: 'italic' }}>
                                                                                                        {formatDescription(desc)}
                                                                                                    </div>
                                                                                                ))}
                                                                                            </>
                                                                                        );
                                                                                    }
                                                                                    
                                                                                    // 已展开：合并所有tier的数值，并去除重复数字
                                                                                    const mergeDescriptions = (key: 'description' | 'extra_description'): string[] => {
                                                                                        const result: string[] = [];
                                                                                        const firstTier = tierOrder.find(t => hoveredMonsterItem.tiers![t]);
                                                                                        if (!firstTier) return [];
                                                                                        
                                                                                        const firstTierData = hoveredMonsterItem.tiers![firstTier];
                                                                                        if (!firstTierData) return [];
                                                                                        const firstDescs = firstTierData[key] || [];
                                                                                        
                                                                                        firstDescs.forEach((_, lineIdx) => {
                                                                                            // 收集所有tier在这一行的描述
                                                                                            const descsAcrossTiers: string[] = [];
                                                                                            tierOrder.forEach(tier => {
                                                                                                const tierData = hoveredMonsterItem.tiers![tier];
                                                                                                if (tierData && tierData[key] && tierData[key][lineIdx]) {
                                                                                                    descsAcrossTiers.push(tierData[key][lineIdx]);
                                                                                                }
                                                                                            });
                                                                                            
                                                                                            if (descsAcrossTiers.length === 0) return;
                                                                                            
                                                                                            // 提取所有数字
                                                                                            const numbersPerTier = descsAcrossTiers.map(d => d.match(/\d+/g) || []);
                                                                                            
                                                                                            // 检查是否可以合并：所有tier的描述模板相同（去除数字后）
                                                                                            const templates = descsAcrossTiers.map(d => d.replace(/\d+/g, 'NUM'));
                                                                                            const allSameTemplate = templates.every(t => t === templates[0]);
                                                                                            
                                                                                            if (allSameTemplate && numbersPerTier.length > 0 && numbersPerTier[0].length > 0) {
                                                                                                // 合并每个位置的数字，去除重复
                                                                                                let merged = templates[0];
                                                                                                for (let numIdx = 0; numIdx < numbersPerTier[0].length; numIdx++) {
                                                                                                    const numsAtPos = numbersPerTier.map(nums => nums[numIdx]).filter(Boolean);
                                                                                                    // 去重：如果所有数字都相同，只保留一个
                                                                                                    const uniqueNums = [...new Set(numsAtPos)];
                                                                                                    merged = merged.replace('NUM', uniqueNums.join('/'));
                                                                                                }
                                                                                                result.push(merged);
                                                                                            } else {
                                                                                                // 无法合并，使用第一个tier的描述
                                                                                                result.push(descsAcrossTiers[0]);
                                                                                            }
                                                                                        });
                                                                                        
                                                                                        return result;
                                                                                    };
                                                                                    
                                                                                    const mergedDescriptions = mergeDescriptions('description');
                                                                                    const mergedExtraDescriptions = mergeDescriptions('extra_description');
                                                                                    
                                                                                    if (mergedDescriptions.length === 0 && mergedExtraDescriptions.length === 0) return null;
                                                                                    
                                                                                    return (
                                                                                        <>
                                                                                            {mergedDescriptions.map((desc, i) => (
                                                                                                <div key={i} style={{ fontSize: '13px', color: '#ddd', marginBottom: '6px', lineHeight: '1.4' }}>
                                                                                                    {formatDescription(desc)}
                                                                                                </div>
                                                                                            ))}
                                                                                            {mergedExtraDescriptions.map((desc, i) => (
                                                                                                <div key={`extra-${i}`} style={{ fontSize: '13px', color: '#aaa', marginBottom: '6px', lineHeight: '1.4', fontStyle: 'italic' }}>
                                                                                                    {formatDescription(desc)}
                                                                                                </div>
                                                                                            ))}
                                                                                        </>
                                                                                    );
                                                                                })()}
                                                                            </div>
                                                                        </div>
                                                                    </div>
                                                                );
                                                            })()}
                                                        </div>
                                                    </div>
                                                )}
                                            </div>
                                        </Fragment>
                                    );
                                })()}
                            </div>
                        )}
                    </div>
                    
                    {/* Resize Handle */}
                    <div 
                        onMouseDown={(e) => {
                            e.preventDefault();
                            setIsResizing(true);
                        }}
                        style={{
                            position: 'absolute',
                            right: '0',
                            bottom: '0',
                            width: '20px',
                            height: '20px',
                            cursor: 'nwse-resize',
                            background: 'linear-gradient(135deg, transparent 50%, rgba(255,205,25,0.3) 50%)',
                            borderBottomRightRadius: '12px'
                        }}
                    />

                    <div style={{ padding: '8px', fontSize: '10px', color: '#666', textAlign: 'center', borderTop: '1px solid rgba(255,255,255,0.05)', background: 'rgba(0,0,0,0.2)', flexShrink: 0 }}>
                        在设置中调整位置/大小 | 按设置的热键切换显示
                    </div>
                </div>
            )}

            {/* Realtime YOLO Monitor - Conditional */}
            {showYoloMonitor && (
                <div 
                ref={yoloMonitorRef}
                style={{
                    position: 'absolute',
                    left: '10px',
                    top: '10px',
                    background: 'rgba(0, 0, 0, 0.8)',
                    border: isPolling ? '1px solid #00ffcc' : '1px solid #444',
                    borderRadius: '8px',
                    padding: '0',
                    color: '#00ffcc',
                    fontFamily: 'Consolas, monospace',
                    zIndex: 1000,
                    width: '200px',
                    boxShadow: isPolling ? '0 0 10px rgba(0, 255, 204, 0.2)' : 'none',
                    pointerEvents: 'auto',
                    transition: 'border 0.2s, box-shadow 0.2s'
                }}
            >
                 <div 
                    style={{
                        padding: '4px 8px',
                        background: 'rgba(0, 255, 204, 0.1)',
                        fontSize: '12px',
                        fontWeight: 'bold',
                        borderBottom: '1px solid rgba(0, 255, 204, 0.3)',
                        userSelect: 'none',
                        display: 'flex',
                        justifyContent: 'space-between',
                        alignItems: 'center'
                    }}
                >
                     <span>⚡ YOLO {localStorage.getItem('use-gpu-acceleration') !== 'false' ? 'GPU' : 'CPU'}</span>
                     <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                        <button 
                            onMouseDown={(e) => e.stopPropagation()}
                            onClick={async (e) => {
                                e.stopPropagation();
                                if (isPolling) return;
                                setIdentifying(true);
                                setIsPolling(true);
                                try {
                                    if ((window as any).__yolo_running) {
                                        console.log("[Overlay] YOLO scan already running, skipping realtime trigger");
                                    } else {
                                        (window as any).__yolo_running = true;
                                        const useGpu = localStorage.getItem('use-gpu-acceleration') === 'true';
                                        await invoke('trigger_yolo_scan', { useGpu: useGpu });
                                        // 获取统计信息
                                        const stats = await invoke('get_yolo_stats');
                                        setYoloStats(stats as any);
                                    }
                                } catch (e) {
                                    console.error("Manual scan failed:", e);
                                } finally {
                                    (window as any).__yolo_running = false;
                                    setTimeout(() => {
                                        setIdentifying(false);
                                        setIsPolling(false);
                                    }, 800);
                                }
                            }}
                            style={{
                                background: 'rgba(0, 255, 204, 0.1)',
                                border: '1px solid rgba(0, 255, 204, 0.5)',
                                color: '#00ffcc',
                                fontSize: '10px',
                                padding: '1px 5px',
                                borderRadius: '3px',
                                cursor: 'pointer'
                            }}
                        >
                            Scan
                        </button>
                        <span style={{ 
                            color: isPolling ? '#00ffcc' : '#666', 
                            opacity: isPolling ? 1 : 0.5,
                            fontSize: '10px',
                            width: '10px',
                            textAlign: 'center'
                        }}>
                            {isPolling ? "●" : "○"}
                        </span>
                     </div>
                </div>
                <div style={{ padding: '8px', minHeight: '32px' }}>
                    {yoloStats ? (
                        <div style={{ fontSize: '11px', color: '#00ffcc', marginBottom: '6px' }}>
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '2px', marginBottom: '4px' }}>
                                <div>🃏 卡牌: {yoloStats.items}</div>
                                <div>🎨 技能: {yoloStats.skills}</div>
                                <div>📜 事件: {yoloStats.events}</div>
                                <div>👾 怪物: {yoloStats.monsters}</div>
                            </div>
                            <div style={{ fontSize: '10px', color: '#666', textAlign: 'center', borderTop: '1px solid rgba(255,255,255,0.1)', paddingTop: '4px' }}>
                                总计: {yoloStats.total} 个目标
                            </div>
                        </div>
                    ) : null}
                    {realtimeList && realtimeList.length > 0 ? (
                        <div style={{ borderTop: yoloStats ? '1px solid rgba(255,255,255,0.1)' : 'none', paddingTop: yoloStats ? '6px' : '0' }}>
                            <div style={{ fontSize: '10px', color: '#666', marginBottom: '4px' }}>识别结果:</div>
                            {realtimeList.map((name, i) => (
                                <div key={i} style={{ fontSize: '11px', marginBottom: '2px', display: 'flex', alignItems: 'center' }}>
                                    <span style={{ marginRight: '6px', opacity: 0.7 }}>&gt;</span>
                                    {name}
                                </div>
                            ))}
                        </div>
                    ) : (
                        !yoloStats && (
                            <div style={{ fontSize: '11px', color: '#666', fontStyle: 'italic', textAlign: 'center', paddingTop: '4px' }}>
                                {isPolling ? "Scanning..." : "No targets"}
                            </div>
                        )
                    )}
                </div>
                </div>
            )}
        </div>
    );
}