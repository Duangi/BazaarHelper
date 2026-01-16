import { useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalPosition, LogicalSize, currentMonitor } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import { getVersion } from '@tauri-apps/api/app';
import { check, Update } from '@tauri-apps/plugin-updater';
import "./App.css";

import { exit, relaunch } from '@tauri-apps/plugin-process';

// --- 接口定义 ---
interface ItemData {
  uuid: string;
  name: string;
  name_cn: string;
  tier: string;
  tags: string;
  processed_tags: string[];
  heroes: string[];
  cooldown?: number;
  skills: string[];
  enchantments: string[];
  description: string;
  image: string;
  displayImg?: string;
}

interface SyncPayload {
  hand_items: ItemData[];
  stash_items: ItemData[];
  all_tags: string[];
}

interface TierInfo {
  description: string[];
  extra_description: string[];
  cd: string | null;
}

interface MonsterSubItem { 
  name: string; 
  name_en?: string;
  tier?: string;
  current_tier?: string;
  tags?: string[];
  tiers: Record<string, TierInfo | null>;
  image: string; 
  displayImg?: string; 
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
}

type TabType = "hand" | "stash" | "monster";

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

const TIER_COLORS = ["#cd7f32", "#c0c0c0", "#ffd700", "#b9f2ff"]; // Bronze, Silver, Gold, Diamond

