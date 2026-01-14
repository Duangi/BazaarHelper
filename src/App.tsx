import { useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalPosition, LogicalSize, currentMonitor } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import { getVersion } from '@tauri-apps/api/app';
import "./App.css";

import { exit } from '@tauri-apps/plugin-process';

// --- 接口定义 ---
interface Enchantment { 
  id: string; 
  name: string; 
  description: string; 
}

interface ItemData { 
  id: string; 
  name_zh: string; 
  image: string; 
  enchantments: Enchantment[] | null; 
  displayImg?: string; 
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
  available: string;
  health: number;
  skills: MonsterSubItem[]; 
  items: MonsterSubItem[]; 
  image: string;
  displayImg?: string; 
}

interface SyncPayload { 
  hand_items: ItemData[]; 
  stash_items: ItemData[]; 
}

type TabType = "hand" | "stash" | "monster";

// --- 颜色映射：对应 JSON 里的 id 和 CSS 里的变量 ---
const ID_TO_COLOR: Record<string, string> = {
  heavy: "slow",
  golden: "golden",
  icy: "freeze",
  turbo: "haste",
  shielded: "shield",
  restorative: "heal",
  toxic: "poison",
  fiery: "burn",
  shiny: "tag",
  deadly: "damage",
  radiant: "freeze",
  obsidian: "lifesteal"
};

