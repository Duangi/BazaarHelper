import * as React from "react";
import { useEffect, useRef, useState, Fragment, useCallback } from "react";
import { getCurrentWindow, LogicalPosition, LogicalSize, currentMonitor } from "@tauri-apps/api/window";
import { listen, emit } from "@tauri-apps/api/event";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import { getVersion } from '@tauri-apps/api/app';
import { check, Update } from '@tauri-apps/plugin-updater';
import "./App.css";

import { exit, relaunch } from '@tauri-apps/plugin-process';
import { SettingGroup } from './components/SettingsPanel';

// 导入新组件
// import { TopBar } from './components/TopBar';
// import { TabBar } from './components/TabBar';
// import { ToastContainer } from './components/Toast';
// import { MonsterView } from './views/MonsterView';
// import { ItemsView } from './views/ItemsView';
// import { CardRecognitionView } from './views/CardRecognitionView';

// 导入类型和工具
import type { ItemData, MonsterData, TabType, SyncPayload, TierInfo, MonsterSubItem } from './types';
import { getImg, getHotkeyLabel } from './utils/helpers';
import { renderText, renderEnchantText } from './utils/renderText';
import { ENCHANT_COLORS, HERO_COLORS } from './constants/colors';

// 保持兼容性的类型定义（以防其他地方还在使用）
// interface ItemDataLegacy... removed