export default function App() {
  const [activeTab, setActiveTab] = useState<TabType>("monster");
  const [syncData, setSyncData] = useState<SyncPayload & { monster: any[] }>({ 
    hand_items: [], 
    stash_items: [], 
    all_tags: [],
    monster: [] 
  });
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [manualMonsters, setManualMonsters] = useState<MonsterData[]>([]);
  const [allMonsters, setAllMonsters] = useState<Record<string, MonsterData>>({});
  const [selectedDay, setSelectedDay] = useState<string>("");
  const [identifiedNames, setIdentifiedNames] = useState<string[]>([]); // 存储按顺序识别到的怪物名
  const [pinnedItems, setPinnedItems] = useState<Map<string, number>>(new Map()); // 存储置顶物品ID和置顶时间戳
  const [pinnedCounter, setPinnedCounter] = useState(0); // 置顶计数器，用于确定置顶顺序
  const [isRecognizing, setIsRecognizing] = useState(false); // 是否正在识别怪物
  const [templateLoading, setTemplateLoading] = useState({ loaded: 0, total: 0, is_complete: false, current_name: "" }); // 模板加载进度
  const [currentDay, setCurrentDay] = useState<number | null>(null);
  const [progressionMode, setProgressionMode] = useState<Set<string>>(new Set()); // 记录哪些卡片开启了“数值横评模式”
  const [fontSize, setFontSize] = useState(() => {
    const saved = localStorage.getItem("user-font-size");
    return saved ? parseInt(saved, 10) : 16;
  }); // 自定义字号
  const [showSettings, setShowSettings] = useState(false);
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set()); // 手牌/仓库点击展开附魔
  const [expandedMonsters, setExpandedMonsters] = useState<Set<string>>(new Set()); // 野怪点击展开

  // 图片路径缓存，避免重复解析
  const [imgCache] = useState<Map<string, string>>(new Map());

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const appWindow = getCurrentWindow(); // 获取当前窗口实例
  
  // 自定义位置状态，用于记忆用户是否手动拖拽过窗口
  const [hasCustomPosition, setHasCustomPosition] = useState(false);
  const lastKnownPosition = useRef<{ x: number; y: number } | null>(null);
  
  // 存储当前屏幕缩放比例，用于坐标转换
  const currentScale = useRef(1);
  
  // 初始化完成标志，防止初始定位触发移动监听
  const isInitialized = useRef(false);
  const moveDebounceTimer = useRef<number | null>(null);
  
  // 版本显示相关状态
  const [showVersionScreen, setShowVersionScreen] = useState(true); // 启动时显示版本号
  const [currentVersion, setCurrentVersion] = useState(""); // 当前版本号
  
  // 更新相关状态
  const [updateAvailable, setUpdateAvailable] = useState<Update | null>(null);
  const [updateStatus, setUpdateStatus] = useState<"none" | "checking" | "available" | "downloading" | "ready">("none");
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [isInstalling, setIsInstalling] = useState(false); // 正在安装状态

  // 置顶/取消置顶功能
  const togglePin = (uuid: string, e: React.MouseEvent) => {
    e.stopPropagation(); // 防止触发展开/收起
    setPinnedItems(prev => {
      const newPinned = new Map(prev);
      if (newPinned.has(uuid)) {
        newPinned.delete(uuid);
      } else {
        setPinnedCounter(c => c + 1);
        newPinned.set(uuid, pinnedCounter + 1);
      }
      return newPinned;
    });
  };

  const toggleExpand = (uuid: string) => {
    setExpandedItems(prev => {
      const next = new Set(prev);
      if (next.has(uuid)) next.delete(uuid);
      else next.add(uuid);
      return next;
    });
  };

  const toggleMonsterExpand = (name_zh: string) => {
    setExpandedMonsters(prev => {
      const next = new Set(prev);
      if (next.has(name_zh)) next.delete(name_zh);
      else next.add(name_zh);
      return next;
    });
  };

  const renderText = (text: string) => {
    if (!text) return null;
    
    // 1. 处理数值序列如 3/6/9/12 或 9/12
    // 逻辑：匹配由数字和斜杠组成的模式
    const parts = text.split(/(\d+(?:\/\d+)+)/g);
    
    return parts.map((part, i) => {
      if (part.includes('/')) {
        const nums = part.split('/');
        return (
          <span key={i} className="progression-nums">
            {nums.map((n, idx) => {
              // 决定颜色偏移量。如果有4个数则是0,1,2,3。如果有2个数且是高阶卡通常是2,3
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

      // 2. 处理关键词和标签颜色
      // 构建正则，包含关键词和动态从 backend 获取的 tags
      const keywords = Object.keys(KEYWORD_COLORS);
      const tags = syncData.all_tags || [];
      const allMatches = [...new Set([...keywords, ...tags])].filter(k => k.length > 0);
      
      if (allMatches.length === 0) return part;
      
      const regex = new RegExp(`(${allMatches.join('|')})`, 'g');
      const subParts = part.split(regex);
      
      return subParts.map((sub, j) => {
        if (KEYWORD_COLORS[sub]) {
          return <span key={j} style={{ color: KEYWORD_COLORS[sub], fontWeight: 'bold' }}>{sub}</span>;
        }
        if (tags.includes(sub)) {
          return <span key={j} style={{ color: '#8eba31', fontWeight: 'bold' }}>{sub}</span>; // 统一标签颜色
        }
        return sub;
      });
    });
  };

 // 获取当前 Day 并定期刷新
 useEffect(() => {
   let mounted = true;
   const fetchDay = async () => {
     try {
       const d = await invoke<number>("get_current_day", { hours_per_day: 6, retro: true });
       if (mounted) {
         if (d !== currentDay) {
           setCurrentDay(d);
           // 初始加载 or 检测到变化时，更新选中的标签
           updateDayTabSelection(d);
         }
       }
     } catch (e) {
       console.warn("get_current_day failed:", e);
     }
   };
   fetchDay();
   // 移除 setInterval 自动刷新，只在初始化和收到后端事件时刷新
   return () => { mounted = false; };
 }, []); // 仅在挂载时运行一次

 // 辅助函数：根据天数数字更新选中的 Tab
 const updateDayTabSelection = (day: number) => {
   const dayStr = day >= 10 ? "Day 10+" : `Day ${day}`;
   setSelectedDay(dayStr);
 };
  // 获取排序后的物品列表（手牌和仓库）
  const getSortedItems = (items: ItemData[]) => {
    return [...items].sort((a, b) => {
      const aPin = pinnedItems.get(a.uuid);
      const bPin = pinnedItems.get(b.uuid);
      if (aPin && bPin) return bPin - aPin; // 都置顶，后置顶的在前
      if (aPin) return -1; // a置顶，a在前
      if (bPin) return 1; // b置顶，b在前
      return 0; // 都不置顶，保持原顺序
    });
  };

  // 1. 记忆宽度与高度
  const [expandedWidth, setExpandedWidth] = useState(() => {
    const saved = localStorage.getItem("plugin-width");
    return saved ? parseInt(saved, 10) : 400;
  });
  const [expandedHeight, setExpandedHeight] = useState(() => {
    const saved = localStorage.getItem("plugin-height");
    return saved ? parseInt(saved, 10) : 700;
  });

  // 图片路径处理函数
  const getImg = async (path: string | null | undefined) => {
    if (!path) return "";
    if (imgCache.has(path)) return imgCache.get(path)!;
    try {
      const fullPath = await resolveResource(`resources/${path}`);
      const assetUrl = convertFileSrc(fullPath);
      imgCache.set(path, assetUrl);
      return assetUrl;
    } catch { return ""; }
  };
  
  const enterApp = () => {
    console.log("[Update] Entering App. updateAvailable:", !!updateAvailable);
    setShowVersionScreen(false);
    invoke("start_template_loading").catch(console.error);
    
    // 如果有更新，进入应用后开始后台下载
    if (updateAvailable) {
      console.log("[Update] Found update, starting background download...");
      startUpdateDownload();
    } else {
      console.log("[Update] No update found, skipping download.");
    }
  };

  const startUpdateDownload = async () => {
    if (!updateAvailable) {
      console.warn("[Update] startUpdateDownload 被调用，但没有可用更新");
      return;
    }
    
    try {
      console.log(`[Update] 开始下载并安装版本: ${updateAvailable.version} (当前版本: ${currentVersion})`);
      setUpdateStatus("downloading");
      let downloaded = 0;
      let total = 0;
      
      await updateAvailable.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            total = event.data.contentLength || 0;
            console.log(`[Update] 下载开始。总大小: ${total} 字节`);
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (total > 0) {
              const p = Math.round((downloaded / total) * 100);
              if (p % 10 === 0 && p !== downloadProgress) { 
                console.log(`[Update] 下载进度: ${p}% (${downloaded}/${total})`);
              }
              setDownloadProgress(p);
            }
            break;
          case 'Finished':
            console.log('[Update] 下载完成，更新已就绪。');
            setUpdateStatus("ready");
            break;
        }
      });
    } catch (e) {
      console.error("[Update] 自动更新失败:", e);
      setUpdateStatus("available"); 
    }
  };

  // 启动时显示版本信息并检查更新
  useEffect(() => {
    const initApp = async () => {
      console.log("[App] initApp 开始执行...");
      try {
        const appVersion = await getVersion();
        setCurrentVersion(appVersion);
        console.log(`[App] 启动初始化。当前版本: v${appVersion}`);

        // 检查更新
        console.log("[Update] 正在连接服务器检查更新...");
        setUpdateStatus("checking");
        const update = await check();
        console.log("[Update] check() 响应结果:", update);
        if (update) {
          console.log(`[Update] 检测到新版本! 远端版本: v${update.version}, 发布日期: ${update.date}`);
          setUpdateAvailable(update);
          setUpdateStatus("available");
        } else {
          console.log("[Update] 已经是最新版本 (v" + appVersion + ")，无需更新。");
          setUpdateStatus("none");
        }
        
        // 3.5秒后自动进入应用
        setTimeout(() => {
          enterApp();
        }, 3500);
      } catch (error) {
        console.error("[App] 初始化加载更新失败:", error);
        setUpdateStatus("none");
        setTimeout(() => {
          enterApp();
        }, 2000);
      }
    };
    
    initApp();
  }, []);

  // 轮询检查模板加载进度
  useEffect(() => {
    let timer: any = null;
    
    const checkProgress = async () => {
      try {
        const progress = await invoke("get_template_loading_progress") as any;
        setTemplateLoading(progress);
        
        // 如果加载完成，停止轮询
        if (progress.is_complete && timer) {
          clearInterval(timer);
          timer = null;
        }
      } catch (e) {
        console.error("获取加载进度失败:", e);
      }
    };
    
    // 立即执行一次
    checkProgress();
    
    // 每500ms检查一次
    timer = setInterval(checkProgress, 500);
    
    return () => {
      if (timer) {
        clearInterval(timer);
      }
    };
  }, []); // 只在mount时执行一次

  // 监听后端事件
  useEffect(() => {
    let unlistenMonster: any = null;
    let unlistenDay: any = null;
    let unlistenSync: any = null;
    
    const setupListeners = async () => {
      // 1. 怪物识别触发
      unlistenMonster = await listen<number | null>('trigger-monster-recognition', async (event) => {
        console.log("收到自动识别触发事件, Day:", event.payload);
        const dayNum = event.payload;
        if (dayNum) {
          const dayLabel = dayNum >= 10 ? "Day 10+" : `Day ${dayNum}`;
          setSelectedDay(dayLabel);
          setCurrentDay(dayNum);
        }
        setTimeout(async () => {
           await handleAutoRecognition(dayNum);
        }, 500);
      });

      // 2. 天数更新
      unlistenDay = await listen<number>('day-update', (event) => {
        console.log("收到天数更新事件:", event.payload);
        const d = event.payload;
        setCurrentDay(d);
        const dayLabel = d >= 10 ? "Day 10+" : `Day ${d}`;
        setSelectedDay(dayLabel);
      });

      // 3. 物品同步 (sync-items)
      unlistenSync = await listen<SyncPayload>("sync-items", async (event) => {
        const payload = event.payload;
        const processItems = (items: ItemData[]) => 
          Promise.all(items.map(async (i) => ({ ...i, displayImg: await getImg(i.image) })));

        const [hand, stash] = await Promise.all([
          processItems(payload.hand_items || []),
          processItems(payload.stash_items || [])
        ]);

        setSyncData(prev => ({ 
          ...prev, 
          hand_items: hand, 
          stash_items: stash, 
          all_tags: payload.all_tags || [] 
        }));
      });
    };
    
    setupListeners();
    
    return () => {
      if (unlistenMonster) unlistenMonster();
      if (unlistenDay) unlistenDay();
      if (unlistenSync) unlistenSync();
    };
  }, []); // 仅在挂载时运行一次，不再依赖 currentDay

  // 基础环境侦测：分辨率适配
  useEffect(() => {
    const detectScale = async () => {
      try {
        const monitor = await currentMonitor();
        if (monitor) {
          currentScale.current = monitor.scaleFactor;
          const { height } = monitor.size;
          const logicalHeight = height / monitor.scaleFactor;
          console.log(`[Screen] height: ${height}, scale: ${monitor.scaleFactor}, logical: ${logicalHeight}`);
          
          // 初始高度适配逻辑：如果没有保存过高度，则默认屏幕高度 - 200
          if (!localStorage.getItem("plugin-height")) {
            setExpandedHeight(Math.max(600, Math.floor(logicalHeight - 200)));
          }
        }
      } catch (e) {
        console.error("检测屏幕信息失败:", e);
      }
    };
    detectScale();
  }, []);

  // 监听窗口移动事件，检测用户拖拽
  useEffect(() => {
    const setupMoveListener = async () => {
      // 等待2秒后才开始监听，避免初始定位触发
      setTimeout(() => {
        isInitialized.current = true;
      }, 2000);

      // 监听窗口移动事件 (Tauri v2)
      const unlisten = await appWindow.listen<{ x: number; y: number }>('tauri://move', (event) => {
        // 如果还在初始化阶段，忽略移动事件
        if (!isInitialized.current) {
          return;
        }

        // 清除之前的防抖定时器
        if (moveDebounceTimer.current) {
          clearTimeout(moveDebounceTimer.current);
        }

        // 设置防抖定时器，只有停止移动200ms后才记录位置
        moveDebounceTimer.current = window.setTimeout(() => {
          // 【关键修复】直接存储物理坐标，不做任何转换
          // 这是绝对真理，不随缩放改变
          setHasCustomPosition(true);
          lastKnownPosition.current = { x: event.payload.x, y: event.payload.y };
        }, 200);
      });
      return unlisten;
    };

    const unlistenPromise = setupMoveListener();
    return () => {
      unlistenPromise.then(unlisten => { if(unlisten) unlisten(); });
      if (moveDebounceTimer.current) {
        clearTimeout(moveDebounceTimer.current);
      }
    };
  }, []); // 只在组件挂载时运行一次

  // 加载全量怪物数据
  useEffect(() => {
    const loadAllMonsters = async () => {
      try {
        const res: Record<string, MonsterData> = await invoke("get_all_monsters");
        setAllMonsters(res);
      } catch (e) {
        console.error("加载全量怪物失败:", e);
      }
    };
    loadAllMonsters();
  }, []);

  // 当 selectedDay 或 allMonsters 改变时，更新显示的怪物
  useEffect(() => {
    if (activeTab === "monster") {
       updateFilteredMonsters(selectedDay);
    }
  }, [activeTab, selectedDay, allMonsters, identifiedNames]);

  const updateFilteredMonsters = async (day: string) => {
    // 如果天数还没加载出来，且目前已经有怪物全量数据，默认显示第一天
    let targetDay = day;
    if (!targetDay && Object.keys(allMonsters).length > 0) {
      targetDay = "Day 1";
    }

    const monstersOnDay = Object.values(allMonsters).filter(m => m.available === targetDay);
    
    // 如果在该天数下没有找到怪物，可能是加载还没完成或者数据格式匹配问题
    if (monstersOnDay.length === 0 && Object.keys(allMonsters).length > 0 && targetDay !== "") {
       console.warn(`[MonsterTab] No monsters found for ${targetDay}, total monsters in DB: ${Object.keys(allMonsters).length}`);
    }

    // 根据识别结果进行排序
    const sorted = [...monstersOnDay].sort((a, b) => {
      const indexA = identifiedNames.indexOf(a.name_zh);
      const indexB = identifiedNames.indexOf(b.name_zh);
      
      const posA = indexA === -1 ? 999 : indexA;
      const posB = indexB === -1 ? 999 : indexB;
      
      return posA - posB;
    });

    const processed = await Promise.all(sorted.map(processMonsterImages));
    setManualMonsters(processed);
  };

  const processMonsterImages = async (m: MonsterData) => {
    return {
      ...m,
      displayImg: await getImg(m.image),
      skills: m.skills ? await Promise.all(m.skills.map(async s => ({ 
        ...s, 
        displayImg: await getImg(s.image) 
      }))) : [],
      items: m.items ? await Promise.all(m.items.map(async i => ({ 
        ...i, 
        displayImg: await getImg(i.image) 
      }))) : []
    };
  };

  const renderTierInfo = (item: MonsterSubItem) => {
    const isProgressionActive = progressionMode.has(item.name + (item.current_tier || ''));
    
    const toggleProgression = (e: React.MouseEvent) => {
      e.stopPropagation();
      const key = item.name + (item.current_tier || '');
      const newModes = new Set(progressionMode);
      if (newModes.has(key)) newModes.delete(key);
      else newModes.add(key);
      setProgressionMode(newModes);
    };

    // 辅助格式化函数
    const formatDescription = (text: string) => {
      const parts = text.split(/(\[Locked\]|Quest:)/g);
      return parts.map((part, i) => {
        if (part === "[Locked]") return <span key={i} className="icon-locked" title="Locked">🔒</span>;
        if (part === "Quest:") return <span key={i} className="icon-quest" title="Quest">📜</span>;
        return part;
      });
    };

    // 兼容性修整：如果 current_tier 不存在，尝试根据名称中是否包含级位来猜测
    let currentTier = "bronze";
    const tiers: Record<string, TierInfo | null> = (item as any).tiers || {};
    
    if (item.current_tier) {
      currentTier = item.current_tier.toLowerCase();
    } else {
      // 检查 tiers 对象里有哪些 key，有些数据可能直接把数据塞到了特定的 key 里
      const availableTiers = Object.keys(tiers);
      if (availableTiers.length > 0) {
        // 如果只有一个 key 或者包含特定的 key
        if (availableTiers.includes("bronze")) currentTier = "bronze";
        else if (availableTiers.includes("silver")) currentTier = "silver";
        else if (availableTiers.includes("gold")) currentTier = "gold";
        else currentTier = availableTiers[0]; // 实在不行拿第一个
      }
    }

    const tierData = tiers[currentTier];
    // 如果该级位没数据，显示第一个有数据的级位
    const finalData = tierData || Object.values(tiers).find(t => t !== null);
    
    // --- 升级效果合并逻辑 (用于显示在卡片上或悬浮框) ---
    const getProgressionText = (line: string, lineIdx: number, field: 'description' | 'extra_description' = 'description') => {
      const tierSequence = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
      const activeTiers = tierSequence
        .map(t => ({ tier: t, data: tiers[t] }))
        .filter(t => t.data !== null && t.data !== undefined);
      
      const numRegex = /(\d+(\.\d+)?%?)/g;
      const matches = [...line.matchAll(numRegex)];
      
      if (matches.length > 0 && activeTiers.length > 1) {
        let lastIndex = 0;
        const parts: any[] = [];
        matches.forEach((match, mIdx) => {
          const tierValues = activeTiers.map(at => {
            const fieldData = (at.data as any)[field] || [];
            const atMatches = [...(fieldData[lineIdx] || "").matchAll(numRegex)];
            return atMatches[mIdx] ? atMatches[mIdx][0] : match[0];
          });

          // 如果所有数值都一致，则不显示升级箭头，保持原样
          const isConstant = tierValues.every(v => v === tierValues[0]);

          parts.push(line.substring(lastIndex, match.index));
          if (isConstant) {
            parts.push(match[0]);
          } else {
            parts.push(
              <span key={mIdx} className="progression-inline-values">
                {tierValues.map((val, i) => (
                  <span key={activeTiers[i].tier}>
                    <span className={`val-${activeTiers[i].tier}`}>{val}</span>
                    {i < activeTiers.length - 1 && <span className="upgrade-arrow">»</span>}
                  </span>
                ))}
              </span>
            );
          }
          lastIndex = match.index! + match[0].length;
        });
        parts.push(line.substring(lastIndex));
        return parts;
      }
      return formatDescription(line);
    };

    if (!finalData) {
      return (
        <div className="sub-item-card tier-unknown">
           <div className="sub-item-header">
              <div className="sub-item-img-wrap">
                <img src={item.displayImg} className="sub-item-img" />
              </div>
              <span className="sub-item-name">{item.name} (无描述)</span>
           </div>
        </div>
      );
    }

    const borderColorMap: Record<string, string> = {
      bronze: "#CD7F32",
      silver: "#C0C0C0",
      gold: "#FFD700",
      diamond: "#B9F2FF",
      legendary: "#FF4500",
    };
    const borderColor = borderColorMap[currentTier] || borderColorMap.bronze;

    return (
      <div 
        className={`sub-item-card tier-${currentTier} ${isProgressionActive ? 'progression-active' : ''}`} 
        style={{ borderLeft: `4px solid ${borderColor}` }}
        onClick={toggleProgression}
      >
        <div className="sub-item-header">
          <div className="sub-item-img-wrap" style={{ outline: `2px solid ${borderColor}` }}>
            <img src={item.displayImg} className="sub-item-img" />
          </div>
          <div className="sub-item-title-row">
            <span className="sub-item-name">{item.name}</span>
            {(() => {
                const tierSequence = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
                const activeTiers = tierSequence
                  .map(t => ({ tier: t, data: (item.tiers as any)?.[t] }))
                  .filter(t => t.data !== null && t.data !== undefined);

                if (isProgressionActive && activeTiers.length > 1) {
                  const cdValues = activeTiers.map(at => at.data!.cd || "");
                  const isConstant = cdValues.every(v => v === cdValues[0]);
                  if (!cdValues.some(v => v)) return null; 
                  if (isConstant) return <div className="sub-item-cd">⏳ {cdValues[0]}</div>;
                  return (
                    <div className="sub-item-cd progression-cd">
                      ⏳ {cdValues.map((v, i) => (
                        <span key={activeTiers[i].tier}>
                          <span className={`val-${activeTiers[i].tier}`}>{v || '-'}</span>
                          {i < activeTiers.length - 1 && <span className="upgrade-arrow">»</span>}
                        </span>
                      ))}
                    </div>
                  );
                } else {
                  return finalData.cd && <div className="sub-item-cd">⏳ {finalData.cd}</div>;
                }
            })()}
          </div>
        </div>
        <div className="sub-item-desc">
          {finalData.description.map((d, i) => (
            <div key={i} className="desc-line">
              {isProgressionActive ? getProgressionText(d, i, 'description') : formatDescription(d)}
            </div>
          ))}
          {finalData.extra_description?.map((d, i) => (
            <div key={`extra-${i}`} className="desc-line extra-desc">
              {isProgressionActive ? getProgressionText(d, i, 'extra_description') : formatDescription(d)}
            </div>
          ))}
        </div>
      </div>
    );
  };

  // 手动修改当前天数
  const handleDayChange = async (newDay: number) => {
    if (newDay < 1) return;
    setCurrentDay(newDay);
    updateDayTabSelection(newDay); // 手动修改时也跳转 Tab
    try {
      await invoke("update_day", { day: newDay });
    } catch (e) {
      console.error("更新天数失败:", e);
    }
  };

  const handleAutoRecognition = async (day: number | null) => {
    if (isRecognizing) return;
    setIsRecognizing(true);
    try {
      console.log(`[Recognition] Triggering recognition for Day: ${day}`);
      const results = await invoke("recognize_monsters_from_screenshot", { day }) as any[];
      if (results && results.length > 0) {
        const names = new Array(3).fill("");
        results.forEach(r => {
          if (r.position >= 1 && r.position <= 3) names[r.position - 1] = r.name;
        });
        const validNames = names.filter(n => n !== "");
        console.log(`[Recognition Success] Found: ${validNames.join(', ')}`);
        setIdentifiedNames(validNames);
        
        // 自动展开识别出的怪物，方便用户直接看到技能
        setExpandedMonsters(prev => {
          const next = new Set(prev);
          validNames.forEach(name => {
            // 在 monsters_db.json 中，key 已经就是中文字符串
            if (allMonsters[name]) next.add(name);
          });
          return next;
        });
      } else {
        console.log("[Recognition] No monsters found in screenshot");
      }
    } catch (e) {
      console.error("自动识别失败:", e);
      if (typeof e === 'string' && e.includes("Templates not loaded")) {
        console.warn("[Recognition] Templates still loading, will not auto-retry. Please ensure 'Enter App' was clicked.");
      }
    } finally {
      setIsRecognizing(false);
    }
  };

  // 手动触发怪物识别
  const handleManualRecognition = async () => {
    setIsRecognizing(true);
    try {
      console.log("开始手动识别怪物...");
      // 传入当前天数以加速识别
      const results = await invoke("recognize_monsters_from_screenshot", { day: currentDay }) as any[];
      console.log("识别结果:", results);
      
      if (results && results.length > 0) {
        // 按照 position (1, 2, 3) 提取怪物名
        const names = new Array(3).fill("");
        results.forEach(r => {
          if (r.position >= 1 && r.position <= 3) {
            names[r.position - 1] = r.name;
          }
        });
        
        // 过滤掉空的，保留 [左, 中, 右] 的顺序
        const validNames = names.filter(n => n !== "");
        console.log("%c[识别成功]", "color: #ffcd19; font-weight: bold", "识别到的怪物顺序 (从左至右):", validNames);
        setIdentifiedNames(validNames);

        // 识别后自动展开
        setExpandedMonsters(prev => {
          const next = new Set(prev);
          validNames.forEach(name => {
            if (allMonsters[name]) next.add(name);
          });
          return next;
        });
        
        // setActiveTab("monster");
      } else {
        console.log("未识别到怪物");
        setIdentifiedNames([]);
      }
    } catch (e) {
      console.error("识别失败:", e);
    } finally {
      setIsRecognizing(false);
    }
  };


  // 4. 窗口定位与尺寸控制 (更新界面居中、overlay贴边)
  const lastLayout = useRef<string>("");

  useEffect(() => {
    const syncLayout = async () => {
      const appWindow = getCurrentWindow();
      
      // 1. 获取当前显示器
      const monitor = await currentMonitor(); 
      if (!monitor) return;

      const logicalScale = monitor.scaleFactor;
      currentScale.current = logicalScale;
      
      const pX = monitor.position.x;
      const pY = monitor.position.y;
      const pWidth = monitor.size.width;
      const pHeight = monitor.size.height;

      // 生成当前布局状态的唯一标识
      let targetW = 0;
      let targetH = 0;
      let targetX = 0;
      let targetY = 0;

      if (showVersionScreen) {
        targetW = 500;
        targetH = 350;
        targetX = Math.round(pX / logicalScale + (pWidth / logicalScale - targetW) / 2);
        targetY = Math.round(pY / logicalScale + (pHeight / logicalScale - targetH) / 2);
      } else {
        const screenWLogical = pWidth / logicalScale;
        const screenHLogical = pHeight / logicalScale;
        
        targetW = Math.round(Math.min(expandedWidth, screenWLogical - 20));
        targetH = Math.round(Math.min(isCollapsed ? 45 : expandedHeight, screenHLogical - 40));

        if (hasCustomPosition && lastKnownPosition.current) {
          targetX = Math.round(lastKnownPosition.current.x / logicalScale);
          targetY = Math.round(lastKnownPosition.current.y / logicalScale);
        } else {
          targetX = Math.round((pX + pWidth) / logicalScale - targetW);
          targetY = Math.round(pY / logicalScale); 
        }
      }

      const layoutKey = `${targetW}-${targetH}-${targetX}-${targetY}`;
      if (lastLayout.current === layoutKey) return;
      lastLayout.current = layoutKey;

      try {
        // 先关掉阴影减少重绘压力
        if (appWindow.setShadow) await appWindow.setShadow(false);
        
        // 关键：合并调整，虽然 Tauri V2 依然是分开的 API，
        // 但我们可以判断当前位置是否已经是目标，减少不必要的调用
        const size = await appWindow.innerSize();
        const pos = await appWindow.outerPosition();
        
        const currentW = Math.round(size.width / logicalScale);
        const currentH = Math.round(size.height / logicalScale);
        const currentX = Math.round(pos.x / logicalScale);
        const currentY = Math.round(pos.y / logicalScale);

        if (currentW !== targetW || currentH !== targetH) {
          await appWindow.setSize(new LogicalSize(targetW, targetH));
        }
        if (currentX !== targetX || currentY !== targetY) {
          await appWindow.setPosition(new LogicalPosition(targetX, targetY));
        }
        
        await appWindow.setAlwaysOnTop(true);
        await appWindow.show(); // 确保在位置调整后显示
      } catch (e) { 
        console.error("[Layout] Sync failed:", e); 
        lastLayout.current = ""; 
        // 即使出错也尝试显示，避免应用不可见
        await appWindow.show().catch(() => {});
      }
    };

    const delay = showVersionScreen ? 100 : 20; // 稍微增加延迟让 React 渲染稳定
    const timer = setTimeout(syncLayout, delay); 
    return () => clearTimeout(timer);
  }, [showVersionScreen, expandedWidth, expandedHeight, isCollapsed, hasCustomPosition]);

  // 分离的手动调整逻辑
  const handleResizeWidth = (e: React.MouseEvent) => {
    e.preventDefault();
    const appWindow = getCurrentWindow();
    const startX = e.screenX;
    const startWidth = expandedWidth;
    const scale = currentScale.current;
    
    // 记录起始右边界物理坐标
    const startRightX = lastKnownPosition.current 
      ? lastKnownPosition.current.x + (startWidth * scale)
      : null;
    
    // 如果没有 customPosition，说明在屏幕右上角
    // 这种情况下，targetX = screenWidth - currentWidth
    // 我们也需要获取显示器的信息
    let monitorRect = { x: 0, width: 0 };
    currentMonitor().then(m => {
      if (m) {
        monitorRect.x = m.position.x;
        monitorRect.width = m.size.width;
      }
    });

    const onMouseMove = (moveE: MouseEvent) => {
      const deltaX = startX - moveE.screenX;
      const newWidth = Math.max(200, Math.min(1600, startWidth + deltaX));
      
      // 使用 requestAnimationFrame 确保平滑度且不阻塞
      requestAnimationFrame(async () => {
        const currentHeight = isCollapsed ? 45 : expandedHeight;
        await appWindow.setSize(new LogicalSize(newWidth, currentHeight));
        
        let targetXPhys = 0;
        if (hasCustomPosition && startRightX !== null && lastKnownPosition.current) {
          targetXPhys = startRightX - (newWidth * scale);
          lastKnownPosition.current = { x: targetXPhys, y: lastKnownPosition.current.y };
        } else {
          targetXPhys = monitorRect.x + monitorRect.width - (newWidth * scale);
        }
        await appWindow.setPosition(new LogicalPosition(targetXPhys / scale, (lastKnownPosition.current?.y || 0) / scale));
      });

      setExpandedWidth(newWidth);
    };

    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      localStorage.setItem("plugin-width", expandedWidth.toString());
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  };

  const handleResizeHeight = (e: React.MouseEvent) => {
    e.preventDefault();
    const appWindow = getCurrentWindow();
    const startY = e.screenY;
    const startHeight = expandedHeight;

    const onMouseMove = (moveE: MouseEvent) => {
      const deltaY = moveE.screenY - startY; 
      const newHeight = Math.max(200, Math.min(2560, startHeight + deltaY));
      
      requestAnimationFrame(async () => {
        await appWindow.setSize(new LogicalSize(expandedWidth, newHeight));
      });
      
      setExpandedHeight(newHeight);
    };

    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      localStorage.setItem("plugin-height", expandedHeight.toString());
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  };

  if (showVersionScreen) {
    return (
      <div className="update-screen">
        <div className="update-content">
          <h1>BazaarHelper</h1>
          <div className="update-message">
            {currentVersion ? `当前版本 v${currentVersion}` : "加载中..."}
          </div>
          <div className="update-available-info">
            {updateStatus === "checking" && <span className="status-checking">正在检查更新...</span>}
            {updateStatus === "available" && <span className="status-available">检测到新版本 v{updateAvailable?.version}</span>}
            {updateStatus === "none" && <span className="status-none">已是最新版本</span>}
          </div>
          <button className="skip-btn" onClick={enterApp}>
            进入应用
          </button>
        </div>
      </div>
    );
  }

  return (
    <div 
      className={`overlay ${isCollapsed ? 'collapsed' : 'expanded'}`}
      style={{ 
        '--user-font-size': `${fontSize}px`,
        '--font-scale': fontSize / 16 
      } as any}
      onMouseLeave={(e) => {
        // 如果鼠标离开时按键未松开（可能正在拖动或缩放），则不交还焦点，防止操作中断
        if (e.buttons !== 0) return;
        // 当鼠标划出插件界面时，自动尝试把焦点还给游戏
        invoke("restore_game_focus").catch(() => {});
      }}
    >
      {!isCollapsed && (
        <>
          <div className="resize-handle-width" onMouseDown={handleResizeWidth} title="左右拖动调整宽度" />
          <div className="resize-handle-height" onMouseDown={handleResizeHeight} title="上下拖动调整高度" />
        </>
      )}
      
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

        <button className="settings-btn" onClick={() => setShowSettings(!showSettings)} title="设置">
          <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 15C13.6569 15 15 13.6569 15 12C15 10.3431 13.6569 9 12 9C10.3431 9 9 10.3431 9 12C9 13.6569 10.3431 15 12 15Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33 1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82 1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </button>
        
        <div className="collapse-btn" onClick={() => setIsCollapsed(!isCollapsed)}>
          {isCollapsed ? "展开" : "收起"}
          <span className={`collapse-arrow ${isCollapsed ? 'collapsed' : 'expanded'}`}>▾</span>
        </div>
        
        <button className="close-btn" onClick={() => exit(0)} title="关闭">
          <svg className="close-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M18 6L6 18M6 6L18 18" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </button>
      </div>

      {showSettings && (
        <div className="settings-panel-overlay" onClick={() => setShowSettings(false)}>
          <div className="settings-panel" onClick={e => e.stopPropagation()}>
            <div className="settings-header">
              <h3>应用设置</h3>
              <button className="close-panel-btn" onClick={() => setShowSettings(false)}>×</button>
            </div>
            <div className="settings-content">
              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <label>字体大小: {fontSize}px</label>
                  <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={() => {
                    setFontSize(16);
                    localStorage.setItem("user-font-size", "16");
                  }}>重置</button>
                </div>
                <input 
                  type="range" 
                  min="10" 
                  max="32" 
                  value={fontSize} 
                  onChange={(e) => {
                    const val = parseInt(e.target.value);
                    setFontSize(val);
                    localStorage.setItem("user-font-size", val.toString());
                  }} 
                />
              </div>
              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <label>数据缓存</label>
                  <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={async () => {
                    try {
                      await invoke("clear_monster_cache");
                      setStatusMsg("野怪特征缓存已清空，请手动重启插件以重新加载。");
                    } catch (e) {
                      setStatusMsg("清空失败: " + e);
                    }
                  }}>清空野怪特征缓存</button>
                </div>
              </div>
              
              {statusMsg && (
                <div style={{ 
                  background: 'rgba(255, 205, 25, 0.1)', 
                  border: '1px solid rgba(255, 205, 25, 0.3)', 
                  color: '#ffcd19', 
                  padding: '8px', 
                  fontSize: '12px',
                  borderRadius: '4px',
                  marginTop: '10px',
                  position: 'relative'
                }}>
                  {statusMsg}
                  <button 
                    onClick={() => setStatusMsg(null)}
                    style={{ 
                      position: 'absolute', right: '5px', top: '5px', 
                      background: 'transparent', border: 'none', color: '#ffcd19',
                      cursor: 'pointer', fontSize: '14px'
                    }}>×</button>
                </div>
              )}

              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <label>窗口布局</label>
                  <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={() => {
                    localStorage.removeItem("plugin-width");
                    localStorage.removeItem("plugin-height");
                    setExpandedWidth(400);
                    setExpandedHeight(700);
                    setHasCustomPosition(false);
                  }}>重置宽高与位置</button>
                </div>
              </div>
              <div className="setting-tip">调整后将实时影响所有文字大小</div>

              <div className="setting-divider" style={{ borderTop: '1px solid rgba(255,255,255,0.1)', margin: '15px 0' }}></div>

              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                  <label>版本信息: v{currentVersion}</label>
                  <button 
                    className="bulk-btn" 
                    style={{ 
                      padding: '2px 8px', 
                      opacity: updateStatus === "checking" ? 0.5 : 1,
                      cursor: updateStatus === "checking" ? 'not-allowed' : 'pointer'
                    }} 
                    disabled={updateStatus === "checking" || updateStatus === "downloading"}
                    onClick={async () => {
                      const ENDPOINT = "https://gh.llkk.cc/https://raw.githubusercontent.com/Duangi/BazaarHelper/main/update.json";
                      console.log(`[Update] 用户手动触发更新检查...`);
                      console.log(`[Update] 目标 JSON 地址: ${ENDPOINT}`);
                      setUpdateStatus("checking");

                      try {
                        // 额外做一个手动 Fetch 用于调试，展示 JSON 内容
                        console.log("[Update] 尝试手动 Fetch 远端内容以验证访问...");
                        const response = await fetch(ENDPOINT, { cache: 'no-store' });
                        if (response.ok) {
                          const remoteJson = await response.json();
                          console.log("[Update] 远端 JSON 内容获取成功:", remoteJson);
                          console.log(`[Update] 远端版本: ${remoteJson.version}, 当前本地版本: ${currentVersion}`);
                          
                          if (remoteJson.version === currentVersion) {
                            console.log("[Update] 提示: 版本号完全一致，Tauri check() 必然返回 null");
                          }
                        } else {
                          console.error(`[Update] 远端 JSON 访问失败! 状态码: ${response.status}`);
                        }

                        console.log("[Update] 调用 Tauri 插件 check() 进行正式比对与签名校验...");
                        const u = await check();
                        console.log("[Update] check() 返回对象:", u);
                        
                        if (u) {
                          console.log(`[Update] 手动检查发现新版本: v${u.version}`);
                          setUpdateAvailable(u);
                          setUpdateStatus("available");
                        } else {
                          console.log("[Update] 手动检查结果: 已经是最新版本 (check 返回 null)");
                          setUpdateStatus("none");
                        }
                      } catch (e) {
                        console.error("[Update] 手动检查过程中发生异常:", e);
                        setUpdateStatus("none");
                      }
                    }}
                  >
                    {updateStatus === "checking" ? "检查中..." : "检查更新"}
                  </button>
                </div>

                {updateStatus === "checking" && <div style={{ fontSize: '12px', color: '#999' }}>正在检查远端更新...</div>}
                {updateStatus === "none" && <div style={{ fontSize: '12px', color: '#238636' }}>当前已经是最新版本</div>}
                
                {(updateStatus === "available" || updateStatus === "downloading" || updateStatus === "ready") && (
                  <div style={{ 
                    background: 'rgba(56, 139, 253, 0.15)', 
                    border: '1px solid rgba(56, 139, 253, 0.4)', 
                    padding: '10px', 
                    borderRadius: '6px' 
                  }}>
                    <div style={{ fontSize: '13px', fontWeight: 'bold', marginBottom: '8px', color: '#58a6ff' }}>
                      发现新版本: v{updateAvailable?.version}
                    </div>
                    
                    {updateStatus === "available" && (
                      <button className="bulk-btn" style={{ width: '100%', padding: '6px', background: '#238636', border: 'none' }} onClick={startUpdateDownload}>
                        立即下载更新
                      </button>
                    )}

                    {updateStatus === "downloading" && (
                      <div>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
                          <span>正在下载后台更新...</span>
                          <span>{downloadProgress}%</span>
                        </div>
                        <div style={{ background: 'rgba(255,255,255,0.1)', height: '4px', borderRadius: '2px' }}>
                          <div style={{ background: '#58a6ff', width: `${downloadProgress}%`, height: '100%', borderRadius: '2px', transition: 'width 0.3s' }}></div>
                        </div>
                      </div>
                    )}

                    {updateStatus === "ready" && (
                      <button className="bulk-btn" style={{ width: '100%', padding: '6px', background: '#238636', border: 'none' }} onClick={() => {
                        setIsInstalling(true);
                        setTimeout(() => relaunch(), 1000);
                      }}>
                        下载完成，点击重启安装
                      </button>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {!isCollapsed && (
        <>
          {/* 更新按钮 */}
          <nav className="nav-bar">
            {(["monster", "hand", "stash"] as TabType[]).map(t => (
              <div key={t} className={`nav-item ${activeTab === t ? 'active' : ''}`} onClick={() => setActiveTab(t)}>
                {t === 'monster' ? '野怪一览' : t === 'hand' ? '手牌' : '仓库'}
              </div>
            ))}
          </nav>

          <div className="scroll-area">
            <div className="items" ref={wrapRef}>
              {activeTab === "monster" ? (
                <>
                  <div className="monster-controls">
                    <div className="day-tabs">
                      <div className="day-row">
                        {["Day 1", "Day 2", "Day 3", "Day 4", "Day 5"].map(d => (
                          <div key={d} className={`day-tab ${selectedDay === d ? 'active' : ''}`} onClick={() => {
                            setSelectedDay(d);
                            const dayNum = parseInt(d.split(" ")[1]);
                            handleDayChange(dayNum);
                          }}>{d}</div>
                        ))}
                      </div>
                      <div className="day-row">
                        {["Day 6", "Day 7", "Day 8", "Day 9", "Day 10+"].map(d => (
                          <div key={d} className={`day-tab ${selectedDay === d ? 'active' : ''}`} onClick={() => {
                            setSelectedDay(d);
                            // 将 Day 10+ 映射为 10，其余提取数字部分
                            const dayNum = d === "Day 10+" ? 10 : parseInt(d.split(" ")[1]);
                            handleDayChange(dayNum);
                          }}>{d}</div>
                        ))}
                      </div>
                    </div>

                    <div className="search-container">
                      <button 
                        className="manual-recognition-btn" 
                        onClick={handleManualRecognition}
                        disabled={isRecognizing || !templateLoading.is_complete}
                        style={{ width: '100%' }}
                      >
                        {isRecognizing ? "识别中..." : "🎯 识别画面中的怪物"}
                      </button>
                    </div>

                    {!templateLoading.is_complete && templateLoading.total > 0 && (
                      <div className="loading-progress">
                        <div className="progress-text">加载怪物模板: {templateLoading.loaded}/{templateLoading.total}</div>
                        <div className="progress-bar"><div className="progress-fill" style={{ width: `${templateLoading.total > 0 ? (templateLoading.loaded / templateLoading.total * 100) : 0}%` }} /></div>
                      </div>
                    )}
                  </div>

                  <div className="monster-list-v2">
                    {manualMonsters.map((m, i) => {
                      const isIdentified = identifiedNames.includes(m.name_zh);
                      const isExpanded = expandedMonsters.has(m.name_zh);
                      
                      return (
                        <div key={i} className={`monster-card-v2 ${isIdentified ? 'identified-glow' : ''} ${isExpanded ? 'expanded' : ''}`} onClick={() => toggleMonsterExpand(m.name_zh)}>
                          <div className="monster-header-v2">
                            <div className="avatar-wrap">
                              <img src={m.displayImg} className="monster-avatar-v2" alt="" />
                            </div>
                            <div className="monster-info-v2">
                              <div className="monster-name-zh">
                                {m.name_zh}
                                {isIdentified && <span className="id-badge">MATCH</span>}
                              </div>
                              <div className="monster-health">❤️ {m.health?.toString() || m.health}</div>
                            </div>
                            <div className="monster-available-tag">
                              {m.available}
                              <span className="expand-indicator" style={{ marginLeft: '8px' }}>{isExpanded ? '▴' : '▾'}</span>
                            </div>
                          </div>
                        
                        {isExpanded && (
                          <div className="monster-assets-grid">
                            <div className="assets-section">
                              <div className="section-title">技能 (Skills)</div>
                              {m.skills?.map((s, idx) => <div key={idx}>{renderTierInfo(s)}</div>)}
                            </div>
                            <div className="assets-section">
                              <div className="section-title">物品 (Items)</div>
                              {m.items?.map((it, idx) => <div key={idx}>{renderTierInfo(it)}</div>)}
                            </div>
                          </div>
                        )}
                      </div>
                    )
                  })}
                  {manualMonsters.length === 0 && <div className="empty-tip">该天数下暂无怪物数据</div>}
                </div>
              </>
            ) : (
                <div className="card-list">
                  {getSortedItems(activeTab === "hand" ? syncData.hand_items : syncData.stash_items).map(item => {
                    const isExpanded = expandedItems.has(item.uuid);
                    const tierClass = item.tier.split(' / ')[0].toLowerCase();
                    const heroZh = item.heroes[0]?.split(' / ')[1] || item.heroes[0] || "通用";

                    return (
                      <div key={item.uuid} className={`item-card-container ${isExpanded ? 'expanded' : ''}`} onClick={() => toggleExpand(item.uuid)}>
                        <div className={`item-card tier-${tierClass}`}>
                          <div className="card-left">
                            <div className="image-box">
                              <img src={item.displayImg} alt={item.name} />
                            </div>
                          </div>

                          <div className="card-center">
                            <div className="name-line">
                              <span className="name-cn">{item.name_cn}</span>
                              <span className="tier-label">{item.tier.split(' / ')[0].toUpperCase()}+</span>
                            </div>
                            <div className="tags-line">
                              {item.processed_tags.slice(0, 3).map(t => (
                                <span key={t} className="tag-badge">{t}</span>
                              ))}
                            </div>
                          </div>

                          <div className="card-right">
                            <div className="top-right-group">
                              <span className="hero-badge">{heroZh}</span>
                              <div 
                                className={`pin-btn ${pinnedItems.has(item.uuid) ? 'active' : ''}`}
                                onClick={(e) => togglePin(item.uuid, e)}
                              >
                                {pinnedItems.has(item.uuid) ? "📌" : "📍"}
                              </div>
                            </div>
                            <div className="expand-chevron">{isExpanded ? '▴' : '▾'}</div>
                          </div>
                        </div>

                        {isExpanded && (
                          <div className="item-details-v2">
                            {item.cooldown !== undefined && item.cooldown > 0 && (
                              <div className="details-left">
                                <div className="cd-display">
                                  <div className="cd-value">{item.cooldown.toFixed(1)}</div>
                                  <div className="cd-unit">秒</div>
                                </div>
                              </div>
                            )}
                            <div className="details-right">
                              {item.skills.map((s, idx) => (
                                <div key={idx} className="skill-item">
                                  {renderText(s)}
                                </div>
                              ))}
                              {item.enchantments.map((enc, idx) => (
                                <div key={idx} className="enchant-item">
                                  {renderText(enc)}
                                </div>
                              ))}
                              {item.description && (
                                <div className="description-text">
                                  {renderText(item.description)}
                                </div>
                              )}
                            </div>
                          </div>
                        )}
                      </div>
                    );
                  })}
                  {(activeTab === "hand" ? syncData.hand_items : syncData.stash_items).length === 0 && (
                    <div className="empty-tip">当前暂无数据，请在游戏中操作相应卡牌</div>
                  )}
                </div>
              )}
            </div>
          </div>
        </>
      )}

      {/* 正在安装层 */}
      {isInstalling && (
        <div style={{
          position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
          background: '#292521', color: '#ffcd19',
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          zIndex: 9999
        }}>
          <div className="version-logo">BH</div>
          <div style={{ fontSize: '18px', marginBottom: '10px' }}>正在启动更新安装程序...</div>
          <div style={{ fontSize: '12px', opacity: 0.7 }}>程序即将自动重启以完成安装</div>
          <div className="loader" style={{ marginTop: '20px' }}></div>
        </div>
      )}
    </div>
  );
}