export default function App() {
  const [activeTab, setActiveTab] = useState<TabType>("monster");
  const [syncData, setSyncData] = useState<Record<TabType, any[]>>({ hand: [], stash: [], monster: [] });
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

  // 置顶/取消置顶功能
  const togglePin = (itemId: string, e: React.MouseEvent) => {
    e.stopPropagation(); // 防止触发展开/收起
    setPinnedItems(prev => {
      const newPinned = new Map(prev);
      if (newPinned.has(itemId)) {
        newPinned.delete(itemId);
      } else {
        setPinnedCounter(c => c + 1);
        newPinned.set(itemId, pinnedCounter + 1);
      }
      return newPinned;
    });
  };

  const toggleExpand = (itemId: string) => {
    setExpandedItems(prev => {
      const next = new Set(prev);
      if (next.has(itemId)) next.delete(itemId);
      else next.add(itemId);
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

 // 获取当前 Day 并定期刷新
 useEffect(() => {
   let mounted = true;
   const fetchDay = async () => {
     try {
       const d = await invoke<number>("get_current_day", { hours_per_day: 6, retro: true });
       if (mounted) {
         if (d !== currentDay) {
           setCurrentDay(d);
           // 初始加载或检测到变化时，更新选中的标签
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
      const aPin = pinnedItems.get(a.id);
      const bPin = pinnedItems.get(b.id);
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
    setShowVersionScreen(false);
    invoke("start_template_loading").catch(console.error);
  };

  // 启动时显示版本信息
  useEffect(() => {
    const showVersionInfo = async () => {
      try {
        const appVersion = await getVersion();
        setCurrentVersion(appVersion);
        
        // 3秒后自动进入应用
        setTimeout(() => {
          enterApp();
        }, 3000);
      } catch (error) {
        console.error("获取版本信息失败:", error);
        // 2秒后自动进入
        setTimeout(() => {
          enterApp();
        }, 2000);
      }
    };
    
    showVersionInfo();
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

        setSyncData(prev => ({ ...prev, hand, stash }));
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
    const monstersOnDay = Object.values(allMonsters).filter(m => m.available === day);
    
    // 根据识别结果进行排序
    const sorted = [...monstersOnDay].sort((a, b) => {
      const indexA = identifiedNames.indexOf(a.name_zh); // 改为使用中文名匹配 backend 的 key
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
      skills: await Promise.all(m.skills.map(async s => ({ 
        ...s, 
        displayImg: await getImg(s.image) 
      }))),
      items: await Promise.all(m.items.map(async i => ({ 
        ...i, 
        displayImg: await getImg(i.image) 
      })))
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
    
    if (item.current_tier) {
      currentTier = item.current_tier.toLowerCase();
    } else {
      // 检查 tiers 对象里有哪些 key，有些数据可能直接把数据塞到了特定的 key 里
      const availableTiers = Object.keys(item.tiers);
      if (availableTiers.length > 0) {
        // 如果只有一个 key 或者包含特定的 key
        if (availableTiers.includes("bronze")) currentTier = "bronze";
        else if (availableTiers.includes("silver")) currentTier = "silver";
        else if (availableTiers.includes("gold")) currentTier = "gold";
        else currentTier = availableTiers[0]; // 实在不行拿第一个
      }
    }

    const tierData = item.tiers[currentTier];
    // 如果该级位没数据，显示第一个有数据的级位
    const finalData = tierData || Object.values(item.tiers).find(t => t !== null);
    
    // --- 升级效果合并逻辑 (用于显示在卡片上或悬浮框) ---
    const getProgressionText = (line: string, lineIdx: number, field: 'description' | 'extra_description' = 'description') => {
      const tierSequence = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
      const activeTiers = tierSequence
        .map(t => ({ tier: t, data: item.tiers[t] }))
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
                  .map(t => ({ tier: t, data: item.tiers[t] }))
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
      const results = await invoke("recognize_monsters_from_screenshot", { day }) as any[];
      if (results && results.length > 0) {
        const names = new Array(3).fill("");
        results.forEach(r => {
          if (r.position >= 1 && r.position <= 3) names[r.position - 1] = r.name;
        });
        const validNames = names.filter(n => n !== "");
        console.log(`[Recognition Success] Found: ${validNames.join(', ')}`);
        setIdentifiedNames(validNames);
        // 不再自动跳转到 monster tab
        // setActiveTab("monster");
      }
    } catch (e) {
      console.error("自动识别失败:", e);
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
  useEffect(() => {
    const syncLayout = async () => {
      const appWindow = getCurrentWindow();
      
      // 1. 获取当前显示器
      const monitor = await currentMonitor(); 
      if (!monitor) return;

      const scale = monitor.scaleFactor;
      currentScale.current = scale;
      
      // 获取该显示器的物理位置和尺寸
      // 注意：多屏环境下，monitor.position.x 可能不是 0
      const screenX = monitor.position.x / scale;
      const screenY = monitor.position.y / scale;
      const screenWidth = monitor.size.width / scale;
      const screenHeight = monitor.size.height / scale;

      if (appWindow.setShadow) await appWindow.setShadow(false);

      // --- 场景 A：显示版本号界面 (屏幕正中央) ---
      if (showVersionScreen) {
        const updateWidth = 500;
        const updateHeight = 350;
        
        // 计算相对于当前屏幕的居中坐标
        const centerX = screenX + (screenWidth - updateWidth) / 2;
        const centerY = screenY + (screenHeight - updateHeight) / 2;
        
        try {
          await appWindow.setSize(new LogicalSize(updateWidth, updateHeight));
          await appWindow.setPosition(new LogicalPosition(centerX, centerY));
          await appWindow.setAlwaysOnTop(true);
        } catch (e) { console.error(e); }
        return;
      }

      // --- 场景 B：显示主插件界面 (默认右上角) ---
      const currentWidth = Math.min(expandedWidth, screenWidth - 20); // 留一点边距
      const currentHeight = Math.min(isCollapsed ? 45 : expandedHeight, screenHeight - 40); // 留出任务栏空间

      let targetX = 0;
      let targetY = 0;

      if (hasCustomPosition && lastKnownPosition.current) {
        // 如果用户拖过，使用记忆的物理坐标并实时转换
        targetX = lastKnownPosition.current.x / scale;
        targetY = lastKnownPosition.current.y / scale;
      } else {
        // 默认逻辑：贴在当前屏幕的最右侧
        targetX = screenX + screenWidth - currentWidth;
        targetY = screenY; 
      }

      try {
        await appWindow.setSize(new LogicalSize(currentWidth, currentHeight));
        await appWindow.setPosition(new LogicalPosition(targetX, targetY));
        await appWindow.setAlwaysOnTop(true);
      } catch (e) { console.error(e); }
    };

    // 防抖
    const timer = setTimeout(syncLayout, 50);
    return () => clearTimeout(timer);
  }, [showVersionScreen, expandedWidth, expandedHeight, isCollapsed, hasCustomPosition]);

  // 分离的手动调整逻辑
  const handleResizeWidth = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.screenX;
    const startWidth = expandedWidth;
    
    // 如果已有自定义位置，记录起始的右边界物理坐标，以便后续维持贴右
    const startRightX = lastKnownPosition.current ? lastKnownPosition.current.x + (startWidth * currentScale.current) : null;

    const onMouseMove = (moveE: MouseEvent) => {
      const deltaX = startX - moveE.screenX;
      // 向左拖动 deltaX 为正，宽度增加
      const newWidth = Math.max(200, Math.min(1600, startWidth + deltaX));
      setExpandedWidth(newWidth);
      
      // 如果用户之前移动过窗口，我们需要更新其记录的位置，使其看起来是向左延伸（保持右边界不动）
      if (hasCustomPosition && startRightX !== null && lastKnownPosition.current) {
        const newPhysicalWidth = newWidth * currentScale.current;
        const newX = startRightX - newPhysicalWidth;
        lastKnownPosition.current = { x: newX, y: lastKnownPosition.current.y };
      }
      
      localStorage.setItem("plugin-width", newWidth.toString());
    };
    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  };

  const handleResizeHeight = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.screenY;
    const startHeight = expandedHeight;
    const onMouseMove = (moveE: MouseEvent) => {
      const deltaY = moveE.screenY - startY; // 往下拖变高
      const newHeight = Math.max(200, Math.min(2560, startHeight + deltaY));
      setExpandedHeight(newHeight);
      localStorage.setItem("plugin-height", newHeight.toString());
    };
    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
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

        <button className="settings-btn" onClick={() => setShowSettings(true)} title="设置">
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
                              <div className="monster-health">❤️ {m.health}</div>
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
                              {m.skills.map((s, idx) => <div key={idx}>{renderTierInfo(s)}</div>)}
                            </div>
                            <div className="assets-section">
                              <div className="section-title">物品 (Items)</div>
                              {m.items.map((it, idx) => <div key={idx}>{renderTierInfo(it)}</div>)}
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
                getSortedItems(syncData[activeTab]).map(item => (
                  <div key={item.id} className={`content-wrap ${expandedItems.has(item.id) ? 'expanded' : ''}`} onClick={() => toggleExpand(item.id)}>
                    <div className="item-main-info">
                      <div className="left-section">
                        <div 
                          className={pinnedItems.has(item.id) ? "pin-btn pinned" : "pin-btn"}
                          onClick={(e) => togglePin(item.id, e)}
                          title={pinnedItems.has(item.id) ? "取消置顶" : "置顶"}>
                          {pinnedItems.has(item.id) ? "📌" : "📍"}
                        </div>
                        <div className="left-image"><img src={item.displayImg} alt="" /></div>
                        <div className="item-name">{item.name_zh}</div>
                      </div>
                      <div className="expand-indicator">{expandedItems.has(item.id) ? '▴' : '▾附魔'}</div>
                    </div>
                    
                    {expandedItems.has(item.id) && (
                      <div className="effect-table">
                        {item.enchantments?.map((enc: any) => {
                          const colorKey = ID_TO_COLOR[enc.id] || "tag";
                          return (
                            <div key={enc.id} className="effect-cell">
                              <strong className="effect-label" style={{ 
                                  color: `var(--c-${colorKey})`, 
                                  borderLeft: `3px solid var(--c-${colorKey})` 
                              }}>
                                {enc.name}
                              </strong>
                              <span className="effect-desc">{enc.description}</span>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                ))
              )}
              {syncData[activeTab].length === 0 && activeTab !== "monster" && <div className="empty-tip">等待游戏日志更新...</div>}
            </div>
          </div>
        </>
      )}
    </div>
  );
}