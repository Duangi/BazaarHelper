import * as React from "react";
import { useEffect, useRef, useState, Fragment } from "react";
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
  available_tiers: string;
  size?: string;
  tags: string;
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
  id?: string;
  name: string; 
  name_en?: string;
  tier?: string;
  current_tier?: string;
  tags?: string[];
  tiers: Record<string, TierInfo | null>;
  image: string; 
  displayImg?: string; 
  size?: string;
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

type TabType = "hand" | "stash" | "monster" | "card";

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
  const [announcement, setAnnouncement] = useState(""); // 公告内容
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set()); // 手牌/仓库点击展开附魔
  const [expandedMonsters, setExpandedMonsters] = useState<Set<string>>(new Set()); // 野怪点击展开
  const [recognizedCards, setRecognizedCards] = useState<ItemData[]>([]); // 识别出的卡牌列表 (Top 3)
  const [isRecognizingCard, setIsRecognizingCard] = useState(false); // 是否正在识别卡牌

  // 图片路径缓存，避免重复解析
  const [imgCache] = useState<Map<string, string>>(new Map());

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const appWindow = getCurrentWindow(); // 获取当前窗口实例
  
  // 自定义位置状态，用于记忆用户是否手动拖拽过窗口
  const [hasCustomPosition, setHasCustomPosition] = useState(false);
  const lastKnownPosition = useRef<{ x: number; y: number } | null>(null);
  
  // 存储当前屏幕缩放比例，用于坐标转换
  const currentScale = useRef(1);

  // 新增：识别热键状态
  const [detectionHotkey, setDetectionHotkey] = useState<number | null>(null);
  const [cardDetectionHotkey, setCardDetectionHotkey] = useState<number | null>(null);
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [isRecordingCardHotkey, setIsRecordingCardHotkey] = useState(false);
  
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
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // 监听扫描错误
  useEffect(() => {
    const unlisten = listen<string>("scan-error", (event) => {
      console.error("[Backend Error]", event.payload);
      setErrorMessage(`识别错误: ${event.payload}`);
      // 3秒后自动清除
      setTimeout(() => setErrorMessage(null), 5000);
    });
    return () => {
      unlisten.then(f => f());
    };
  }, []);

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

  const handleRecognizeCard = async (switchTab = false) => {
    if (isRecognizingCard) return;
    if (switchTab) setActiveTab("card");
    setIsRecognizingCard(true);
    setErrorMessage(null);
    try {
      const results = await invoke<any[] | null>("recognize_card_at_mouse");
      if (results && results.length > 0) {
        const fullInfos: ItemData[] = [];
        for (const res of results) {
          const itemInfo = await invoke<ItemData | null>("get_item_info", { id: res.id });
          if (itemInfo) {
            const imgUrl = await getImg(`images/${itemInfo.uuid || itemInfo.name}.webp`);
            fullInfos.push({ ...itemInfo, displayImg: imgUrl });
          }
        }
        
        if (fullInfos.length > 0) {
          setRecognizedCards(fullInfos);
          // 自动展开识别到的所有前三项，方便用户查看
          setExpandedItems(prev => {
            const next = new Set(prev);
            fullInfos.forEach(info => next.add(info.uuid));
            return next;
          });
          setStatusMsg(`识别成功: 找到 ${fullInfos.length} 个匹配项`);
          setTimeout(() => setStatusMsg(null), 2000);
        } else {
          setErrorMessage("识别到了卡牌，但没能在数据库中找到对应信息");
        }
      } else {
        setErrorMessage("未能识别到鼠标下的卡牌。请确保鼠标指向卡牌中心。");
      }
    } catch (e: any) {
      console.error(e);
      setErrorMessage(`卡牌识别执行出错: ${e}`);
    } finally {
      setIsRecognizingCard(false);
      setTimeout(() => setErrorMessage(null), 3000);
    }
  };

  const renderText = (text: any) => {
    if (!text || typeof text !== 'string') return null;
    
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

  // 辅助函数：将虚拟键码转换为可读文本
  const getHotkeyLabel = (code: number) => {
    if (code >= 65 && code <= 90) return `Key ${String.fromCharCode(code)}`;
    if (code >= 48 && code <= 57) return `Key ${code - 48}`;
    if (code >= 112 && code <= 123) return `F${code - 111}`;
    
    switch(code) {
      case 1: return "鼠标左键";
      case 2: return "鼠标右键";
      case 4: return "鼠标中键";
      case 5: return "鼠标侧键1 (后退)";
      case 6: return "鼠标侧键2 (前进)";
      case 8: return "BackSpace";
      case 9: return "Tab";
      case 13: return "Enter";
      case 16: return "Shift";
      case 17: return "Ctrl";
      case 18: return "Alt";
      case 20: return "CapsLock";
      case 27: return "Esc";
      case 32: return "Space";
      case 33: return "PageUp";
      case 34: return "PageDown";
      case 35: return "End";
      case 36: return "Home";
      case 37: return "Left";
      case 38: return "Up";
      case 39: return "Right";
      case 40: return "Down";
      case 45: return "Insert";
      case 46: return "Delete";
      case 192: return "~";
    }
    return `Unknown (${code})`;
  };

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

        // 获取公告内容 (从 GitHub 代理)
        const fallbackNotice = "🧠 脑子是用来构筑的，数据交给小抄记。\n\n💡 这只是个免费的记牌小工具，又不是考研资料，谁要是敢收你的费，请反手给他一个大逼兜！👊\n\n🍖 本小抄由 B站@这是李Duang啊 免费发放，付费获取的同学请立刻退款买排骨吃！";
        try {
          const res = await fetch("https://gh.llkk.cc/https://raw.githubusercontent.com/Duangi/BazaarHelper/main/update.json");
          if (res.ok) {
            const data = await res.json();
            if (data.notes) {
              setAnnouncement(data.notes + "\n\n------------------\n\n" + fallbackNotice);
            } else {
              setAnnouncement(fallbackNotice);
            }
          } else {
            setAnnouncement(fallbackNotice);
          }
        } catch (err) {
          console.error("[App] 获取公告失败:", err);
          setAnnouncement(fallbackNotice);
        }
      } catch (error) {
        console.error("[App] 初始化加载更新失败:", error);
        setUpdateStatus("none");
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
    let unlistenAutoJump: any = null;
    let unlistenCard: any = null;
    
    const setupListeners = async () => {
      // 0. 加载热键配置
      invoke<number | null>("get_detection_hotkey").then(setDetectionHotkey);
      invoke<number | null>("get_card_detection_hotkey").then(setCardDetectionHotkey);

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

      // 1.1 卡牌识别触发 (热键)
      unlistenCard = await listen('hotkey-detect-card', () => {
        console.log("收到卡牌识别触发事件");
        handleRecognizeCard(true); // 自动识别并跳转
      });

      // 1.5 自动识别并跳转事件 (auto-jump-to-monster)
      unlistenAutoJump = await listen<{ day: number; monster_name: string }>('auto-jump-to-monster', async (event) => {
          const { day, monster_name } = event.payload;
          console.log(`收到自动跳转事件: Day ${day}, Monster: ${monster_name}`);
          
          // 支持并列名称 (如: "毒素 吹箭枪陷阱|黑曜石 吹箭枪陷阱|炽焰 吹箭枪陷阱")
          const names = monster_name.includes('|') ? monster_name.split('|') : [monster_name];

          // 1. 自动展开插件
          if (isCollapsed) {
              setIsCollapsed(false);
          }

          // 2. 跳转到对应天数
          setCurrentDay(day);
          const dayLabel = day >= 10 ? "Day 10+" : `Day ${day}`;
          setSelectedDay(dayLabel);

          // 3. 高亮匹配的怪物 (设置 Identify 和 Expand)
          setIdentifiedNames(names);
          setExpandedMonsters(prev => {
              const next = new Set(prev);
              names.forEach(n => next.add(n));
              return next;
          });
          
          // 滚动到第一个匹配项 (可选)
          setTimeout(() => {
              const element = document.getElementById(`monster-${names[0]}`);
              if (element) {
                  element.scrollIntoView({ behavior: 'smooth', block: 'start' });
              }
          }, 100);

          // 4. 切换到怪物 Tab
          setActiveTab("monster");
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
          Promise.all(items.map(async (i) => ({ ...i, displayImg: await getImg(`images/${i.uuid || i.name}.webp`) })));

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
      if (unlistenCard) unlistenCard();
      if (unlistenDay) unlistenDay();
      if (unlistenSync) unlistenSync();
      if (unlistenAutoJump) unlistenAutoJump();
    };
  }, [isCollapsed]); // 添加依赖以确保 isCollapsed 更新有效 (但要注意闭包陷阱，最好用 ref 或函数式更新)

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

    const monstersOnDay = Object.values(allMonsters).filter(m =>
      m && typeof m.name_zh === "string" && m.name_zh.length > 0 && m.available === targetDay
    );
    
    console.log(`[DEBUG] Filtering monsters for ${targetDay}:`, monstersOnDay.length, 'found');
    const jackMonster = monstersOnDay.find(m => m.name_zh === '快乐杰克南瓜');
    if (targetDay === 'Day 7') {
      console.log('[DEBUG] Day 7 快乐杰克南瓜:', jackMonster);
    }
    
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
    // 优先使用后端传递的 image 字段
    let filename = m.image ? m.image.split('/').pop() || `${m.name_zh}.webp` : `${m.name_zh}.webp`;
    
    // 调试日志：如果图片依然出不来，请查看此输出
    if (m.name_zh === '快乐杰克南瓜' || m.name_zh === '绿洲守护神') {
       console.log(`[Image Processing] ${m.name_zh}:`, { m_image: m.image, derived_filename: filename });
    }

    // 尝试寻找角色图
    let displayImg = await getImg(`images_monster_char/${filename}`);
    
    // 如果找不到特定图片，尝试剥离前缀（针对陷阱类：毒素 吹箭枪陷阱 -> 吹箭枪陷阱.webp）
    if (!displayImg && m.name_zh.includes(' ')) {
      const spacePos = m.name_zh.lastIndexOf(' ');
      const baseName = m.name_zh.substring(spacePos + 1);
      const fallbackFilename = `${baseName}.webp`;
      const fallbackImg = await getImg(`images_monster_char/${fallbackFilename}`);
      if (fallbackImg) {
        displayImg = fallbackImg;
        filename = fallbackFilename; // 更新 filename 以供背景图共享
      }
    }
    
    // 背景图路径
    let bgFilename = filename;
    // 绿洲守护神背景图特殊处理
    if (m.name_zh === '绿洲守护神') {
      bgFilename = '绿洲守护神_Day9.webp';
    }
    const displayImgBg = await getImg(`images_monster_bg/${bgFilename}`);

    return {
      ...m,
      displayImg: displayImg,
      displayImgBg: displayImgBg,
      skills: m.skills ? await Promise.all(m.skills.map(async s => ({ 
        ...s, 
        displayImg: await getImg(`images/${s.id || s.name}.webp`) 
      }))) : [],
      items: m.items ? await Promise.all(m.items.map(async i => ({ 
        ...i, 
        displayImg: await getImg(`images/${i.id || i.name}.webp`) 
      }))) : []
    };
  };

  const renderTierInfo = (item: MonsterSubItem) => {
    if (!item) return null;
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
      const sizeClassFallback = (item.size || 'Medium').split(' / ')[0].toLowerCase();
      return (
        <div className="sub-item-card tier-unknown">
           <div className="sub-item-header">
              <div className={`sub-item-img-wrap size-${sizeClassFallback}`}>
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
    const sizeClass = (item.size || 'Medium').split(' / ')[0].toLowerCase();

    return (
      <div 
        className={`sub-item-card tier-${currentTier} ${isProgressionActive ? 'progression-active' : ''}`} 
        style={{ borderLeft: `4px solid ${borderColor}` }}
        onClick={toggleProgression}
      >
        <div className="sub-item-header">
          <div className={`sub-item-img-wrap size-${sizeClass}`} style={{ outline: `2px solid ${borderColor}` }}>
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
                    <div className="sub-item-cd-progression">
                      {cdValues.map((v, i) => (
                        <Fragment key={activeTiers[i].tier}>
                          <div className={`cd-step val-${activeTiers[i].tier}`}>
                            {v.replace('s', '')}
                          </div>
                          {i < activeTiers.length - 1 && <div className="cd-arrow">»</div>}
                        </Fragment>
                      ))}
                      <div className="cd-unit">秒</div>
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

        // 自动切换到对应 Day Tab
        if (validNames.length > 0) {
          const firstMonsterName = validNames[0];
          const monster = allMonsters[firstMonsterName];
          if (monster && monster.available) {
             if (selectedDay !== monster.available) {
               console.log(`[Auto-Switch] 自动识别到 ${firstMonsterName} (${monster.available})，自动切换 Day Tab`);
               setSelectedDay(monster.available);
               
               try {
                 const match = monster.available.match(/Day\s+(\d+)/);
                 if (match && match[1]) {
                   const dayNum = parseInt(match[1]);
                   // 仅当差异较大时才更新 currentDay，或者总是更新？
                   // 为了保持一致性，总是更新比较好
                   setCurrentDay(dayNum);
                 }
               } catch (e) {
                 console.warn("Failed to parse day from available string:", monster.available);
               }
             }
          }
        }
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
        targetW = 600;
        targetH = 850;
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
      <div 
        className="update-screen"
        style={{ 
          '--user-font-size': `${fontSize}px`,
          '--font-scale': fontSize / 16 
        } as any}
      >
        <div className="update-content">
          <h1 className="bulletin-title" data-tauri-drag-region>集市小抄</h1>
          
          <div className="bulletin-body">
            {announcement ? (
              <div className="bulletin-text">
                {announcement.split('\n').map((line, i) => (
                  <p key={i}>{line}</p>
                ))}
              </div>
            ) : (
              <div className="bulletin-loading">正在获取最新公告...</div>
            )}
          </div>

          <div className="version-info-row">
            <span className="current-v">当前版本: v{currentVersion || "..."}</span>
            <div className="update-status-tag">
              {updateStatus === "checking" && <span className="status-checking">检查更新中...</span>}
              {updateStatus === "available" && <span className="status-available pulsate">新版本 v{updateAvailable?.version} 可用</span>}
              {updateStatus === "none" && <span className="status-none">已是最新版</span>}
            </div>
          </div>

          <div className="bulletin-actions">
            {updateStatus === "available" && (
              <button className="update-now-btn" onClick={() => {
                // 如果有更新，可以引导去下载或直接触发更新逻辑
                // 这里暂时保持原样，或者可以调用 installUpdate
              }}>
                立即更新
              </button>
            )}
            <button className="enter-btn" onClick={enterApp}>
              进入插件
            </button>
          </div>
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
      {/* 3. 全局错误提示 Toast */}
      {errorMessage && (
        <div style={{
          position: 'fixed',
          top: '20px',
          left: '50%',
          transform: 'translateX(-50%)',
          backgroundColor: '#ff4d4f',
          color: '#fff',
          padding: '12px 24px',
          borderRadius: '8px',
          boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
          zIndex: 9999,
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          fontSize: '14px',
          fontWeight: 600
        }}>
          <span>⚠ {errorMessage}</span>
          <button 
            onClick={() => setErrorMessage(null)}
            style={{ 
              background: 'transparent', 
              border: 'none', 
              color: '#fff', 
              cursor: 'pointer',
              fontSize: '16px' 
            }}
          >
            ×
          </button>
        </div>
      )}

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
                  {/* <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={async () => {
                    try {
                      await invoke("clear_monster_cache");
                      setStatusMsg("野怪特征缓存已清空，请手动重启插件以重新加载。");
                    } catch (e) {
                      setStatusMsg("清空失败: " + e);
                    }
                  }}>清空野怪特征缓存</button> */}
                  <span style={{ fontSize: '12px', color: '#888' }}>已启用内置缓存</span>
                </div>
              </div>
              
              {statusMsg && (
                <div style={{ 
                  background: 'rgba(255, 205, 25, 0.1)', 
                  border: '1px solid rgba(255, 205, 25, 0.3)', 
                  color: '#ffcd19', 
                  padding: '8px', 
                  fontSize: 'calc(12px * var(--font-scale, 1))',
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

              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                  <label>怪物识别按键</label>
                  <button 
                    className="bulk-btn" 
                    style={{ padding: '2px 8px' }}
                    onClick={(e) => {
                      e.preventDefault();
                      setIsRecordingHotkey(true);
                    }}
                  >
                    {isRecordingHotkey ? "请按键..." : (detectionHotkey ? getHotkeyLabel(detectionHotkey) : "未设置")}
                  </button>
                </div>
                {isRecordingHotkey && (
                  <div 
                    style={{ 
                      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, 
                      background: 'rgba(0,0,0,0.8)', zIndex: 9999,
                      display: 'flex', flexDirection: 'column',
                      justifyContent: 'center', alignItems: 'center', color: '#fff' 
                    }}
                    onMouseDown={(e) => {
                      // 阻止默认行为（比如上下文菜单）
                      e.preventDefault();
                      e.stopPropagation();
                      // 根据 MouseEvent.button 映射到虚拟键码 (简单映射)
                      // 0: Left -> 1 (VK_LBUTTON)
                      // 1: Middle -> 4 (VK_MBUTTON)
                      // 2: Right -> 2 (VK_RBUTTON)
                      // 3: Back -> 5 (VK_XBUTTON1)
                      // 4: Forward -> 6 (VK_XBUTTON2)
                      let vk = 0;
                      switch(e.button) {
                        case 0: vk = 1; break;
                        case 1: vk = 4; break;
                        case 2: vk = 2; break;
                        case 3: vk = 5; break;
                        case 4: vk = 6; break;
                      }
                      if (vk > 0) {
                        setDetectionHotkey(vk);
                        invoke("set_detection_hotkey", { hotkey: vk });
                        setIsRecordingHotkey(false);
                      }
                    }}
                    onKeyDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      // 如何在JS中获取 Windows VK Code?
                      // 其实 keyCode 属性虽然被废弃，但在大部分现代浏览器 + Windows WebView2 环境下
                      // 其实大部分都能对应上 Windows 的 Virtual Key Code。
                      // 如 F2 -> 113, A -> 65
                      if (e.keyCode) {
                        setDetectionHotkey(e.keyCode);
                        invoke("set_detection_hotkey", { hotkey: e.keyCode });
                        setIsRecordingHotkey(false);
                      }
                    }}
                    // 使 div 能获取焦点以接收键盘事件
                    tabIndex={0}
                    ref={(el) => el?.focus()}
                  >
                    <div style={{ fontSize: '20px', marginBottom: '10px' }}>请按下新的热键</div>
                    <div style={{ fontSize: '14px', color: '#aaa' }}>支持: 键盘按键, 鼠标左/中/右键/侧键</div>
                    <button 
                      style={{ marginTop: '20px', padding: '5px 15px' }}
                      onClick={(e) => { e.stopPropagation(); setIsRecordingHotkey(false); }}
                    >取消</button>
                  </div>
                )}
                <div className="setting-tip">默认: 鼠标右键 (VK: 2)</div>
              </div>

              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                  <label>卡牌识别按键</label>
                  <button 
                    className="bulk-btn" 
                    style={{ padding: '2px 8px' }}
                    onClick={(e) => {
                      e.preventDefault();
                      setIsRecordingCardHotkey(true);
                    }}
                  >
                    {isRecordingCardHotkey ? "请按键..." : (cardDetectionHotkey ? getHotkeyLabel(cardDetectionHotkey) : "未设置")}
                  </button>
                </div>
                {isRecordingCardHotkey && (
                  <div 
                    style={{ 
                      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, 
                      background: 'rgba(0,0,0,0.8)', zIndex: 9999,
                      display: 'flex', flexDirection: 'column',
                      justifyContent: 'center', alignItems: 'center', color: '#fff' 
                    }}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      let vk = 0;
                      switch(e.button) {
                        case 0: vk = 1; break;
                        case 1: vk = 4; break;
                        case 2: vk = 2; break;
                        case 3: vk = 5; break;
                        case 4: vk = 6; break;
                      }
                      if (vk > 0) {
                        setCardDetectionHotkey(vk);
                        invoke("set_card_detection_hotkey", { hotkey: vk });
                        setIsRecordingCardHotkey(false);
                      }
                    }}
                    onKeyDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (e.keyCode) {
                        setCardDetectionHotkey(e.keyCode);
                        invoke("set_card_detection_hotkey", { hotkey: e.keyCode });
                        setIsRecordingCardHotkey(false);
                      }
                    }}
                    tabIndex={0}
                    ref={(el) => el?.focus()}
                  >
                    <div style={{ fontSize: '20px', marginBottom: '10px' }}>请按下新的热键</div>
                    <div style={{ fontSize: '14px', color: '#aaa' }}>支持: 键盘按键, 鼠标左/中/右键/侧键</div>
                    <button 
                      style={{ marginTop: '20px', padding: '5px 15px' }}
                      onClick={(e) => { e.stopPropagation(); setIsRecordingCardHotkey(false); }}
                    >取消</button>
                  </div>
                )}
                <div className="setting-tip">默认: Alt (VK: 18)</div>
              </div>

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
                          
                          if (remoteJson.notes) {
                            setAnnouncement(remoteJson.notes);
                          }
                          
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

                {updateStatus === "checking" && <div style={{ fontSize: 'calc(12px * var(--font-scale, 1))', color: '#999' }}>正在检查远端更新...</div>}
                {updateStatus === "none" && <div style={{ fontSize: 'calc(12px * var(--font-scale, 1))', color: '#238636' }}>当前已经是最新版本</div>}
                
                {(updateStatus === "available" || updateStatus === "downloading" || updateStatus === "ready") && (
                  <div style={{ 
                    background: 'rgba(56, 139, 253, 0.15)', 
                    border: '1px solid rgba(56, 139, 253, 0.4)', 
                    padding: '10px', 
                    borderRadius: '6px' 
                  }}>
                    <div style={{ fontSize: 'calc(13px * var(--font-scale, 1))', fontWeight: 'bold', marginBottom: '8px', color: '#58a6ff' }}>
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

              {announcement && (
                <div className="setting-item" style={{ marginTop: '20px' }}>
                  <label style={{ display: 'block', marginBottom: '8px', color: '#8b949e' }}>当前公告</label>
                  <div className="settings-announcement-text">
                    {announcement}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {!isCollapsed && (
        <>
          {/* 更新按钮 */}
          <nav className="nav-bar">
            {(["monster", "card", "hand", "stash"] as TabType[]).map(t => (
              <div key={t} className={`nav-item ${activeTab === t ? 'active' : ''}`} onClick={() => setActiveTab(t)}>
                {t === 'monster' ? '野怪一览' : t === 'card' ? '卡牌识别' : t === 'hand' ? '手牌' : '仓库'}
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

                    {/* 原识别按钮已移除 */}

                    {!templateLoading.is_complete && templateLoading.total > 0 && (
                      <div className="loading-progress">
                        <div className="progress-text">加载怪物模板: {templateLoading.loaded}/{templateLoading.total}</div>
                        <div className="progress-bar"><div className="progress-fill" style={{ width: `${templateLoading.total > 0 ? (templateLoading.loaded / templateLoading.total * 100) : 0}%` }} /></div>
                      </div>
                    )}
                  </div>

                  <div className="monster-list-v2">
                    {manualMonsters.sort((a, b) => {
                      // 识别成功的怪物排在前面
                      const aIdentified = identifiedNames.includes(a.name_zh);
                      const bIdentified = identifiedNames.includes(b.name_zh);
                      if (aIdentified && !bIdentified) return -1;
                      if (!aIdentified && bIdentified) return 1;
                      return 0;
                    }).map((m, i) => {
                      const isIdentified = identifiedNames.includes(m.name_zh);
                      const isExpanded = expandedMonsters.has(m.name_zh);
                      
                      return (
                        <div id={`monster-${m.name_zh}`} key={i} className={`monster-card-v2 ${isIdentified ? 'identified-glow' : ''} ${isExpanded ? 'expanded' : ''}`} onClick={() => toggleMonsterExpand(m.name_zh)}>
                          <div className="monster-header-v2">
                            <div className="avatar-wrap">
                              <div className="monster-image-layers">
                                <img src={m.displayImgBg} className="monster-layer-bg" alt="" />
                                <img src={m.displayImg} className="monster-layer-char" alt="" />
                              </div>
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
                <>
                  <div className="card-list">
                    {(activeTab === "card" ? recognizedCards : getSortedItems(activeTab === "hand" ? syncData.hand_items : syncData.stash_items)).map((item, idx) => {
                      const isExpanded = expandedItems.has(item.uuid);
                      const isRecognized = activeTab === "card";
                      const isTopMatch = idx === 0;
                      const tierClass = item.tier.split(' / ')[0].toLowerCase();
                      const tierNameZh = {
                        'bronze': '青铜+',
                        'silver': '白银+',
                        'gold': '黄金+',
                        'diamond': '钻石+'
                      }[tierClass] || tierClass;
                      const heroZh = item.heroes[0]?.split(' / ')[1] || item.heroes[0] || "通用";
                      const sizeClass = item.size?.split(' / ')[0].toLowerCase() || 'medium';

                      return (
                        <div key={item.uuid} className={`item-card-container ${isExpanded ? 'expanded' : ''} ${isRecognized ? 'identified-glow' : ''}`} onClick={() => toggleExpand(item.uuid)}>
                          <div className={`item-card tier-${tierClass}`}>
                            <div className="card-left">
                              <div className={`image-box size-${sizeClass}`}>
                                <img src={item.displayImg} alt={item.name} />
                              </div>
                            </div>

                            <div className="card-center">
                              <div className="name-line">
                                <span className="name-cn">{item.name_cn}</span>
                                {isRecognized && (
                                  <span className="id-badge" style={{ 
                                    marginLeft: '4px',
                                    backgroundColor: isTopMatch ? '#238636' : '#8b949e' 
                                  }}>
                                    {isTopMatch ? "MATCH" : "MAYBE"}
                                  </span>
                                )}
                                <span className={`tier-label tier-${tierClass}`}>{tierNameZh}</span>
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
                          <div className={`item-details-v2 ${pinnedItems.has(item.uuid) ? 'progression-active' : ''}`}>
                            {(() => {
                                try {
                                    // 强制从原始数据读取，防止类型系统干扰
                                    const cdTiersRaw = (item as any).cooldown_tiers;
                                    const availTiersRaw = (item as any).available_tiers;
                                    
                                    const hasProgression = cdTiersRaw && typeof cdTiersRaw === 'string' && cdTiersRaw.includes('/');
                                    
                                    if (hasProgression) {
                                      const cdVals = (cdTiersRaw as string).split('/').map((v: string) => {
                                        const ms = parseFloat(v);
                                        return isNaN(ms) ? "0.0" : (ms/1000).toFixed(1);
                                      });
                                      const availTiers = (availTiersRaw || "").split('/').map((t: string) => t.toLowerCase().trim());
                                      const tierSequence = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
                                      
                                      return (
                                        <div className="details-left">
                                          <div className="sub-item-cd-progression" style={{ 
                                            position: 'static', 
                                            background: 'rgba(0,0,0,0.2)', 
                                            border: '1px solid rgba(255,255,255,0.05)', 
                                            padding: '4px',
                                            borderRadius: '4px',
                                            minWidth: '50px'
                                          }}>
                                            {cdVals.map((v: string, i: number) => {
                                              let tierName = 'gold';
                                              if (availTiers[i]) {
                                                tierName = availTiers[i];
                                              } else {
                                                if (cdVals.length === 2) tierName = i === 0 ? 'gold' : 'diamond';
                                                else tierName = tierSequence[i] || 'gold';
                                              }

                                              return (
                                                <Fragment key={i}>
                                                  <div className={`cd-step val-${tierName}`} style={{ fontSize: '16px' }}>{v}</div>
                                                  {i < cdVals.length - 1 && <div className="cd-arrow" style={{ transform: 'none', margin: '0' }}>↓</div>}
                                                </Fragment>
                                              );
                                            })}
                                            <div className="cd-unit">秒</div>
                                          </div>
                                        </div>
                                      );
                                    }
                                } catch (e) {
                                  console.error("Error rendering CD progression:", e);
                                }
                                
                                return item.cooldown !== undefined && item.cooldown > 0 && (
                                  <div className="details-left">
                                    <div className="cd-display">
                                      <div className="cd-value">{item.cooldown.toFixed(1)}</div>
                                      <div className="cd-unit">秒</div>
                                    </div>
                                  </div>
                                );
                            })()}
                            <div className="details-right">
                              {item.skills.map((s, idx) => (
                                <div key={idx} className="skill-item">
                                  {renderText(s)}
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                        {item.enchantments.length > 0 && isExpanded && (
                          <div className="item-enchantments-row">
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
                                    <span className="enchant-effect">{renderText(effect)}</span>
                                  </div>
                                );
                              }
                              return (
                                <div key={idx} className="enchant-item">
                                  {renderText(enc)}
                                </div>
                              );
                            })}
                          </div>
                        )}
                        {item.description && isExpanded && (
                          <div className="item-description-row">
                            <div className="description-text">
                              {renderText(item.description)}
                            </div>
                          </div>
                        )}
                      </div>
                    );
                  })}
                  {activeTab === "card" && recognizedCards.length === 0 && !isRecognizingCard && (
                    <div className="empty-tip">按下 Alt 键 识别鼠标指向的卡牌</div>
                  )}
                  {activeTab === "card" && isRecognizingCard && (
                    <div className="empty-tip">🔍 正在识别中...</div>
                  )}
                  {(activeTab === "hand" || activeTab === "stash") && (activeTab === "hand" ? syncData.hand_items : syncData.stash_items).length === 0 && (
                    <div className="empty-tip">当前暂无数据，请在游戏中操作相应卡牌</div>
                  )}
                </div>
              </>
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