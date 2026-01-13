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
  const [probabilities, setProbabilities] = useState<any>(null);

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
  const togglePin = (itemId: string) => {
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

  // 1. 记忆宽度 (默认 680px 保证内容不拥挤)
  const [expandedWidth, setExpandedWidth] = useState(() => {
    const saved = localStorage.getItem("plugin-width");
    return saved ? parseInt(saved, 10) : 400;
  });

  // 图片路径处理函数
  const getImg = async (path: string | null | undefined) => {
    if (!path) return "";
    try {
      const fullPath = await resolveResource(`resources/${path}`);
      return convertFileSrc(fullPath);
    } catch { return ""; }
  };
  
  // 启动时显示版本信息
  useEffect(() => {
    const showVersionInfo = async () => {
      try {
        const appVersion = await getVersion();
        setCurrentVersion(appVersion);
        
        // 3秒后自动进入应用
        setTimeout(() => {
          setShowVersionScreen(false);
        }, 3000);
      } catch (error) {
        console.error("获取版本信息失败:", error);
        // 2秒后自动进入
        setTimeout(() => {
          setShowVersionScreen(false);
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
    
    const setupListeners = async () => {
      // 1. 怪物识别触发
      unlistenMonster = await listen<number | null>('trigger-monster-recognition', async (event) => {
        console.log("收到自动识别触发事件, Day:", event.payload);
        const day = event.payload;
        if (day) {
          const dayLabel = day >= 10 ? "Day 10+" : `Day ${day}`;
          setSelectedDay(dayLabel);
        }
        setTimeout(async () => {
           // 传入事件带过来的天数
           await handleAutoRecognition(day);
        }, 500);
      });

      // 2. 天数更新
      unlistenDay = await listen<number>('day-update', (event) => {
        console.log("收到天数更新事件:", event.payload);
        const d = event.payload;
        setCurrentDay(d);
        // 同步切换 UI 上的天数按钮选中状态
        const dayLabel = d >= 10 ? "Day 10+" : `Day ${d}`;
        setSelectedDay(dayLabel);
      });
    };
    
    setupListeners();
    
    return () => {
      if (unlistenMonster) unlistenMonster();
      if (unlistenDay) unlistenDay();
    };
  }, [currentDay]);

  // 监听窗口移动事件，检测用户拖拽
  useEffect(() => {
    const setupMoveListener = async () => {
      // 先获取一次缩放比例存起来
      const monitor = await currentMonitor();
      if (monitor) {
        currentScale.current = monitor.scaleFactor;
      }

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
      unlistenPromise.then(unlisten => unlisten());
      if (moveDebounceTimer.current) {
        clearTimeout(moveDebounceTimer.current);
      }
    };
  }, []); // 只在组件挂载时运行一次

  useEffect(() => {
    console.log("设置事件监听器...");
    const setupListener = async () => {
      try {
        const unlisten = await listen<SyncPayload>("sync-items", async (event) => {
          console.log("收到同步事件:", event.payload);
          
          const payload = event.payload;

          const processItems = (items: ItemData[]) => 
            Promise.all(items.map(async (i) => ({ ...i, displayImg: await getImg(i.image) })));

          // 修正字段名匹配
          const [hand, stash] = await Promise.all([
            processItems(payload.hand_items || []),
            processItems(payload.stash_items || [])
          ]);

          console.log("处理后的数据:", { hand: hand.length, stash: stash.length });
          setSyncData(prev => ({ ...prev, hand, stash }));
        });
        console.log("事件监听器设置完成");
        
        // 延迟500ms后主动获取一次数据，防止错过初始事件
        setTimeout(async () => {
          try {
            console.log("主动获取当前数据...");
            // 这里暂时用一个虚假的调用，后面再实现
            // const data = await invoke<SyncPayload>("get_current_items");
            console.log("等待后端实现获取当前数据的接口");
          } catch (error) {
            console.log("获取当前数据失败:", error);
          }
        }, 500);
        
        return unlisten;
      } catch (error) {
        console.error("设置事件监听器失败:", error);
        return () => {};
      }
    };

    const l = setupListener();
    return () => { l.then(f => f()).catch(console.error); };
  }, []);


  // 加载概率数据
  useEffect(() => {
    const loadProbabilities = async () => {
      try {
        const res = await invoke("get_card_probabilities");
        console.log("加载概率数据成功:", res);
        setProbabilities(res);
      } catch (e) {
        console.error("加载概率数据失败:", e);
      }
    };
    loadProbabilities();
  }, []);

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
    const getProgressionText = (line: string, lineIdx: number) => {
      const tierSequence = ['bronze', 'silver', 'gold', 'diamond'];
      const activeTiers = tierSequence
        .map(t => ({ tier: t, data: item.tiers[t] }))
        .filter(t => t.data !== null);
      
      const numRegex = /(\d+(\.\d+)?%?)/g;
      const matches = [...line.matchAll(numRegex)];
      
      if (matches.length > 0 && activeTiers.length > 1) {
        let lastIndex = 0;
        const parts = [];
        matches.forEach((match, mIdx) => {
          parts.push(line.substring(lastIndex, match.index));
          parts.push(
            <span key={mIdx} className="progression-inline-values">
              {activeTiers.map((at, i) => {
                const atMatches = [...(at.data!.description[lineIdx] || "").matchAll(numRegex)];
                const val = atMatches[mIdx] ? atMatches[mIdx][0] : match[0];
                return (
                  <span key={at.tier}>
                    <span className={`val-${at.tier}`}>{val}</span>
                    {i < activeTiers.length - 1 && <span className="upgrade-arrow">»</span>}
                  </span>
                );
              })}
            </span>
          );
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
            {finalData.cd && <div className="sub-item-cd">⏳ {finalData.cd}</div>}
          </div>
        </div>
        <div className="sub-item-desc">
          {finalData.description.map((d, i) => (
            <div key={i} className="desc-line">
              {isProgressionActive ? getProgressionText(d, i) : formatDescription(d)}
            </div>
          ))}
          {finalData.extra_description?.map((d, i) => (
            <div key={`extra-${i}`} className="desc-line extra-desc">
              {isProgressionActive ? getProgressionText(d, i) : formatDescription(d)}
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
      const currentWidth = expandedWidth;
      const currentHeight = isCollapsed ? 45 : (screenHeight - 200);

      let targetX = 0;
      let targetY = 0;

      if (hasCustomPosition && lastKnownPosition.current) {
        // 如果用户拖过，使用记忆的物理坐标并实时转换
        // 【关键修复】用当前屏幕的实时缩放比转换物理坐标
        targetX = lastKnownPosition.current.x / scale;
        targetY = lastKnownPosition.current.y / scale;
      } else {
        // 默认逻辑：贴在当前屏幕的最右侧
        // 公式：屏幕起始X + 屏幕宽度 - 窗口宽度
        targetX = screenX + screenWidth - currentWidth;
        // 贴顶：屏幕起始Y
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
  }, [showVersionScreen, expandedWidth, isCollapsed, hasCustomPosition]);

  // 手动调整宽度逻辑
  const handleResize = (e: React.MouseEvent) => {
    const startX = e.screenX;
    const startWidth = expandedWidth;
    const onMouseMove = (moveE: MouseEvent) => {
      const delta = startX - moveE.screenX;
      const newWidth = Math.max(400, Math.min(1000, startWidth + delta));
      setExpandedWidth(newWidth);
      localStorage.setItem("plugin-width", newWidth.toString());
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
          <button className="skip-btn" onClick={() => setShowVersionScreen(false)}>
            进入应用
          </button>
        </div>
      </div>
    );
  }

  return (
    <div 
      className={`overlay ${isCollapsed ? 'collapsed' : 'expanded'}`}
    >
      {!isCollapsed && <div className="resize-handle" onMouseDown={handleResize} />}
      
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
        
        {/* 暂时隐藏检查更新按钮
        <button className="top-update-btn" onClick={handleUpdateClick} title="检查更新">
          <svg className="update-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M21 10C21 10 18.995 7.26822 17.3662 5.63824C15.7373 4.00827 13.4864 3 11 3C6.02944 3 2 7.02944 2 12C2 16.9706 6.02944 21 11 21C15.1031 21 18.5649 18.2543 19.6482 14.5M21 10V4M21 10H15" 
                  stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
          {updateAvailable && <span className="update-badge"></span>}
        </button>
        */}
        
        <div className="collapse-btn" onClick={() => setIsCollapsed(!isCollapsed)}>
          {isCollapsed ? "展开插件" : "收起插件"}
          <span className={`collapse-arrow ${isCollapsed ? 'collapsed' : 'expanded'}`}>▾</span>
        </div>
        
        <button className="close-btn" onClick={() => exit(0)} title="关闭">
          <svg className="close-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M18 6L6 18M6 6L18 18" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </button>
      </div>

      {!isCollapsed && (
        <>
          {/* 更新按钮 */}
          {/* {updateAvailable && (
            <div className="update-notification">
              <button className="update-notify-btn" onClick={performUpdate}>
                🔔 发现新版本 v{updateVersion}
              </button>
            </div>
          )} */}
          
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

                    <div className="card-probabilities" style={{ 
                      margin: '10px 0', 
                      padding: '8px 0', 
                      background: 'rgba(255, 255, 255, 0.03)', 
                      borderRadius: '8px', 
                      border: '1px solid rgba(255, 255, 255, 0.1)',
                      minHeight: '34px'
                    }}>
                      {(() => {
                         if (!probabilities) return <div style={{ textAlign: 'center', fontSize: '12px', color: '#666' }}>加载概率中...</div>;
                         let dayKey = selectedDay || "Day 1";
                         if (dayKey === "Day 10+") dayKey = "Day 9+";
                         const prob = probabilities[dayKey] || probabilities["Day 9+"];
                         if (!prob) return null;
                         return (
                           <div className="prob-row" style={{ display: 'flex', justifyContent: 'center', gap: '22px', fontSize: '15px', fontWeight: '900', textShadow: '1px 1px 2px rgba(0,0,0,0.8)' }}>
                             <span style={{ color: '#CD7F32' }}>{(prob.bronze * 100).toFixed(0)}%</span>
                             <span style={{ color: '#C0C0C0' }}>{(prob.silver * 100).toFixed(0)}%</span>
                             <span style={{ color: '#FFD700' }}>{(prob.gold * 100).toFixed(0)}%</span>
                             <span style={{ color: '#B9F2FF' }}>{(prob.diamond * 100).toFixed(0)}%</span>
                           </div>
                         );
                      })()}
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
                      return (
                        <div key={i} className={`monster-card-v2 ${isIdentified ? 'identified-glow' : ''}`}>
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
                            <div className="monster-available-tag">{m.available}</div>
                          </div>
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
                      </div>
                    )
                  })}
                  {manualMonsters.length === 0 && <div className="empty-tip">该天数下暂无怪物数据</div>}
                </div>
              </>
            ) : (
                getSortedItems(syncData[activeTab]).map(item => (
                  <div key={item.id} className="content-wrap">
                    <div className="left-section">
                      <div 
                        className={pinnedItems.has(item.id) ? "pin-btn pinned" : "pin-btn"}
                        onClick={() => togglePin(item.id)}
                        title={pinnedItems.has(item.id) ? "取消置顶" : "置顶"}>
                        {pinnedItems.has(item.id) ? "📌" : "📍"}
                      </div>
                      <div className="left-image"><img src={item.displayImg} alt="" /></div>
                      <div className="item-name">{item.name_zh}</div>
                    </div>
                    <div className="effect-table">
                      {item.enchantments?.map((enc: any) => {
                        const colorKey = ID_TO_COLOR[enc.id] || "tag";
                        return (
                          <div key={enc.id} className="effect-cell">
                            {/* 强行应用颜色并增加间距 */}
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