// const imgCache = new Map<string, string>();

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
  const [enableYoloAuto, setEnableYoloAuto] = useState(() => {
    const saved = localStorage.getItem("enable-yolo-auto");
    return saved === "true";
  });
  const [yoloScanInterval, setYoloScanInterval] = useState(() => {
    const saved = localStorage.getItem("yolo-scan-interval");
    return saved ? parseFloat(saved) : 1.0; // Default 1 second
  });

  // 调试日志：检查初始加载的设置
  useEffect(() => {
    console.log(`[App Config] Loaded from cache - EnableYoloAuto: ${enableYoloAuto}, Interval: ${yoloScanInterval}s`);
  }, []);

  const [useGpuAcceleration, setUseGpuAcceleration] = useState(() => {
    const saved = localStorage.getItem("use-gpu-acceleration");
    if (saved === null) {
      // 首次运行，默认开启并写入 localStorage
      localStorage.setItem("use-gpu-acceleration", "true");
      return true;
    }
    return saved === "true";
  });
  const [showYoloMonitor, setShowYoloMonitor] = useState(() => {
    const saved = localStorage.getItem("show-yolo-monitor");
    if (saved === null) {
      // 首次运行，默认开启并写入 localStorage
      localStorage.setItem("show-yolo-monitor", "true");
      return true;
    }
    return saved === "true";
  });
  // Toast 提示系统
  interface Toast {
    id: number;
    message: string;
    type: 'success' | 'error' | 'warning' | 'info';
  }
  const [toasts, setToasts] = useState<Toast[]>([]);
  
  const [yoloHotkey, setYoloHotkey] = useState<number | null>(() => {
    const saved = localStorage.getItem("yolo-hotkey");
    return saved ? parseInt(saved) : 0; // 默认未设置
  });
  const [announcement, setAnnouncement] = useState(""); // 公告内容
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set()); // 手牌/仓库点击展开附魔
  const [expandedMonsters, setExpandedMonsters] = useState<Set<string>>(new Set()); // 野怪点击展开
  const [recognizedCards, setRecognizedCards] = useState<ItemData[]>([]); // 识别出的卡牌列表 (Top 3)
  const [isRecognizingCard, setIsRecognizingCard] = useState(false); // 是否正在识别卡牌
  // Listen to backend hotkey events
  useEffect(() => {
    const unlistenMonster = listen("hotkey-monster", async () => {
        console.log("[App] Received hotkey-monster");
        if (!isRecognizing) {
             setIsRecognizing(true);
             try {
                 setActiveTab("monster");
                 
                 const res = await invoke("recognize_monsters_from_screenshot", { day: currentDay });
                 console.log("Recognition result:", res);
                 if (Array.isArray(res)) {
                      setManualMonsters(res as MonsterData[]);
                 }
             } catch (e) {
                 console.error("Recognition failed", e);
             } finally {
                 setIsRecognizing(false);
             }
        }
    });
    
    const unlistenCollapse = listen("hotkey-collapse", () => {
        console.log("[App] Received hotkey-collapse");
        setIsCollapsed(prev => !prev);
    });

    return () => {
        unlistenMonster.then(f => f());
        unlistenCollapse.then(f => f());
    };
  }, [useGpuAcceleration, currentDay, isRecognizing]);
  const [searchQuery, setSearchQuery] = useState({
    keyword: "",
    item_type: "all", // "all", "item", "skill"
    size: "",
    start_tier: "",
    hero: "",
    tags: "",
    hidden_tags: ""
  });
  const [searchResults, setSearchResults] = useState<ItemData[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [isSearchFilterCollapsed, setIsSearchFilterCollapsed] = useState(false);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [selectedHiddenTags, setSelectedHiddenTags] = useState<string[]>([]);
  const [matchMode, setMatchMode] = useState<'all' | 'any'>('all'); // 'all' = 匹配所有, 'any' = 匹配任一
  const [isInputFocused, setIsInputFocused] = useState(false); // 跟踪输入框焦点状态
  const [lastItemSize, setLastItemSize] = useState(''); // 记住切换到技能前的尺寸选择
  const [searchFilterHeight, setSearchFilterHeight] = useState(300);
  const [isResizingFilter, setIsResizingFilter] = useState(false);
  const [resizeStartY, setResizeStartY] = useState(0);
  const [resizeStartHeight, setResizeStartHeight] = useState(0);

  // 隐藏标签图标URL缓存
  const [hiddenTagIcons, setHiddenTagIcons] = useState<Record<string, string>>({});
  // 赞助图片URL缓存
  const [sponsorIcons, setSponsorIcons] = useState<{vx: string, zfb: string}>({vx: '', zfb: ''});

  // 预加载隐藏标签图标和赞助图片
  useEffect(() => {
    (async () => {
      // 加载隐藏标签图标
      const iconNames = ["Ammo", "Burn", "Charge", "Cooldown", "CritChance", "Damage", "Income", 
                         "Flying", "Freeze", "Haste", "Health", "MaxHPHeart", "Lifesteal", "Poison", 
                         "Regen", "Shield", "Slowness"];
      const icons: Record<string, string> = {};
      for (const name of iconNames) {
        try {
          const fullPath = await resolveResource(`resources/images_GUI/${name}.webp`);
          const url = convertFileSrc(fullPath);
          icons[name] = url;
        } catch (e) {
          console.error(`Failed to load icon ${name}:`, e);
        }
      }
      setHiddenTagIcons(icons);

      // 加载赞助图片
      try {
        const vxPath = await resolveResource('resources/sponsor/vx.png');
        const zfbPath = await resolveResource('resources/sponsor/zfb.png');
        setSponsorIcons({
            vx: convertFileSrc(vxPath),
            zfb: convertFileSrc(zfbPath)
        });
      } catch (e) {
          console.error("Failed to load sponsor icons", e);
      }
    })();
  }, []);


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
      } catch (e) {
        console.warn('Failed to load skills_db.json for art map', e);
      }
    })();
  }, []);

  // Lazy Load State
  const [visibleCount, setVisibleCount] = useState(50);
  const scrollAreaRef = useRef<HTMLDivElement | null>(null);

  // Reset filtered items count when query changes
  useEffect(() => {
    setVisibleCount(50);
    // Scroll to top
    if (scrollAreaRef.current) {
        scrollAreaRef.current.scrollTop = 0;
    }
  }, [searchQuery, activeTab, selectedDay, syncData]);

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const { scrollTop, scrollHeight, clientHeight } = e.currentTarget;
    // Load more if scrolled to bottom (within 200px)
    if (scrollHeight - scrollTop - clientHeight < 200) {
      setVisibleCount(prev => prev + 20);
    }
  };

  // Debounced search effect
  useEffect(() => {
    const handler = setTimeout(async () => {
      if (activeTab === "search") {
        setIsSearching(true);
        try {
          const res = await invoke<ItemData[]>("search_items", { query: searchQuery });
          
          // Filter out "中型包裹" and apply multi-select tag filters
          let filtered = res.filter(item => 
            !item.name_cn.includes('中型包裹') && 
            !item.name.includes('Medium Package')
          );
          
          // Apply multi-select tag filters based on match mode
          if (selectedTags.length > 0) {
            filtered = filtered.filter(item => {
              const itemTags = item.tags.toLowerCase();
              if (matchMode === 'all') {
                // 匹配所有：必须包含所有选中的标签
                return selectedTags.every(tag => itemTags.includes(tag.toLowerCase()));
              } else {
                // 匹配任一：包含任意一个选中的标签即可
                return selectedTags.some(tag => itemTags.includes(tag.toLowerCase()));
              }
            });
          }
          if (selectedHiddenTags.length > 0) {
            filtered = filtered.filter(item => {
              const itemHiddenTags = item.hidden_tags.toLowerCase();
              if (matchMode === 'all') {
                // 匹配所有：必须包含所有选中的隐藏标签
                return selectedHiddenTags.every(tag => itemHiddenTags.includes(tag.toLowerCase()));
              } else {
                // 匹配任一：包含任意一个选中的隐藏标签即可
                return selectedHiddenTags.some(tag => itemHiddenTags.includes(tag.toLowerCase()));
              }
            });
          }
          
          // Image patching: Search results don't have displayImg set.
          const patched = await Promise.all(filtered.map(async (item) => {
            let imgPath = '';
            
            // Check if this item is a skill by looking up in skillsArtMap
            const art = item.uuid ? skillsArtMap[item.uuid] : undefined;
            if (art) {
              // It's a skill - use art_key based path
              const base = art.split('/').pop() || art;
              const nameNoExt = base.replace(/\.[^/.]+$/, '');
              imgPath = `images/skill/${nameNoExt}.webp`;
            } else {
              // It's a regular item - use uuid
              imgPath = `images/${item.uuid}.webp`;
            }
            
            const url = await getImg(imgPath);
            return { ...item, displayImg: url };
          }));
          
          setSearchResults(patched);
        } catch (e) {
          console.error("Search failed:", e);
        } finally {
          setIsSearching(false);
        }
      }
    }, 300);
    return () => clearTimeout(handler);
  }, [searchQuery, activeTab, skillsArtMap, selectedTags, selectedHiddenTags, matchMode]);

  // Handle filter resize
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isResizingFilter) {
        const deltaY = e.clientY - resizeStartY;
        const newHeight = resizeStartHeight + deltaY;
        setSearchFilterHeight(Math.max(200, Math.min(newHeight, window.innerHeight * 0.6)));
      }
    };
    const handleMouseUp = () => {
      setIsResizingFilter(false);
    };
    if (isResizingFilter) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    }
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingFilter, resizeStartY, resizeStartHeight]);


  // 图片路径缓存，避免重复解析
  // const [imgCache] = useState<Map<string, string>>(new Map());

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const appWindow = getCurrentWindow(); // 获取当前窗口实例
  
  const [hasCustomPosition, setHasCustomPosition] = useState(false);
  const lastKnownPosition = useRef<{ x: number; y: number } | null>(null);
  const isLoadingGeometry = useRef(true); // 防止初始化时覆盖后端位置
  const [isGeometryLoaded, setIsGeometryLoaded] = useState(false); // 触发 effect
  
  // 从后端加载保存的窗口位置
  useEffect(() => {
    const loadSavedPosition = async () => {
      try {
        const geometry = await invoke<{x?: number, y?: number, width?: number, height?: number}>('get_window_geometry', { windowLabel: 'main' });
        console.log('[Frontend] Loaded geometry from backend:', geometry);
        
        // 获取当前屏幕缩放比例，用于物理转逻辑
        let scale = 1.0;
        try {
          const monitor = await currentMonitor();
          if (monitor) scale = monitor.scaleFactor;
        } catch (e) {
             console.warn("Failed to get monitor scale", e);
        }
        currentScale.current = scale;
        
        if (geometry.x !== undefined && geometry.y !== undefined) {
          lastKnownPosition.current = { x: geometry.x, y: geometry.y };
          setHasCustomPosition(true);
          console.log('[Frontend] Using saved position:', geometry.x, geometry.y);
        } else {
          // 没有保存的位置，尝试从localStorage读取（兼容旧版本）
          const x = localStorage.getItem("plugin-pos-x");
          const y = localStorage.getItem("plugin-pos-y");
          if (x !== null && y !== null) {
            lastKnownPosition.current = { x: parseInt(x), y: parseInt(y) };
            setHasCustomPosition(true);
            console.log('[Frontend] Using localStorage position:', x, y);
          }
        }
        
        // 恢复保存的大小 (Physical -> Logical)
        if (geometry.width && geometry.height && geometry.width > 200 && geometry.height > 200) {
             const logicalW = Math.round(geometry.width / scale);
             const logicalH = Math.round(geometry.height / scale);
             console.log(`[Frontend] Using saved size (Physical -> Logical): ${geometry.width}x${geometry.height} -> ${logicalW}x${logicalH}`);
             
             expandedWidthRef.current = logicalW;
             expandedHeightRef.current = logicalH;
             setExpandedWidth(logicalW);
             setExpandedHeight(logicalH);
             
             // Sync localStorage
             localStorage.setItem("plugin-width", logicalW.toString());
             localStorage.setItem("plugin-height", logicalH.toString());
        }
      } catch (e) {
        console.error('[Frontend] Failed to load saved geometry:', e);
      } finally {
        // 加载完成，允许syncLayout工作
        setTimeout(() => {
          isLoadingGeometry.current = false;
          setIsGeometryLoaded(true);
          console.log('[Frontend] Geometry loading complete, syncLayout enabled');
        }, 1000);
      }
    };
    
    loadSavedPosition();
  }, []);
  
  // 存储当前屏幕缩放比例，用于坐标转换
  const currentScale = useRef(1);

  // 新增：识别热键状态
  const [detectionHotkey, setDetectionHotkey] = useState<number | null>(null);
  const [cardDetectionHotkey, setCardDetectionHotkey] = useState<number | null>(null);
  const [toggleCollapseHotkey, setToggleCollapseHotkey] = useState<number | null>(null);
  const [detailDisplayHotkey, setDetailDisplayHotkey] = useState<number | null>(null);
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [isRecordingCardHotkey, setIsRecordingCardHotkey] = useState(false);
  const [isRecordingToggleHotkey, setIsRecordingToggleHotkey] = useState(false);
  const [isRecordingYoloHotkey, setIsRecordingYoloHotkey] = useState(false);
  const [showResetHotkeysConfirm, setShowResetHotkeysConfirm] = useState(false);
  const [isRecordingDetailHotkey, setIsRecordingDetailHotkey] = useState(false);
  
  // 设置分类展开状态
  const [settingsExpanded, setSettingsExpanded] = useState({
    ui: false,
    yolo: false,
    hotkeys: false
  });
  
  const isInitialized = useRef(false);
  const moveDebounceTimer = useRef<number | null>(null);
  const saveSizeTimer = useRef<number | null>(null);
  const isCollapsedRef = useRef(false);
  const isDragging = useRef(false);
  const isResizing = useRef(false);
  const lastUserResize = useRef<number>(0);
  const isProgrammaticResize = useRef(false);
  const showVersionScreenRef = useRef(true);
  
  // 从 localStorage 初始化 ref，确保和 state 的初始值一致
  const getInitialWidth = () => {
    const saved = localStorage.getItem("plugin-width");
    if (saved) {
      const value = parseInt(saved, 10);
      if (value > 200) return value;
    }
    return 400;
  };
  const getInitialHeight = () => {
    const saved = localStorage.getItem("plugin-height");
    if (saved) {
      const value = parseInt(saved, 10);
      if (value > 200) return value;
    }
    return 700;
  };
  const expandedWidthRef = useRef(getInitialWidth());
  const expandedHeightRef = useRef(getInitialHeight());

  // 同步 isCollapsed 到 Ref，用于监听器内部访问最新值
  useEffect(() => {
    isCollapsedRef.current = isCollapsed;
  }, [isCollapsed]);

  // 监听窗口调整大小和移动
  useEffect(() => {
    let unlistenMove: (() => void) | null = null;
    let unlistenResize: (() => void) | null = null;

    const setupListeners = async () => {
      // 等待较短时间后才开始监听，避免初始定位触发
      setTimeout(() => {
        isInitialized.current = true;
      }, 500);

      // 监听窗口移动事件 (Tauri v2)
      unlistenMove = await appWindow.listen<{ x: number; y: number }>('tauri://move', (event) => {
        if (!isInitialized.current) return;
        if (showVersionScreenRef.current) return; // 不保存版本选择界面的位置
        
        isDragging.current = true;
        
        if (moveDebounceTimer.current) clearTimeout(moveDebounceTimer.current);
        moveDebounceTimer.current = window.setTimeout(() => {
          const physicalX = event.payload.x;
          const physicalY = event.payload.y;
          
          console.log('[Frontend] Saving position after move:', physicalX, physicalY);
          
          // Save to backend state (persistent)
          invoke('save_window_geometry', {
             windowLabel: 'main',
             x: physicalX,
             y: physicalY
          }).catch(console.error);

          setHasCustomPosition(true);
          lastKnownPosition.current = { x: physicalX, y: physicalY };
          localStorage.setItem("plugin-pos-x", physicalX.toString());
          localStorage.setItem("plugin-pos-y", physicalY.toString());
          
          setTimeout(() => {
            isDragging.current = false;
          }, 300);
        }, 2000);
      });

      // 监听窗口调整大小事件 (同步状态并保存)
      unlistenResize = await appWindow.listen<{ width: number; height: number }>('tauri://resize', async (_event) => {
        // 如果这是由程序主动调用 setSize 触发的 resize，则忽略
        if (isProgrammaticResize.current) {
          setTimeout(() => { isProgrammaticResize.current = false; }, 200);
          return;
        }
        if (!isInitialized.current) return;
        if (showVersionScreenRef.current) return;

        // 标记正在调整大小
        isResizing.current = true;
        lastUserResize.current = Date.now();

        // 读取物理尺寸并转换为逻辑尺寸 (Physical -> Logical)
        try {
          const factor = await appWindow.scaleFactor();
          const size = await appWindow.innerSize();
          const logicalWidth = Math.round(size.width / factor);
          const logicalHeight = Math.round(size.height / factor);

          if (logicalWidth > 150 && logicalHeight > 150) {
            // 只更新 ref，不更新 state（避免异步更新导致的覆盖问题）
            expandedWidthRef.current = logicalWidth;
            if (!isCollapsedRef.current) {
              expandedHeightRef.current = logicalHeight;
            }

            // 保存到 localStorage
            if (saveSizeTimer.current) clearTimeout(saveSizeTimer.current);
            saveSizeTimer.current = window.setTimeout(() => {
              console.log('[Frontend] Saving size after resize:', size.width, size.height);
              
              // Save to backend state (persistent)
              invoke('save_window_geometry', {
                 windowLabel: 'main',
                 width: Math.round(size.width), // save physical size
                 height: Math.round(size.height)
              }).catch(console.error);

              localStorage.setItem("plugin-width", logicalWidth.toString());
              if (!isCollapsedRef.current) {
                localStorage.setItem("plugin-height", logicalHeight.toString());
              }
              setTimeout(() => { isResizing.current = false; }, 500);
            }, 2000);
          }
        } catch (e) {
          console.error('[Resize] Failed to get window size:', e);
        }
      });
    };

    setupListeners();

    return () => {
      if (unlistenMove) unlistenMove();
      if (unlistenResize) unlistenResize();
      if (moveDebounceTimer.current) clearTimeout(moveDebounceTimer.current);
      if (saveSizeTimer.current) clearTimeout(saveSizeTimer.current);
    };
  }, []); // 只在组件挂载时运行一次
  const [showVersionScreen, setShowVersionScreen] = useState(true); // 启动时显示版本号
  
  // 同步 showVersionScreen 到 Ref
  useEffect(() => {
    showVersionScreenRef.current = showVersionScreen;
  }, [showVersionScreen]);
  
  const [currentVersion, setCurrentVersion] = useState(""); // 当前版本号
  
  // 更新相关状态
  const [updateAvailable, setUpdateAvailable] = useState<Update | null>(null);
  const [updateStatus, setUpdateStatus] = useState<"none" | "checking" | "available" | "downloading" | "ready">("none");
  const [downloadProgress] = useState(0); // setDownloadProgress removed
  const [isInstalling, setIsInstalling] = useState(false); // 正在安装状态
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Toast 提示函数
  const showToast = (message: string, type: 'success' | 'error' | 'warning' | 'info' = 'info') => {
    const id = Date.now();
    setToasts(prev => [...prev, { id, message, type }]);
    
    // 3秒后自动移除
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, 3000);
  };

  // 禁用右键菜单
  useEffect(() => {
    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };
    window.addEventListener("contextmenu", handleContextMenu);
    return () => window.removeEventListener("contextmenu", handleContextMenu);
  }, []);

  // 监听窗口关闭事件，通知overlay
  useEffect(() => {
    const handleBeforeUnload = () => {
      emit('main-window-closing').catch(console.error);
    };
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, []);

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

  // 置顶/取消置顶功能 (Now uses ID which can be instance_id or uuid)
  const togglePin = (id: string, e: React.MouseEvent) => {
    e.stopPropagation(); // 防止触发展开/收起
    setPinnedItems(prev => {
      const newPinned = new Map(prev);
      if (newPinned.has(id)) {
        newPinned.delete(id);
      } else {
        setPinnedCounter(c => c + 1);
        newPinned.set(id, pinnedCounter + 1);
      }
      return newPinned;
    });
  };

  const toggleExpand = (id: string) => {
    setExpandedItems(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
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
          showToast(`识别成功: 找到 ${fullInfos.length} 个匹配项`, 'success');
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

  // 包装 renderText 和 renderEnchantText，提供 allTags
  const renderTextLocal = (text: any) => renderText(text, syncData.all_tags || []);
  const renderEnchantTextLocal = (content: string) => renderEnchantText(content, syncData.all_tags || []);

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
    // 1. 先排序 (确保置顶的在前面)
    const sorted = [...items].sort((a, b) => {
      // 优先使用 instance_id (如果存在)，否则使用 uuid 判断置顶
      const aId = a.instance_id || a.uuid;
      const bId = b.instance_id || b.uuid;
      const aPin = pinnedItems.get(aId) || pinnedItems.get(a.uuid);
      const bPin = pinnedItems.get(bId) || pinnedItems.get(b.uuid);
      
      if (aPin && bPin) return bPin - aPin; // 都置顶，后置顶的在前
      if (aPin) return -1; // a置顶，a在前
      if (bPin) return 1; // b置顶，b在前
      return 0; // 都不置顶，保持原顺序
    });

    // 2. 去重 (同一个 uuid 只保留第一个)
    // 注意：由于已经排序过，置顶的项会排在前面，所以会被保留
    const seen = new Set<string>();
    return sorted.filter(item => {
      // Use instance_id for uniqueness if available (Hand/Stash cases)
      // Otherwise fall back to uuid (Card recognition cases)
      // If we want to allow duplicates in Card recognition (unlikely needed for just "what is this"), keep uuid.
      // But for Hand/Stash, we MUST allow duplicates (e.g. 2 Pigs).
      // Note: If instance_id is missing, we might still dedup by uuid.
      
      const key = item.instance_id || item.uuid;
      
      // If we are in 'items' view, and we have multiple items with same UUID but NO instance_id (shouldn't happen for valid player items),
      // we might hide them. But assuming player items have instance_id.
      
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  };

  // 1. 记忆宽度与高度
  const [_expandedWidth, setExpandedWidth] = useState(() => {
    const saved = localStorage.getItem("plugin-width");
    if (saved) {
      const value = parseInt(saved, 10);
      // 只过滤极不合理的值
      if (value > 200) {
        return value;
      }
    }
    return 400;
  });
  const [_expandedHeight, setExpandedHeight] = useState(() => {
    const saved = localStorage.getItem("plugin-height");
    if (saved) {
      const value = parseInt(saved, 10);
      // 只过滤极不合理的值
      if (value > 200) {
        return value;
      }
    }
    return 700;
  });

  // enterApp 函数，从版本屏幕进入主应用
  const enterApp = async () => {
    console.log("[Update] Entering App. updateAvailable:", !!updateAvailable);
    setShowVersionScreen(false);
    
    // 移除从当前窗口获取尺寸的逻辑，因为当前窗口是版本界面(600x850)
    // 我们希望进入Plugin时使用的是之前保存的 localStorage 中的尺寸
    // 或者 defaults (getInitialWidth/Height)
    
    // 立即开始模板加载，不等待更新检查
    invoke("start_template_loading").catch(console.error);
    invoke("load_event_templates").catch(console.error);
    
    // 如果有更新，进人应用后开始后台下载
    if (updateAvailable) {
      console.log("[Update] Found update, starting background download...");
      startUpdateDownload();
    }
  };

  const startUpdateDownload = async () => {
    // ... rest of the function (no changes needed here)
  };

  // 启动时显示版本信息并检查更新
  useEffect(() => {
    const initApp = async () => {
      console.log("[App] initApp 开始执行...");
      
      // 1. 立即获取版本号展示
      try {
        const appVersion = await getVersion();
        setCurrentVersion(appVersion);
        console.log(`[App] 启动初始化。当前版本: v${appVersion}`);
      } catch (e) {
        console.error("获取版本失败:", e);
      }

      // 2. 获取公告内容 (从 GitHub 代理)
      const fallbackNotice = "🧠 脑子是用来构筑的，数据交给小抄记。\n\n💡 这只是个免费的记牌小工具，又不是考研资料，谁要是敢收你的费，请反手给他一个大逼兜！👊\n\n🍖 本小抄由 B站@这是李Duang啊 免费发放，付费获取的同学请立刻退款买排骨吃！";
      
      // 不等待公告获取，让 UI 先显示
      fetch("https://gh.llkk.cc/https://raw.githubusercontent.com/Duangi/BazaarHelper/main/update.json")
        .then(res => res.ok ? res.json() : null)
        .then(data => {
          if (data && data.notes) {
            setAnnouncement(data.notes + "\n\n------------------\n\n" + fallbackNotice);
          } else {
            setAnnouncement(fallbackNotice);
          }
        })
        .catch(err => {
          console.error("[App] 获取公告失败:", err);
          setAnnouncement(fallbackNotice);
        });

      // 3. 后台检查更新，不阻塞 UI 渲染
      setTimeout(async () => {
        try {
          console.log("[Update] 正在后台检查更新...");
          setUpdateStatus("checking");
          const update = await check();
          console.log("[Update] check() 响应结果:", update);
          if (update) {
            console.log(`[Update] 检测到新版本! 远端版本: v${update.version}`);
            setUpdateAvailable(update);
            setUpdateStatus("available");
          } else {
            setUpdateStatus("none");
          }
        } catch (error) {
          console.error("[Update] 检查更新失败:", error);
          setUpdateStatus("none");
        }
      }, 100);
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
    // 使用数组存储清理函数，确保无论异步何时完成都能清理
    const unlisteners: (() => void)[] = [];
    let isMounted = true; 

    const setupListeners = async () => {
      // 辅助函数：安全注册监听器
      const safeListen = async <T,>(event: string, callback: (payload: T) => void) => {
        try {
          const unlisten = await listen<T>(event, (e) => {
             if (isMounted) callback(e.payload);
          });
          
          if (isMounted) {
            unlisteners.push(unlisten);
          } else {
            // 如果Promise返回时组件已卸载，立即注销
            unlisten();
          }
        } catch (err) {
          console.error(`Failed to listen to ${event}:`, err);
        }
      };

      // 1. 物品同步 (sync-items) —— 修复重点
      await safeListen<SyncPayload>("sync-items", async (payload) => {
        // 图片处理逻辑
        const processItems = (items: ItemData[]) => 
          Promise.all(items.map(async (i) => ({ 
            ...i, 
            displayImg: await getImg(`images/${i.uuid || i.name}.webp`) 
          })));

        const [hand, stash] = await Promise.all([
          processItems(payload.hand_items || []),
          processItems(payload.stash_items || [])
        ]);

        if (isMounted) {
          setSyncData(prev => ({ 
            ...prev, 
            hand_items: hand, 
            stash_items: stash, 
            all_tags: payload.all_tags || [] 
          }));
        }
      });

      // 2. 怪物识别触发
      await safeListen<number | null>('trigger-monster-recognition', (dayNum) => {
        console.log("收到自动识别触发事件, Day:", dayNum);
        if (dayNum) {
          const dayLabel = dayNum >= 10 ? "Day 10+" : `Day ${dayNum}`;
          setSelectedDay(dayLabel);
          setCurrentDay(dayNum);
        }
        setTimeout(() => { if (isMounted) handleAutoRecognition(dayNum); }, 500);
      });

      // 3. 卡牌识别触发 (热键)
      await safeListen<void>('hotkey-detect-card', () => {
        console.log("收到卡牌识别触发事件");
        handleRecognizeCard(true);
      });

      // 保留原有的手动触发事件（用于手动按钮触发）
      await safeListen<void>('trigger_yolo_scan', async () => {
        console.log("[Frontend] Received manual trigger_yolo_scan event from backend");
        // 手动触发事件，不受自动扫描设置影响
        const useGpu = localStorage.getItem("use-gpu-acceleration");
        const useGpuBool = useGpu === "true";
        console.log("[Frontend] GPU加速设置:", useGpu, "-> useGpu =", useGpuBool);
        try {
          if ((window as any).__yolo_running) {
            console.log("[Frontend] YOLO scan already running, skipping duplicate call");
            return;
          }
          (window as any).__yolo_running = true;
          console.log("[Frontend] Invoking trigger_yolo_scan with useGpu=", useGpuBool);
          const count = await invoke<number>("trigger_yolo_scan", { useGpu: useGpuBool });
          console.log(`[Frontend] YOLO扫描完成，检测到 ${count} 个目标`);

          // 获取统计信息并通知Overlay更新
          try {
            const stats = await invoke('get_yolo_stats');
            await emit('yolo-stats-updated', stats);
            console.log("[Frontend] YOLO统计信息已更新并发送给Overlay");
          } catch (statsErr) {
            console.error("[Frontend] 获取YOLO统计失败:", statsErr);
          }
        } catch (err) {
          console.error("[Frontend] YOLO扫描失败:", err);
          setErrorMessage(`YOLO识别失败: ${err}`);
          setTimeout(() => setErrorMessage(null), 5000);
        } finally {
          (window as any).__yolo_running = false;
        }
      });

      // 4. 插件折叠/展开 (热键)
      await safeListen<void>('toggle-collapse', () => {
          // 清除调整大小标志，确保 syncLayout 可以执行
          isResizing.current = false;
          setIsCollapsed(prev => !prev);
      });

      // 5. 自动识别并跳转事件
      await safeListen<{ day: number; monster_name: string }>('auto-jump-to-monster', (payload) => {
          const { day, monster_name } = payload;
          const names = monster_name.includes('|') ? monster_name.split('|') : [monster_name];

          // 清除调整大小标志，确保 syncLayout 可以执行
          isResizing.current = false;
          setIsCollapsed(false);
          setCurrentDay(day);
          setSelectedDay(day >= 10 ? "Day 10+" : `Day ${day}`);
          setIdentifiedNames(names);
          setExpandedMonsters(prev => {
              const next = new Set(prev);
              names.forEach((n: string) => next.add(n));
              return next;
          });
          
          setTimeout(() => {
              const element = document.getElementById(`monster-${names[0]}`);
              if (element) element.scrollIntoView({ behavior: 'smooth', block: 'start' });
          }, 100);

          setActiveTab("monster");
      });

      // 6. 野怪匹配事件（来自Overlay右键识别）
      await safeListen<{ name: string; name_zh: string }>('monster-matched', (payload) => {
          console.log("收到野怪匹配事件:", payload);
          // 将识别的野怪名称添加到identifiedNames
          setIdentifiedNames(prev => {
              if (!prev.includes(payload.name)) {
                  return [...prev, payload.name];
              }
              return prev;
          });
          // 展开该野怪
          setExpandedMonsters(prev => {
              const next = new Set(prev);
              next.add(payload.name);
              return next;
          });
          // 切换到野怪选项卡
          setActiveTab("monster");
          // 滚动到该野怪
          setTimeout(() => {
              const element = document.getElementById(`monster-${payload.name}`);
              if (element) element.scrollIntoView({ behavior: 'smooth', block: 'center' });
          }, 300);
      });

      // 5. 天数更新
      await safeListen<number>('day-update', (d) => {
        setCurrentDay(d);
        setSelectedDay(d >= 10 ? "Day 10+" : `Day ${d}`);
      });
      
      // 加载热键设置
      invoke<number | null>("get_detection_hotkey").then(val => isMounted && setDetectionHotkey(val));
      invoke<number | null>("get_card_detection_hotkey").then(val => isMounted && setCardDetectionHotkey(val));
      invoke<number | null>("get_toggle_collapse_hotkey").then(val => isMounted && setToggleCollapseHotkey(val));
      invoke<number | null>("get_detail_display_hotkey").then(val => isMounted && setDetailDisplayHotkey(val));
    };
    
    setupListeners();
    
    // 清理函数
    return () => {
      isMounted = false;
      unlisteners.forEach(fn => fn());
      unlisteners.length = 0;
    };
  }, []); // 移除enableYoloAuto和yoloScanInterval依赖，避免重复注册

  // YOLO扫描函数（提取到外部以便热键调用）
  const runYoloScan = useCallback(async () => {
    const useGpu = localStorage.getItem("use-gpu-acceleration");
    const useGpuBool = useGpu === "true";
    
    try {
      if ((window as any).__yolo_running) {
        console.log("[YOLO Manual/Auto] Scan already running, skipping");
        return;
      }
      (window as any).__yolo_running = true;
      console.log(`[YOLO Manual/Auto] Starting scan (GPU: ${useGpuBool})`);
      const count = await invoke<number>("trigger_yolo_scan", { useGpu: useGpuBool });
      console.log(`[YOLO Manual/Auto] Scan complete, detected ${count} objects`);

      // 获取统计信息并通知Overlay更新
      try {
        const stats = await invoke('get_yolo_stats');
        await emit('yolo-stats-updated', stats);
      } catch (statsErr) {
        console.error("[YOLO Manual/Auto] Failed to get stats:", statsErr);
      }
    } catch (err) {
      console.error("[YOLO Manual/Auto] Scan failed:", err);
    } finally {
      (window as any).__yolo_running = false;
    }
  }, []);

  // YOLO自动扫描定时器 - 单独的useEffect
  useEffect(() => {
    if (!enableYoloAuto) {
      console.log("[YOLO Auto] Auto scan disabled");
      return;
    }

    // 启动定时器
    const yoloTimer = setInterval(runYoloScan, yoloScanInterval * 1000);
    console.log(`[YOLO Auto] Timer started with interval: ${yoloScanInterval}s`);

    // 清理函数
    return () => {
      clearInterval(yoloTimer);
      console.log("[YOLO Auto] Timer stopped");
    };
  }, [enableYoloAuto, yoloScanInterval, runYoloScan]); // 添加runYoloScan依赖

  // YOLO手动触发热键监听
  useEffect(() => {
    // 监听后端发送的YOLO热键事件
    const unlisten = listen('yolo_hotkey_pressed', () => {
      console.log('[YOLO Hotkey] Triggered');
      runYoloScan();
    });
    return () => { unlisten.then(f => f()); };
  }, [runYoloScan]); // 添加runYoloScan依赖

  // 设置YOLO热键到后端
  useEffect(() => {
    if (yoloHotkey) {
      invoke('set_yolo_hotkey', { hotkey: yoloHotkey }).catch(console.error);
    }
  }, [yoloHotkey]);

  // 基础环境侦测：分辨率适配

  // 冲突检测：防止 YOLO 热键和 详情热键 相同
  useEffect(() => {
    if (yoloHotkey && detailDisplayHotkey && yoloHotkey === detailDisplayHotkey) {
      console.warn("[Hotkey] Conflict detected between YOLO and Detail Display. Resetting YOLO hotkey.");
      setYoloHotkey(null);
      localStorage.removeItem("yolo-hotkey");
      invoke('set_yolo_hotkey', { hotkey: null }).catch(console.error);
      showToast("检测到按键冲突，已自动清除 YOLO 热键", "warning");
    }
  }, [yoloHotkey, detailDisplayHotkey]);

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

  // 加载全量怪物数据
  useEffect(() => {
    let retryCount = 0;
    const maxRetries = 15;

    const loadAllMonsters = async () => {
      try {
        const res: Record<string, MonsterData> = await invoke("get_all_monsters");
        // 有数据则更新
        if (res && Object.keys(res).length > 0) {
          console.log(`[Init] Loaded ${Object.keys(res).length} monsters from backend.`);
          setAllMonsters(res);
        } else {
          // 没数据，如果还在重试次数内，则延迟重试
          if (retryCount < maxRetries) {
            retryCount++;
            console.log(`[Init] Monsters DB empty, retrying in 1s (${retryCount}/${maxRetries})...`);
            setTimeout(loadAllMonsters, 1000);
          } else {
            console.warn("[Init] Failed to load monsters after max retries.");
          }
        }
      } catch (e) {
        console.error("加载全量怪物失败:", e);
      }
    };
    loadAllMonsters();
  }, []);

  // Listen for backend signal that monsters DB is ready and reload
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    const setup = async () => {
      try {
        const l = await appWindow.listen('monsters-db-ready', async (event: any) => {
          try {
            console.log('[Event] monsters-db-ready payload:', event.payload);
            const res: Record<string, MonsterData> = await invoke('get_all_monsters');
            setAllMonsters(res);
          } catch (e) {
            console.error('Failed to reload monsters after monsters-db-ready:', e);
          }
        });
        // `l` is the unlisten function returned by `appWindow.listen`
        unlisten = l;
      } catch (e) {
        console.warn('Failed to listen for monsters-db-ready:', e);
      }
    };
    setup();
    return () => { if (unlisten) unlisten(); };
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
      skills: m.skills ? await Promise.all(m.skills.map(async s => {
        // Prefer art_key from skills_db if available
        let imgPath = '';
        try {
          const art = s.id ? skillsArtMap[s.id] : undefined;
          if (art) {
            const base = art.split('/').pop() || art;
            const nameNoExt = base.replace(/\.[^/.]+$/, '');
            imgPath = `images/skill/${nameNoExt}.webp`;
          } else {
            imgPath = `images/${s.id || s.name}.webp`;
          }
        } catch (e) {
          imgPath = `images/${s.id || s.name}.webp`;
        }
        return { ...s, displayImg: await getImg(imgPath) };
      })) : [],
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
    // 如果还在加载几何信息，不执行syncLayout，防止覆盖后端恢复的位置
    if (isLoadingGeometry.current) {
      console.log('[Layout] Skipping syncLayout - still loading geometry from backend');
      return;
    }
    
    const syncLayout = async () => {
      const appWindow = getCurrentWindow();
      
      let logicalScale = 1.0;
      let pX = 0, pY = 0, pWidth = 1920, pHeight = 1080;
      
      try {
        const monitor = await currentMonitor();
        if (monitor && monitor.size) {
          logicalScale = monitor.scaleFactor || 1.0;
          pX = monitor.position.x;
          pY = monitor.position.y;
          pWidth = Math.round(monitor.size.width / logicalScale);
          pHeight = Math.round(monitor.size.height / logicalScale);
        }
      } catch (e) {
        // 使用默认值
      }
      
      currentScale.current = logicalScale;

      let targetW = 0, targetH = 0, targetX = 0, targetY = 0;

      if (showVersionScreen) {
        targetW = 600;
        targetH = 850;
        targetX = Math.round(pX + (pWidth - targetW) / 2);
        targetY = Math.round(pY + (pHeight - targetH) / 2);
      } else {
        targetW = Math.round(Math.min(expandedWidthRef.current, pWidth - 20));
        targetH = Math.round(Math.min(isCollapsed ? 45 : expandedHeightRef.current, pHeight - 40));

        if (hasCustomPosition && lastKnownPosition.current) {
          targetX = Math.round(lastKnownPosition.current.x / logicalScale);
          targetY = Math.round(lastKnownPosition.current.y / logicalScale);
        } else {
          targetX = Math.round(pX + pWidth - targetW);
          targetY = Math.round(pY);
        }
      }

      try {
        if (appWindow.setShadow) await appWindow.setShadow(false);

        const size = await appWindow.innerSize();
        const pos = await appWindow.outerPosition();
        
        // 关键修复：这里的 size 是物理像素，targetW/H 是逻辑像素
        // 必须统一转换为逻辑像素进行比较和设置
        const currentWPhysical = size.width;
        const currentHPhysical = size.height;
        const currentWLogical = Math.round(currentWPhysical / logicalScale);
        const currentHLogical = Math.round(currentHPhysical / logicalScale);
        
        const currentX = Math.round(pos.x / logicalScale);
        const currentY = Math.round(pos.y / logicalScale);

        const layoutKey = `${targetW}-${targetH}-${targetX}-${targetY}`;
        if (lastLayout.current === layoutKey) return;
        lastLayout.current = layoutKey;

        const now = Date.now();
        const recentlyResized = lastUserResize.current && (now - lastUserResize.current < 1000);
        
        // syncLayout 只负责位置和置顶，以及必要时的尺寸调整
        const shouldSkipResize = isResizing.current || recentlyResized || isProgrammaticResize.current;
        
        const widthDiff = Math.abs(currentWLogical - targetW);
        const heightDiff = Math.abs(currentHLogical - targetH);

        // 检查宽度和高度是否需要调整
        if (!shouldSkipResize && (widthDiff > 5 || heightDiff > 5)) {
          isProgrammaticResize.current = true;
          console.log(`[Layout] Resizing: ${currentWLogical}x${currentHLogical} -> ${targetW}x${targetH}`);
          // 关键修复：这里强制设定为 targetW 和 targetH
          await appWindow.setSize(new LogicalSize(targetW, targetH));
          setTimeout(() => { isProgrammaticResize.current = false; }, 200);
        }

        if (!isDragging.current && (Math.abs(currentX - targetX) > 2 || Math.abs(currentY - targetY) > 2)) {
          console.log(`[Layout] Moving: ${currentX},${currentY} -> ${targetX},${targetY}`);
          await appWindow.setPosition(new LogicalPosition(targetX, targetY));
        }

        await appWindow.setAlwaysOnTop(true);
        await appWindow.show();
      } catch (e) {
        console.error("[Layout] Sync failed:", e);
        lastLayout.current = "";
        await appWindow.show().catch(() => {});
      }
    };

    syncLayout();
  }, [showVersionScreen, isCollapsed, hasCustomPosition, isGeometryLoaded]);

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
                console.log("[Update] 用户在开始界面点击立即更新");
                startUpdateDownload();
              }}>
                立即更新
              </button>
            )}
            {updateStatus === "downloading" && (
              <div style={{ width: '100%', textAlign: 'center' }}>
                <div style={{ fontSize: '14px', color: '#58a6ff', marginBottom: '8px' }}>
                  正在下载更新... {downloadProgress}%
                </div>
                <div style={{ 
                  width: '100%', 
                  height: '6px', 
                  background: 'rgba(255, 255, 255, 0.1)', 
                  borderRadius: '3px',
                  overflow: 'hidden'
                }}>
                  <div style={{ 
                    width: `${downloadProgress}%`, 
                    height: '100%', 
                    background: 'linear-gradient(to right, var(--c-golden), #fff)',
                    transition: 'width 0.3s ease'
                  }}></div>
                </div>
              </div>
            )}
            {updateStatus === "ready" && (
              <button className="update-now-btn" onClick={() => {
                console.log("[Update] 更新下载完成，准备安装并重启");
                setIsInstalling(true);
                setTimeout(() => relaunch(), 1000);
              }}>
                更新已就绪，点击安装
              </button>
            )}
            {(updateStatus === "none" || updateStatus === "checking") && (
              <button className="enter-btn" onClick={enterApp}>
                进入插件
              </button>
            )}
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
        // 如果输入框正在输入，则不交还焦点，防止焦点抢夺导致输入打断
        if (isInputFocused) return;
        // 当鼠标划出插件界面时，自动尝试把焦点还给游戏
        invoke("restore_game_focus").catch(() => {});
        invoke("set_overlay_ignore_cursor", { ignore: true }).catch(() => {});
      }}
    >
      {/* 3. 全局错误提示 Toast */}
      {errorMessage && (
        <div className="error-toast" style={{
          position: 'fixed',
          top: '80px', // Lowered position
          left: '50%',
          transform: 'translateX(-50%)',
          backgroundColor: 'rgba(40, 35, 30, 0.95)',
          color: '#ff6b6b',
          border: '1px solid #ff4d4f',
          padding: '12px 24px',
          borderRadius: '8px',
          boxShadow: '0 8px 24px rgba(0,0,0,0.6)',
          zIndex: 9999,
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          fontSize: '14px',
          fontWeight: 600,
          backdropFilter: 'blur(5px)',
          animation: 'slideDown 0.3s ease-out'
        }}>
          <span style={{ fontSize: '18px' }}>⚠️</span>
          <span>{errorMessage}</span>
          <button 
            onClick={() => setErrorMessage(null)}
            style={{ 
              background: 'transparent', 
              border: 'none', 
              color: '#888', 
              cursor: 'pointer',
              fontSize: '18px',
              marginLeft: '8px',
              lineHeight: 1
            }}
          >
            ×
          </button>
        </div>
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
        
        <div className="collapse-btn" onClick={async () => {
          if (expandedHeightRef.current < 200) {
            expandedHeightRef.current = 700;
            setExpandedHeight(700);
          }
          
          // 切换状态
          const newCollapsed = !isCollapsed;
          
          // 立即调整尺寸，使用 ref 中的宽度（用户调整时已保存）
          try {
            const appWindow = getCurrentWindow();
            const targetW = expandedWidthRef.current;
            const targetH = newCollapsed ? 45 : expandedHeightRef.current;
            
            // 标记为程序调整，避免触发 resize 监听器
            isProgrammaticResize.current = true;
            await appWindow.setSize(new LogicalSize(targetW, targetH));
            setTimeout(() => { isProgrammaticResize.current = false; }, 300);
            
            // 改变状态（会触发 syncLayout，但 syncLayout 会因为 isProgrammaticResize 跳过）
            setIsCollapsed(newCollapsed);
          } catch (e) {
            console.error('Failed to resize on collapse/expand:', e);
            setIsCollapsed(newCollapsed);
          }
        }}>
          {isCollapsed ? "展开" : "收起"}
          <span className={`collapse-arrow ${isCollapsed ? 'collapsed' : 'expanded'}`}>▾</span>
        </div>
        
        <button className="close-btn" onClick={() => exit(0)} title="关闭">
          <svg className="close-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M18 6L6 18M6 6L18 18" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </button>

        {/* 原生调整宽度的隐藏把手（右侧）和右下角把手，用于 frameless 窗口的 startResizing */}
        <div
          className="resize-handle-right"
          onMouseDown={() => {
            try { (appWindow as any).startResizing('Right'); } catch (e) { console.error('startResizing Right failed', e); }
          }}
        />
        <div
          className="resize-handle-br"
          onMouseDown={() => {
            try { (appWindow as any).startResizing('BottomRight'); } catch (e) { console.error('startResizing BottomRight failed', e); }
          }}
        />
      </div>

      {showSettings && (
        <div className="settings-panel-overlay" onClick={() => setShowSettings(false)}>
          <div className="settings-panel" onClick={e => e.stopPropagation()}>
            <div className="settings-header">
              <h3>应用设置</h3>
              <button className="close-panel-btn" onClick={() => setShowSettings(false)}>×</button>
            </div>
            <div className="settings-content">
              
              {/* 界面设置分组 */}
              <SettingGroup
                title="⚙️ 界面设置"
                expanded={settingsExpanded.ui}
                onToggle={() => setSettingsExpanded(prev => ({ ...prev, ui: !prev.ui }))}
              >
                {/* 字体大小 */}
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                    <label>字体大小: {fontSize}px</label>
                    <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={() => {
                      setFontSize(16);
                      localStorage.setItem("user-font-size", "16");
                      showToast('字体大小已重置', 'success');
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

                {/* 窗口布局 */}
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                    <label>窗口布局</label>
                    <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={() => {
                      localStorage.removeItem("plugin-width");
                      localStorage.removeItem("plugin-height");
                      setExpandedWidth(400);
                      setExpandedHeight(700);
                      setHasCustomPosition(false);
                      showToast('窗口布局已重置', 'success');
                    }}>重置宽高与位置</button>
                  </div>
                  <div className="setting-tip">调整后将实时影响所有文字大小</div>
                </div>

                {/* 详情弹窗位置 */}
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>详情弹窗位置</label>
                    <button className="bulk-btn" style={{ padding: '4px 12px' }} onClick={async () => {
                      try {
                        await invoke('reset_detail_popup_position');
                        showToast('详情弹窗位置已重置', 'success');
                      } catch (e) {
                        console.error("Failed to reset detail popup position:", e);
                        showToast('重置失败', 'error');
                      }
                    }}>重置位置</button>
                  </div>
                  <div style={{ fontSize: '11px', color: '#888', marginTop: '8px' }}>
                    重置详情弹窗到默认位置（鼠标所在屏幕的中心）
                  </div>
                </div>

                {/* 野怪识别校准 - 暂时禁用，使用固定裁剪
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>野怪识别校准</label>
                    <button 
                      className="bulk-btn" 
                      style={{ 
                        padding: '4px 12px',
                        background: 'linear-gradient(135deg, rgba(255, 205, 25, 0.15), rgba(255, 180, 25, 0.1))',
                        borderColor: 'rgba(255, 205, 25, 0.5)',
                        color: '#ffcd19',
                        fontWeight: 'bold'
                      }} 
                      onClick={async () => {
                        try {
                          await invoke('open_calibration_window');
                          showToast('校准窗口已打开', 'info');
                        } catch (err) {
                          console.error('[Settings] Failed to open calibration window:', err);
                          showToast('打开校准窗口失败: ' + err, 'error');
                        }
                      }}
                    >
                      开始校准
                    </button>
                  </div>
                  <div style={{ fontSize: '11px', color: '#888', marginTop: '8px' }}>
                    校准三个野怪识别区域，用于一键识别所有野怪功能
                  </div>
                </div>
                */}
              </SettingGroup>
              
              {/* YOLO设置分组 */}
              <SettingGroup
                title="🔍 YOLO设置"
                expanded={settingsExpanded.yolo}
                onToggle={() => setSettingsExpanded(prev => ({ ...prev, yolo: !prev.yolo }))}
              >
                {/* YOLO自动识别 */}
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>YOLO自动识别</label>
                    <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                      {enableYoloAuto && (
                        <button 
                          className="bulk-btn" 
                          style={{ 
                            padding: '4px 12px',
                            background: useGpuAcceleration ? 'rgba(76, 175, 80, 0.2)' : 'rgba(244, 67, 54, 0.2)',
                            borderColor: useGpuAcceleration ? '#4CAF50' : '#f44336',
                            color: useGpuAcceleration ? '#4CAF50' : '#f44336'
                          }} 
                          onClick={() => {
                            const newVal = !useGpuAcceleration;
                            setUseGpuAcceleration(newVal);
                            localStorage.setItem("use-gpu-acceleration", newVal.toString());
                            showToast(`GPU加速已${newVal ? '开启' : '关闭'}`, 'info');
                          }}
                        >
                          GPU加速: {useGpuAcceleration ? '开' : '关'}
                        </button>
                      )}
                      <button 
                        className="bulk-btn" 
                        style={{ 
                          padding: '4px 12px',
                          background: enableYoloAuto ? 'rgba(76, 175, 80, 0.2)' : 'rgba(244, 67, 54, 0.2)',
                          borderColor: enableYoloAuto ? '#4CAF50' : '#f44336',
                          color: enableYoloAuto ? '#4CAF50' : '#f44336'
                        }} 
                        onClick={() => {
                          const newVal = !enableYoloAuto;
                          setEnableYoloAuto(newVal);
                          localStorage.setItem("enable-yolo-auto", newVal.toString());
                          showToast(`YOLO自动识别已${newVal ? '开启' : '关闭'}`, 'info');
                        }}
                      >
                        {enableYoloAuto ? '已开启' : '已关闭'}
                      </button>
                    </div>
                  </div>
                  <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                    启用后每隔固定时间自动触发YOLO识别卡牌（下方可调整频率）
                  </div>
                </div>
                
                {/* YOLO扫描频率设置 */}
                <div className="setting-item" style={{ opacity: enableYoloAuto ? 1 : 0.5 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>YOLO扫描频率</label>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <input 
                        type="range"
                        min="0.5"
                        max="2"
                        step="0.1"
                        value={yoloScanInterval}
                        disabled={!enableYoloAuto}
                        onChange={(e) => {
                          const newVal = parseFloat(e.target.value);
                          setYoloScanInterval(newVal);
                          localStorage.setItem("yolo-scan-interval", newVal.toString());
                        }}
                        style={{
                          width: '120px',
                          accentColor: '#ffcd19'
                        }}
                      />
                      <span style={{ 
                        fontSize: '13px', 
                        color: '#ffcd19', 
                        fontWeight: 'bold',
                        minWidth: '50px'
                      }}>
                        {yoloScanInterval.toFixed(1)}s
                      </span>
                    </div>
                  </div>
                  <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                    设置YOLO自动识别的时间间隔（0.5秒 - 2秒）
                  </div>
                </div>
                
                {/* YOLO实时监控 */}
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>YOLO实时监控</label>
                    <button 
                      className="bulk-btn" 
                      style={{ 
                        padding: '4px 12px',
                        background: showYoloMonitor ? 'rgba(76, 175, 80, 0.2)' : 'rgba(244, 67, 54, 0.2)',
                        borderColor: showYoloMonitor ? '#4CAF50' : '#f44336',
                        color: showYoloMonitor ? '#4CAF50' : '#f44336'
                      }} 
                      onClick={() => {
                        const newVal = !showYoloMonitor;
                        setShowYoloMonitor(newVal);
                        localStorage.setItem("show-yolo-monitor", newVal.toString());
                        try {
                          invoke('set_show_yolo_monitor', { show: newVal }).catch(console.error);
                        } catch (e) { console.error(e); }
                        showToast(`YOLO监控已${newVal ? '显示' : '隐藏'}`, 'info');
                      }}
                    >
                      {showYoloMonitor ? '隐藏' : '显示'}
                    </button>
                  </div>
                  <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                    显示/隐藏YOLO实时监控窗口，用于查看识别结果
                  </div>
                </div>

                {/* YOLO手动触发快捷键设置 */}
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>YOLO手动触发快捷键</label>
                    <button 
                      className="bulk-btn" 
                      style={{ padding: '2px 8px' }}
                      onClick={(e) => {
                        e.preventDefault();
                        setIsRecordingYoloHotkey(true);
                      }}
                    >
                      {isRecordingYoloHotkey ? "请按键..." : (yoloHotkey ? getHotkeyLabel(yoloHotkey) : "未设置")}
                    </button>
                  </div>
                  {isRecordingYoloHotkey && (
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
                        // 禁止左键和右键
                        if (e.button === 0 || e.button === 2) {
                          return;
                        }
                        let vk = 0;
                        switch(e.button) {
                          case 1: vk = 4; break;
                          case 3: vk = 5; break;
                          case 4: vk = 6; break;
                        }
                        if (vk > 0) {
                          setYoloHotkey(vk);
                          localStorage.setItem("yolo-hotkey", vk.toString());
                          setIsRecordingYoloHotkey(false);
                        }
                      }}
                      onKeyDown={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        if (e.keyCode) {
                          setYoloHotkey(e.keyCode);
                          localStorage.setItem("yolo-hotkey", e.keyCode.toString());
                          setIsRecordingYoloHotkey(false);
                        }
                      }}
                      tabIndex={0}
                      ref={(el) => el?.focus()}
                    >
                      <div style={{ fontSize: '20px', marginBottom: '10px' }}>请按下新的热键</div>
                      <div style={{ fontSize: '14px', color: '#aaa' }}>支持: 键盘按键, 鼠标中键/侧键（不支持左右键）</div>
                      <button 
                        style={{ marginTop: '20px', padding: '5px 15px' }}
                        onClick={(e) => { e.stopPropagation(); setIsRecordingYoloHotkey(false); }}
                      >取消</button>
                    </div>
                  )}
                  <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                    按此键立即触发YOLO识别（默认: 未设置）
                  </div>
                </div>

                {/* 详情显示热键设置 */}
                <div className="setting-item">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>卡牌详情显示按键</label>
                    <button 
                      className="bulk-btn" 
                      style={{ padding: '2px 8px' }}
                      onClick={(e) => {
                        e.preventDefault();
                        setIsRecordingDetailHotkey(true);
                      }}
                    >
                      {isRecordingDetailHotkey ? "请按键..." : (detailDisplayHotkey ? getHotkeyLabel(detailDisplayHotkey) : "未设置")}
                    </button>
                  </div>
                  {isRecordingDetailHotkey && (
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
                          setDetailDisplayHotkey(vk);
                          invoke("set_detail_display_hotkey", { hotkey: vk });
                          setIsRecordingDetailHotkey(false);
                        }
                      }}
                      onKeyDown={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        if (e.keyCode) {
                          setDetailDisplayHotkey(e.keyCode);
                          invoke("set_detail_display_hotkey", { hotkey: e.keyCode });
                          setIsRecordingDetailHotkey(false);
                        }
                      }}
                      tabIndex={0}
                      ref={(el) => el?.focus()}
                    >
                      <div style={{ fontSize: '20px', marginBottom: '10px' }}>请按下新的热键</div>
                      <div style={{ fontSize: '14px', color: '#aaa' }}>支持: 键盘按键, 鼠标左/中/右键/侧键</div>
                      <button 
                        style={{ marginTop: '20px', padding: '5px 15px' }}
                        onClick={(e) => { e.stopPropagation(); setIsRecordingDetailHotkey(false); }}
                      >取消</button>
                    </div>
                  )}
                  <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                    按此键显示鼠标位置的卡牌/怪物/事件详情（默认: 未设置）
                  </div>
                </div>
              </SettingGroup>
              
              {/* 快捷键设置分组 */}
              <SettingGroup
                title="⌨️ 快捷键设置"
                expanded={settingsExpanded.hotkeys}
                onToggle={() => setSettingsExpanded(prev => ({ ...prev, hotkeys: !prev.hotkeys }))}
              >

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
                <div className="setting-tip">默认: 未设置</div>
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
                <div className="setting-tip">默认: 未设置</div>
              </div>

              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                  <label>一键收起/展开插件</label>
                  <button 
                    className="bulk-btn" 
                    style={{ padding: '2px 8px' }}
                    onClick={(e) => {
                      e.preventDefault();
                      setIsRecordingToggleHotkey(true);
                    }}
                  >
                    {isRecordingToggleHotkey ? "请按键..." : (toggleCollapseHotkey ? getHotkeyLabel(toggleCollapseHotkey) : "未设置")}
                  </button>
                </div>
                {isRecordingToggleHotkey && (
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
                      // 禁止左键和右键
                      if (e.button === 0 || e.button === 2) {
                        return;
                      }
                      let vk = 0;
                      switch(e.button) {
                        case 1: vk = 4; break;
                        case 3: vk = 5; break;
                        case 4: vk = 6; break;
                      }
                      if (vk > 0) {
                        setToggleCollapseHotkey(vk);
                        invoke("set_toggle_collapse_hotkey", { hotkey: vk });
                        setIsRecordingToggleHotkey(false);
                      }
                    }}
                    onKeyDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (e.keyCode) {
                        setToggleCollapseHotkey(e.keyCode);
                        invoke("set_toggle_collapse_hotkey", { hotkey: e.keyCode });
                        setIsRecordingToggleHotkey(false);
                      }
                    }}
                    tabIndex={0}
                    ref={(el) => el?.focus()}
                  >
                    <div style={{ fontSize: '20px', marginBottom: '10px' }}>请按下新的热键</div>
                    <div style={{ fontSize: '14px', color: '#aaa' }}>支持: 键盘按键, 鼠标中键/侧键（不支持左右键）</div>
                    <button 
                      style={{ marginTop: '20px', padding: '5px 15px' }}
                      onClick={(e) => { e.stopPropagation(); setIsRecordingToggleHotkey(false); }}
                    >取消</button>
                  </div>
                )}
                <div className="setting-tip">默认: 未设置</div>
              </div>

              <div className="setting-divider" style={{ borderTop: '1px solid rgba(255,255,255,0.1)', margin: '15px 0' }}></div>

              {/* 重置所有热键 */}
              <div className="setting-item">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <label>快捷键管理</label>
                  <button className="bulk-btn" style={{ padding: '4px 12px', background: 'rgba(255, 69, 58, 0.15)', borderColor: 'rgba(255, 69, 58, 0.4)' }} onClick={() => {
                    setShowResetHotkeysConfirm(true);
                  }}>重置所有快捷键</button>
                </div>
                <div style={{ fontSize: '11px', color: '#888', marginTop: '8px' }}>
                  将所有快捷键重置为"未设置"状态，禁用所有快捷键功能
                </div>
              </div>
              
              {/* 自定义确认对话框 */}
              {showResetHotkeysConfirm && (
                <div style={{
                  position: 'fixed',
                  top: 0,
                  left: 0,
                  right: 0,
                  bottom: 0,
                  background: 'rgba(0, 0, 0, 0.85)',
                  backdropFilter: 'blur(8px)',
                  display: 'flex',
                  justifyContent: 'center',
                  alignItems: 'center',
                  zIndex: 10000,
                  animation: 'fadeIn 0.2s ease'
                }}>
                  <div style={{
                    background: 'linear-gradient(135deg, rgba(20, 18, 15, 0.98) 0%, rgba(30, 25, 20, 0.98) 100%)',
                    border: '2px solid rgba(255, 205, 25, 0.4)',
                    borderRadius: '12px',
                    padding: '24px',
                    maxWidth: '420px',
                    width: '90%',
                    boxShadow: '0 16px 48px rgba(0, 0, 0, 0.9), 0 0 32px rgba(255, 205, 25, 0.15)',
                    animation: 'scaleIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1)'
                  }}>
                    {/* 标题 */}
                    <div style={{
                      fontSize: '20px',
                      fontWeight: 'bold',
                      color: '#ffcd19',
                      marginBottom: '16px',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '10px',
                      textShadow: '0 2px 8px rgba(255, 205, 25, 0.3)'
                    }}>
                      <span style={{ fontSize: '24px' }}>⚠️</span>
                      <span>重置确认</span>
                    </div>
                    
                    {/* 内容 */}
                    <div style={{
                      fontSize: '14px',
                      color: '#ddd',
                      lineHeight: '1.6',
                      marginBottom: '24px',
                      padding: '16px',
                      background: 'rgba(0, 0, 0, 0.3)',
                      borderRadius: '8px',
                      border: '1px solid rgba(255, 205, 25, 0.1)'
                    }}>
                      <p style={{ margin: '0 0 12px 0' }}>
                        此操作将<span style={{ color: '#ff6b6b', fontWeight: 'bold' }}>重置所有快捷键设置</span>，包括：
                      </p>
                      <ul style={{ margin: '8px 0', paddingLeft: '24px', color: '#aaa' }}>
                        <li>怪物识别热键</li>
                        <li>卡牌识别热键</li>
                        <li>YOLO 扫描热键</li>
                        <li>详情显示热键</li>
                        <li>折叠/展开热键</li>
                      </ul>
                      <p style={{ margin: '12px 0 0 0', color: '#888', fontSize: '13px' }}>
                        重置后，所有快捷键功能将被禁用，您需要重新设置才能使用。
                      </p>
                    </div>
                    
                    {/* 按钮组 */}
                    <div style={{
                      display: 'flex',
                      gap: '12px',
                      justifyContent: 'flex-end'
                    }}>
                      <button
                        style={{
                          padding: '10px 24px',
                          background: 'rgba(255, 255, 255, 0.1)',
                          border: '1px solid rgba(255, 255, 255, 0.2)',
                          borderRadius: '6px',
                          color: '#fff',
                          fontSize: '14px',
                          cursor: 'pointer',
                          transition: 'all 0.2s ease',
                          fontWeight: '500'
                        }}
                        onClick={() => setShowResetHotkeysConfirm(false)}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.background = 'rgba(255, 255, 255, 0.15)';
                          e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.3)';
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)';
                          e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.2)';
                        }}
                      >
                        取消
                      </button>
                      <button
                        style={{
                          padding: '10px 24px',
                          background: 'linear-gradient(135deg, rgba(255, 69, 58, 0.8), rgba(255, 59, 48, 0.9))',
                          border: '1px solid rgba(255, 69, 58, 0.6)',
                          borderRadius: '6px',
                          color: '#fff',
                          fontSize: '14px',
                          cursor: 'pointer',
                          transition: 'all 0.2s ease',
                          fontWeight: '600',
                          boxShadow: '0 4px 12px rgba(255, 69, 58, 0.3)'
                        }}
                        onClick={async () => {
                          setShowResetHotkeysConfirm(false);
                          try {
                      await invoke('reset_all_hotkeys');
                      // 重新加载所有热键设置
                      const detection = await invoke<number | null>("get_detection_hotkey");
                      const card = await invoke<number | null>("get_card_detection_hotkey");
                      const toggle = await invoke<number | null>("get_toggle_collapse_hotkey");
                      const yolo = await invoke<number | null>("get_yolo_hotkey");
                      const detail = await invoke<number | null>("get_detail_display_hotkey");
                      
                      setDetectionHotkey(detection || 0);
                      setCardDetectionHotkey(card || 0);
                      setToggleCollapseHotkey(toggle || 0);
                      setYoloHotkey(yolo || 0);
                      setDetailDisplayHotkey(detail || 0);
                      
                      // 清除 localStorage
                      localStorage.removeItem("detection-hotkey");
                      localStorage.removeItem("card-detection-hotkey");
                      localStorage.removeItem("toggle-collapse-hotkey");
                      localStorage.removeItem("yolo-hotkey");
                      localStorage.removeItem("detail-display-hotkey");
                      
                      showToast("所有快捷键已重置", 'success');
                    } catch (e) {
                      console.error("Failed to reset hotkeys:", e);
                      showToast("重置失败", 'error');
                    }
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 79, 68, 0.9), rgba(255, 69, 58, 1))';
                          e.currentTarget.style.boxShadow = '0 6px 16px rgba(255, 69, 58, 0.4)';
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 69, 58, 0.8), rgba(255, 59, 48, 0.9))';
                          e.currentTarget.style.boxShadow = '0 4px 12px rgba(255, 69, 58, 0.3)';
                        }}
                      >
                        确认重置
                      </button>
                    </div>
                  </div>
                </div>
              )}
              </SettingGroup>

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

              {/* 赞助与支持 */}
              <div className="setting-item" style={{ marginTop: '20px', textAlign: 'center' }}>
                <label style={{ display: 'block', marginBottom: '12px', color: '#ffcd19', fontSize: '14px', fontWeight: 'bold' }}>赞助与支持 (Sponsor)</label>
                <div style={{ display: 'flex', justifyContent: 'space-around', alignItems: 'flex-start', flexWrap: 'wrap', gap: '20px' }}>
                    {sponsorIcons.vx && (
                        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
                            <img src={sponsorIcons.vx} alt="WeChat" style={{ width: '180px', borderRadius: '8px', boxShadow: '0 4px 12px rgba(0,0,0,0.5)' }} />
                            <span style={{ fontSize: '12px', color: '#888' }}>微信 (WeChat)</span>
                        </div>
                    )}
                    {sponsorIcons.zfb && (
                        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
                            <img src={sponsorIcons.zfb} alt="Alipay" style={{ width: '180px', borderRadius: '8px', boxShadow: '0 4px 12px rgba(0,0,0,0.5)' }} />
                            <span style={{ fontSize: '12px', color: '#888' }}>支付宝 (Alipay)</span>
                        </div>
                    )}
                </div>
                <div style={{ fontSize: '11px', color: '#666', marginTop: '12px' }}>
                  如果这个工具对你有帮助，欢迎请作者喝杯咖啡 ☕
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {!isCollapsed && (
        <>
          {/* 更新按钮 */}
          <nav className="nav-bar">
            {(["monster", "card", "items", "search"] as TabType[]).map(t => (
              <div key={t} className={`nav-item ${activeTab === t ? 'active' : ''}`} onClick={() => setActiveTab(t)}>
                {t === 'monster' ? '野怪一览' : t === 'card' ? '卡牌识别' : t === 'items' ? '手头物品' : '百科搜索'}
              </div>
            ))}
          </nav>

          {activeTab === "search" && (
            <div className="search-box-container" style={{ 
              zIndex: 100,
              borderBottom: '1px solid rgba(255,255,255,0.1)', 
              background: '#2b2621',
              boxShadow: '0 4px 6px rgba(0,0,0,0.3)',
              display: 'flex',
              flexDirection: 'column',
              height: isSearchFilterCollapsed ? 'auto' : `${searchFilterHeight}px`,
              position: 'relative'
            }}>
              <div style={{ 
                padding: '12px', 
                display: 'flex', 
                flexDirection: 'column', 
                gap: '8px', 
                overflowY: 'auto', 
                flex: 1,
                scrollbarWidth: 'thin',
                scrollbarColor: '#ffcd19 rgba(0,0,0,0.3)'
              }} className="custom-scrollbar">
              {/* Header row with collapse button */}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                  <div style={{ fontSize: '12px', color: '#ffcd19', fontWeight: 'bold' }}>搜索过滤器</div>
                  <div style={{ display: 'flex', gap: '4px' }}>
                    <button
                      onClick={() => setMatchMode('all')}
                      className={`toggle-btn ${matchMode === 'all' ? 'active' : ''}`}
                      style={{
                        padding: '4px 8px',
                        fontSize: '11px',
                        borderRadius: '4px',
                        background: matchMode === 'all' ? '#ffcd19' : 'transparent',
                        color: matchMode === 'all' ? '#1e1b18' : '#ffcd19',
                        border: '1px solid #ffcd19',
                        cursor: 'pointer'
                      }}
                      title="所有筛选项必须同时满足"
                    >
                      匹配所有
                    </button>
                    <button
                      onClick={() => setMatchMode('any')}
                      className={`toggle-btn ${matchMode === 'any' ? 'active' : ''}`}
                      style={{
                        padding: '4px 8px',
                        fontSize: '11px',
                        borderRadius: '4px',
                        background: matchMode === 'any' ? '#ffcd19' : 'transparent',
                        color: matchMode === 'any' ? '#1e1b18' : '#ffcd19',
                        border: '1px solid #ffcd19',
                        cursor: 'pointer'
                      }}
                      title="满足任意一个筛选项即可"
                    >
                      匹配任一
                    </button>
                  </div>
                </div>
                <button 
                  onClick={() => setIsSearchFilterCollapsed(!isSearchFilterCollapsed)}
                  style={{
                    background: 'transparent',
                    border: '1px solid rgba(255,205,25,0.3)',
                    color: '#ffcd19',
                    padding: '4px 8px',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    fontSize: '11px'
                  }}
                >
                  {isSearchFilterCollapsed ? '展开 ▼' : '收起 ▲'}
                </button>
              </div>

              {!isSearchFilterCollapsed && (
                <>
              {/* Row 1: Keyword + Type */}
                <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                  <input 
                    className="search-input"
                    placeholder="搜索名称 / 描述..." 
                    value={searchQuery.keyword}
                    onChange={e => setSearchQuery({...searchQuery, keyword: e.target.value})}
                    onFocus={() => {
                        setIsInputFocused(true);
                        // 确保获得焦点时输入法不被鼠标穿透逻辑干扰
                        invoke("set_overlay_ignore_cursor", { ignore: false }).catch(() => {});
                    }}
                    onBlur={() => {
                        setIsInputFocused(false);
                    }}
                    style={{ 
                      flex: 1, 
                      minWidth: '200px',
                      background: '#1e1b18', 
                      border: '1px solid #48413a', 
                      color: '#eee', 
                      padding: '8px 12px', 
                      borderRadius: '4px',
                      fontSize: '14px'
                    }}
                 />
                 
              </div>

              {/* Row 2: Type, Size, Tier, Hero - button groups (single-choice) */}
              <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                 <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                   {[
                     {val: 'item', label: '物品'},
                     {val: 'skill', label: '技能'}
                   ].map(opt => (
                     <button key={opt.val}
                       className={`toggle-btn ${searchQuery.item_type === opt.val ? 'active' : ''}`}
                       onClick={() => {
                         if (searchQuery.item_type === opt.val) {
                           // Toggle off: set to 'all', 恢复尺寸
                           setSearchQuery({...searchQuery, item_type: 'all', size: opt.val === 'skill' ? lastItemSize : searchQuery.size});
                         } else if (opt.val === 'skill') {
                           // 切换到技能：记住当前尺寸，设置为medium -> 改为"" (不筛选尺寸)
                           setLastItemSize(searchQuery.size);
                           setSearchQuery({...searchQuery, item_type: opt.val, size: ""});
                         } else {
                           // 切换到物品：恢复之前的尺寸选择
                           const restoredSize = searchQuery.item_type === 'skill' ? lastItemSize : searchQuery.size;
                           setSearchQuery({...searchQuery, item_type: opt.val, size: restoredSize});
                         }
                       }}
                       style={{ padding: '6px 10px', borderRadius: 6 }}
                     >{opt.label}</button>
                   ))}
                 </div>

                 {searchQuery.item_type !== 'skill' && (
                   <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                     {[
                       {val: 'small', label: '小'},
                       {val: 'medium', label: '中'},
                       {val: 'large', label: '大'}
                     ].map(opt => (
                       <button key={opt.val}
                         className={`toggle-btn ${searchQuery.size === opt.val ? 'active' : ''}`}
                         onClick={() => setSearchQuery({...searchQuery, size: searchQuery.size === opt.val ? '' : opt.val})}
                         style={{ padding: '6px 10px', borderRadius: 6 }}
                       >{opt.label}</button>
                     ))}
                   </div>
                 )}
              </div>

              {/* Row 3: Tier and Hero - Always on separate line */}
              <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                 <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                   {[
                     {val: 'bronze', label: '青铜', color: '#cd7f32'},
                     {val: 'silver', label: '白银', color: '#c0c0c0'},
                     {val: 'gold', label: '黄金', color: '#ffd700'},
                     {val: 'diamond', label: '钻石', color: '#b9f2ff'},
                     {val: 'legendary', label: '传说', color: '#ff4500'}
                   ].map(opt => (
                     <button key={opt.val}
                       className={`toggle-btn ${searchQuery.start_tier === opt.val ? 'active' : ''}`}
                       onClick={() => setSearchQuery({...searchQuery, start_tier: searchQuery.start_tier === opt.val ? '' : opt.val})}
                       style={{ padding: '6px 10px', borderRadius: 6, color: opt.color }}
                     >{opt.label}</button>
                   ))}
                 </div>

                 <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                   {[
                     {val: 'Common', label: '通用', color: '#E0E0E0', avatar: ''},
                     {val: 'Pygmalien', label: '猪', color: '#5BA3FF', avatar: '/images/heroes/pygmalien.webp'},
                     {val: 'Jules', label: '朱尔斯', color: '#D77EFF', avatar: '/images/heroes/jules.webp'},
                     {val: 'Vanessa', label: '瓦内莎', color: '#FF6B6B', avatar: '/images/heroes/vanessa.webp'},
                     {val: 'Mak', label: '马克', color: '#D4FF85', avatar: '/images/heroes/mak.webp'},
                     {val: 'Dooley', label: '多利', color: '#FFC048', avatar: '/images/heroes/dooley.webp'},
                     {val: 'Stelle', label: '斯黛尔', color: '#FFE74C', avatar: '/images/heroes/stelle.webp'}
                   ].map(opt => (
                     <button key={opt.val}
                       className={`toggle-btn ${opt.avatar ? 'hero-btn' : ''} ${searchQuery.hero === opt.val ? 'active' : ''}`}
                       onClick={() => setSearchQuery({...searchQuery, hero: searchQuery.hero === opt.val ? '' : opt.val})}
                       title={opt.label}
                     >
                       {opt.avatar ? <img src={opt.avatar} alt={opt.label} /> : opt.label}
                     </button>
                   ))}
                 </div>
              </div>

              {/* Row 4: Tags & Hidden Tags - Multi-select buttons */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                <div style={{ fontSize: '11px', color: '#888' }}>标签 (可多选)</div>
                <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
                  {[
                    ["Drone", "无人机"], 
                    ["Property", "地产"], 
                    ["Ray", "射线"], 
                    ["Tool", "工具"], 
                    ["Dinosaur", "恐龙"], 
                    ["Loot", "战利品"], 
                    ["Apparel", "服饰"], 
                    ["Core", "核心"], 
                    ["Weapon", "武器"], 
                    ["Aquatic", "水系"], 
                    ["Toy", "玩具"], 
                    ["Tech", "科技"], 
                    ["Potion", "药水"], 
                    ["Reagent", "原料"], 
                    ["Vehicle", "载具"], 
                    ["Relic", "遗物"], 
                    ["Food", "食物"], 
                    ["Dragon", "龙"],
                    ["Friend", "伙伴"]
                  ].sort((a,b) => a[1].localeCompare(b[1], 'zh-CN')).map(([val, label]) => (
                    <button key={val}
                      className={`toggle-btn ${selectedTags.includes(val) ? 'active' : ''}`}
                      onClick={() => {
                        if (selectedTags.includes(val)) {
                          setSelectedTags(selectedTags.filter(t => t !== val));
                        } else {
                          setSelectedTags([...selectedTags, val]);
                        }
                      }}
                      style={{ padding: '6px 10px', borderRadius: 6, fontSize: '12px' }}
                    >{label}</button>
                  ))}
                </div>
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                <div style={{ fontSize: '11px', color: '#888' }}>隐藏标签 (可多选)</div>
                <div style={{ display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                  {/* 定义分组和图标映射 */}
                  {(() => {
                    const tagGroups = [
                      { tags: [["Ammo", "弹药"], ["AmmoRef", "弹药相关"]], icon: "Ammo", color: "var(--c-ammo)" },
                      { tags: [["Burn", "灼烧"], ["BurnRef", "灼烧相关"]], icon: "Burn", color: "var(--c-burn)" },
                      { tags: [["Charge", "充能"]], icon: "Charge", color: "var(--c-charge)" },
                      { tags: [["Cooldown", "冷却"], ["CooldownReference", "冷却相关"]], icon: "Cooldown", color: "var(--c-cooldown)" },
                      { tags: [["Crit", "暴击"], ["CritRef", "暴击相关"]], icon: "CritChance", color: "var(--c-crit)" },
                      { tags: [["Damage", "伤害"], ["DamageRef", "伤害相关"]], icon: "Damage", color: "var(--c-damage)" },
                      { tags: [["EconomyRef", "经济相关"], ["Gold", "金币"]], icon: "Income", color: "var(--c-golden)" },
                      { tags: [["Flying", "飞行"], ["FlyingRef", "飞行相关"]], icon: "Flying", color: "var(--c-fly)" },
                      { tags: [["Freeze", "冻结"], ["FreezeRef", "冻结相关"]], icon: "Freeze", color: "var(--c-freeze)" },
                      { tags: [["Haste", "加速"], ["HasteRef", "加速相关"]], icon: "Haste", color: "var(--c-haste)" },
                      { tags: [["Heal", "治疗"], ["HealRef", "治疗相关"]], icon: "Health", color: "var(--c-heal)" },
                      { tags: [["Health", "生命值"], ["HealthRef", "生命值相关"]], icon: "MaxHPHeart", color: "var(--c-heal)" },
                      { tags: [["Lifesteal", "生命偷取"]], icon: "Lifesteal", color: "var(--c-lifesteal)" },
                      { tags: [["Poison", "剧毒"], ["PoisonRef", "剧毒相关"]], icon: "Poison", color: "var(--c-poison)" },
                      { tags: [["Quest", "任务"]], icon: null, color: "#9098fe" },
                      { tags: [["Regen", "再生"], ["RegenRef", "再生相关"]], icon: "Regen", color: "var(--c-regen)" },
                      { tags: [["Shield", "护盾"], ["ShieldRef", "护盾相关"]], icon: "Shield", color: "var(--c-shield)" },
                      { tags: [["Slow", "减速"], ["SlowRef", "减速相关"]], icon: "Slowness", color: "var(--c-slow)" },
                    ];

                    return tagGroups.map((group, groupIndex) => (
                      <div key={groupIndex} style={{ display: 'flex', gap: '2px', alignItems: 'center' }}>
                        {group.tags.map(([val, label], index) => (
                          <button key={val}
                            className={`toggle-btn ${selectedHiddenTags.includes(val) ? 'active' : ''}`}
                            onClick={() => {
                              if (selectedHiddenTags.includes(val)) {
                                setSelectedHiddenTags(selectedHiddenTags.filter(t => t !== val));
                              } else {
                                setSelectedHiddenTags([...selectedHiddenTags, val]);
                              }
                            }}
                            style={{ 
                              padding: '6px 10px', 
                              borderRadius: 6, 
                              fontSize: '12px',
                              color: group.color,
                              display: 'flex',
                              alignItems: 'center',
                              gap: '4px'
                            }}
                          >
                            {index === 0 && group.icon && hiddenTagIcons[group.icon] && (
                              <img 
                                src={hiddenTagIcons[group.icon]} 
                                alt="" 
                                style={{ width: '14px', height: '14px', display: 'inline-block' }}
                              />
                            )}
                            {label}
                          </button>
                        ))}
                      </div>
                    ));
                  })()}
                </div>
              </div>
                </>
              )}
              </div>
              
              {/* Results count */}
              <div style={{ 
                padding: '8px 12px',
                borderTop: '1px solid rgba(255,255,255,0.05)',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                background: 'rgba(0,0,0,0.2)'
              }}>
                <div style={{ fontSize: '13px', color: '#a0937d' }}>
                  {isSearching ? (
                    <><span style={{ color: '#d4af37' }}>🔍</span> 搜索中...</>
                  ) : (
                    <>找到 <span style={{ color: '#ffcc00', fontWeight: 'bold' }}>{searchResults.length}</span> 个结果</>
                  )}
                </div>
                <button 
                  className="bulk-btn" 
                  style={{ fontSize: '11px', padding: '4px 8px' }} 
                  onClick={() => {
                    setSearchQuery({ keyword: "", item_type: "all", size: "", start_tier: "", hero: "", tags: "", hidden_tags: "" });
                    setSelectedTags([]);
                    setSelectedHiddenTags([]);
                  }}
                >
                  重置
                </button>
              </div>
              
              {/* Resize Handle */}
              {!isSearchFilterCollapsed && (
                <div 
                  onMouseDown={(e) => {
                    e.preventDefault();
                    setResizeStartY(e.clientY);
                    setResizeStartHeight(searchFilterHeight);
                    setIsResizingFilter(true);
                  }}
                  style={{
                    position: 'absolute',
                    bottom: '0',
                    left: '0',
                    right: '0',
                    height: '8px',
                    cursor: 'ns-resize',
                    background: 'linear-gradient(to bottom, transparent, rgba(255,205,25,0.1))',
                    borderTop: '1px solid rgba(255,205,25,0.2)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    transition: 'background 0.2s'
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'linear-gradient(to bottom, transparent, rgba(255,205,25,0.2))';
                  }}
                  onMouseLeave={(e) => {
                    if (!isResizingFilter) {
                      e.currentTarget.style.background = 'linear-gradient(to bottom, transparent, rgba(255,205,25,0.1))';
                    }
                  }}
                >
                  <div style={{
                    width: '40px',
                    height: '3px',
                    borderRadius: '2px',
                    background: 'rgba(255,205,25,0.4)'
                  }} />
                </div>
              )}
            </div>
          )}

          <div className="scroll-area" ref={scrollAreaRef} onScroll={handleScroll}>
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

                    {/* 一键识别当前野怪按钮 */}
                    <button
                      className="bulk-btn"
                      disabled={isRecognizing}
                      style={{
                        width: '100%',
                        padding: '12px',
                        marginTop: '12px',
                        background: isRecognizing ? '#333' : 'linear-gradient(135deg, rgba(255, 205, 25, 0.2), rgba(255, 180, 25, 0.15))',
                        border: '2px solid rgba(255, 205, 25, 0.5)',
                        borderRadius: '8px',
                        color: isRecognizing ? '#666' : '#ffcd19',
                        fontSize: '16px',
                        fontWeight: 'bold',
                        cursor: isRecognizing ? 'not-allowed' : 'pointer',
                        transition: 'all 0.2s',
                        textShadow: '0 2px 4px rgba(0, 0, 0, 0.5)',
                        boxShadow: '0 4px 12px rgba(255, 205, 25, 0.2)',
                      }}
                      onClick={async () => {
                        try {
                          const dayNum = selectedDay === "Day 10+" ? 10 : parseInt(selectedDay.split(" ")[1]);
                          showToast(`正在识别 Day ${dayNum} 的野怪...`, 'info');
                          await handleAutoRecognition(dayNum);
                          showToast('一键识别完成', 'success');
                        } catch (err: any) {
                          console.error('[Monster Recognition] Failed:', err);
                          showToast('识别失败: ' + err, 'error');
                        }
                      }}
                      onMouseEnter={(e) => {
                        if (isRecognizing) return;
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 205, 25, 0.3), rgba(255, 180, 25, 0.2))';
                        e.currentTarget.style.borderColor = 'rgba(255, 205, 25, 0.8)';
                        e.currentTarget.style.boxShadow = '0 6px 16px rgba(255, 205, 25, 0.3)';
                      }}
                      onMouseLeave={(e) => {
                        if (isRecognizing) return;
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 205, 25, 0.2), rgba(255, 180, 25, 0.15))';
                        e.currentTarget.style.borderColor = 'rgba(255, 205, 25, 0.5)';
                        e.currentTarget.style.boxShadow = '0 4px 12px rgba(255, 205, 25, 0.2)';
                      }}
                    >
                      {isRecognizing ? '⏳ 正在识别中...' : '🎯 一键识别当前野怪'}
                    </button>

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
                    {(() => {
                        let source: ItemData[] = [];
                        if (activeTab === "card") source = recognizedCards;
                        else if (activeTab === "items") source = getSortedItems([...syncData.hand_items, ...syncData.stash_items]);
                        else if (activeTab === "search") source = searchResults;

                        // Filter empty entries (no Chinese name or no skills)
                        const filtered = source.filter(item => {
                             if (!item.name_cn || item.name_cn.trim() === "") return false;
                             
                             // For Items tab, we want to show everything the user has, even if data is incomplete.
                             // For Search tab, we should filter incomplete data.
                             if (activeTab === "search") {
                                 // Show items that have EITHER skills OR enchantments (not require both)
                                 const hasSkills = item.skills && item.skills.length > 0;
                                 const hasEnchantments = item.enchantments && item.enchantments.length > 0;
                                 if (!hasSkills && !hasEnchantments) {
                                     return false;
                                 }
                             }
                             return true;
                        });

                        const displayList = filtered.slice(0, visibleCount);

                        return displayList.map((item, idx) => {
                      // Use instance_id if available (Hand/Stash), fallback to uuid (Card/YOLO)
                      const uniqueKey = item.instance_id || item.uuid + (activeTab === "card" ? `-${idx}` : "");
                      const expansionKey = item.instance_id || item.uuid;
                      
                      const isExpanded = expandedItems.has(expansionKey);
                      const isRecognized = activeTab === "card";
                      const isTopMatch = idx === 0;
                      const tierClass = item.tier.split(' / ')[0].toLowerCase();
                      const tierNameZh = {
                        'bronze': '青铜+',
                        'silver': '白银+',
                        'gold': '黄金+',
                        'diamond': '钻石+',
                        'legendary': '传说'
                      }[tierClass] || tierClass;

                      let heroZh = item.heroes[0]?.split(' / ')[1] || item.heroes[0] || "通用";
                      if (heroZh === "Common") heroZh = "通用";
                      
                      const sizeClass = item.size?.split(' / ')[0].toLowerCase() || 'medium';

                      return (
                        <div key={uniqueKey} className={`item-card-container ${isExpanded ? 'expanded' : ''} ${isRecognized ? 'identified-glow' : ''}`} onClick={() => toggleExpand(expansionKey)}>
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
                                {(() => {
                                  // 修复：正确处理 heroes 字段可能是字符串或数组的情况
                                  let rawHero = 'Common';
                                  if (Array.isArray(item.heroes) && item.heroes.length > 0) {
                                    rawHero = item.heroes[0];
                                  } else if (typeof item.heroes === 'string' && item.heroes) {
                                    rawHero = item.heroes;
                                  }
                                  
                                  const heroKey = rawHero.split(' / ')[0];
                                  const heroColor = HERO_COLORS[heroKey] || undefined;
                                  const heroAvatarMap: Record<string, string> = {
                                    'Pygmalien': '/images/heroes/pygmalien.webp',
                                    'Jules': '/images/heroes/jules.webp',
                                    'Vanessa': '/images/heroes/vanessa.webp',
                                    'Mak': '/images/heroes/mak.webp',
                                    'Dooley': '/images/heroes/dooley.webp',
                                    'Stelle': '/images/heroes/stelle.webp',
                                    // 兼容缩写
                                    'P': '/images/heroes/pygmalien.webp',
                                    'J': '/images/heroes/jules.webp',
                                    'V': '/images/heroes/vanessa.webp',
                                    'M': '/images/heroes/mak.webp',
                                    'D': '/images/heroes/dooley.webp',
                                    'S': '/images/heroes/stelle.webp'
                                  };

                                  const avatar = heroAvatarMap[heroKey] || (heroKey.length === 1 && heroAvatarMap[heroKey.toUpperCase()]);
                                  
                                  const HeroIcon = () => (
                                      <div className="toggle-btn hero-btn" style={{ 
                                          width: 32, height: 32, minWidth: 32, minHeight: 32, 
                                          padding: 0, marginRight: 0, cursor: 'default',
                                          border: avatar ? 'none' : undefined 
                                      }} title={heroZh}>
                                          {avatar ? 
                                              <img src={avatar} alt={heroZh} style={{width: 28, height: 28, borderRadius: '50%'}} /> : 
                                              <span style={{color: heroColor}}>{heroZh}</span>
                                          }
                                      </div>
                                  );

                                  if (activeTab === 'search') {
                                    return <HeroIcon />;
                                  }
                                  return (
                                    <>
                                      <HeroIcon />
                                      <div 
                                        className={`pin-btn ${pinnedItems.has(expansionKey) ? 'active' : ''}`}
                                        onClick={(e) => togglePin(expansionKey, e)}
                                      >
                                        {pinnedItems.has(expansionKey) ? "📌" : "📍"}
                                      </div>
                                    </>
                                  );
                                })()}
                              </div>
                              <div className="expand-chevron">{isExpanded ? '▴' : '▾'}</div>
                            </div>
                          </div>

                        {isExpanded && (
                          <div className={`item-details-v2 ${pinnedItems.has(expansionKey) ? 'progression-active' : ''}`}>
                            {(() => {
                                try {
                                    // 强制从原始数据读取，防止类型系统干扰
                                    const cdTiersRaw = (item as any).cooldown_tiers;
                                    const availTiersRaw = (item as any).available_tiers;
                                    
                                    const hasProgression = cdTiersRaw && typeof cdTiersRaw === 'string' && cdTiersRaw.includes('/');
                                    
                                    if (hasProgression) {
                                      const cdVals = (cdTiersRaw as string).split('/').map((v: string) => {
                                        const ms = parseFloat(v);
                                        if (isNaN(ms)) return "0.0";
                                        return (ms > 100 ? ms / 1000 : ms).toFixed(1);
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
                                      <div className="cd-value">{(item.cooldown > 100 ? item.cooldown / 1000 : item.cooldown).toFixed(1)}</div>
                                      <div className="cd-unit">秒</div>
                                    </div>
                                  </div>
                                );
                            })()}
                            <div className="details-right">
                              {item.skills.map((s, idx) => (
                                <div key={idx} className="skill-item">
                                  {renderTextLocal(s)}
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
                                    <span className="enchant-effect">{renderEnchantTextLocal(effect)}</span>
                                  </div>
                                );
                              }
                              return (
                                <div key={idx} className="enchant-item">
                                  {renderTextLocal(enc)}
                                </div>
                              );
                            })}
                          </div>
                        )}
                        {item.description && isExpanded && (
                          <div className="item-description-row">
                            <div className="description-text">
                              {renderTextLocal(item.description)}
                            </div>
                          </div>
                        )}
                      </div>
                    );
                  });
                })()}
                  {activeTab === "card" && recognizedCards.length === 0 && !isRecognizingCard && (
                    <div className="empty-tip">按下 Alt 键 识别鼠标指向的卡牌</div>
                  )}
                  {activeTab === "card" && isRecognizingCard && (
                    <div className="empty-tip">🔍 正在识别中...</div>
                  )}
                  {activeTab === "items" && (syncData.hand_items.length + syncData.stash_items.length) === 0 && (
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
      
      {/* Toast 提示容器 */}
      {toasts.length > 0 && (
        <div className="toast-container">
          {toasts.map(toast => (
            <div key={toast.id} className={`toast toast-${toast.type}`}>
              <div className="toast-icon">
                {toast.type === 'success' && '✓'}
                {toast.type === 'error' && '✕'}
                {toast.type === 'warning' && '⚠'}
                {toast.type === 'info' && 'ℹ'}
              </div>
              <div className="toast-message">{toast.message}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}