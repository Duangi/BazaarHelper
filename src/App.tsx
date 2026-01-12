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

interface MonsterSubItem { 
  name: string; 
  description: string; 
  image: string; 
  displayImg?: string; 
}

interface MonsterData { 
  name: string; 
  name_zh: string; 
  image: string; 
  skills: MonsterSubItem[]; 
  items: MonsterSubItem[]; 
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
  const [activeTab, setActiveTab] = useState<TabType>("hand");
  const [syncData, setSyncData] = useState<Record<TabType, any[]>>({ hand: [], stash: [], monster: [] });
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [manualMonsters, setManualMonsters] = useState<MonsterData[]>([]);
  const [pinnedItems, setPinnedItems] = useState<Map<string, number>>(new Map()); // 存储置顶物品ID和置顶时间戳
  const [pinnedCounter, setPinnedCounter] = useState(0); // 置顶计数器，用于确定置顶顺序
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

  // 3. 怪物搜索逻辑
  const handleSearch = async (val: string) => {
    setSearchQuery(val);
    if (!val) { setManualMonsters([]); return; }
    try {
      const res: MonsterData[] = await invoke("search_monsters", { query: val });
      const processed = await Promise.all(res.map(async m => ({
        ...m,
        displayImg: await getImg(m.image),
        skills: await Promise.all(m.skills.map(async s => ({ ...s, displayImg: await getImg(s.image) }))),
        items: await Promise.all(m.items.map(async i => ({ ...i, displayImg: await getImg(i.image) })))
      })));
      setManualMonsters(processed);
    } catch (e) { console.error(e); }
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
    <div className={`overlay ${isCollapsed ? 'collapsed' : 'expanded'}`}>
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
            {(["hand", "stash", "monster"] as TabType[]).map(t => (
              <div key={t} className={`nav-item ${activeTab === t ? 'active' : ''}`} onClick={() => setActiveTab(t)}>
                {t === 'hand' ? '手牌' : t === 'stash' ? '仓库' : '野怪查询'}
              </div>
            ))}
          </nav>

          <div className="scroll-area">
            <div className="items" ref={wrapRef}>
              {activeTab === "monster" ? (
                <>
                  <div className="search-container">
                    <input className="search-input" placeholder="🔍 输入怪物名称查询..." value={searchQuery} onChange={e => handleSearch(e.target.value)} />
                  </div>
                  {manualMonsters.map((m, i) => (
                    <div key={i} className="monster-card">
                      <div className="monster-header"><img src={m.displayImg} className="monster-avatar" alt="" /><div>{m.name_zh}</div></div>
                      <div className="monster-grid">
                        {m.skills.map((s, idx) => <div key={idx} className="mini-cell skill-cell"><img src={s.displayImg} className="mini-img" alt="" /><div><strong>{s.name}</strong><p>{s.description}</p></div></div>)}
                        {m.items.map((it, idx) => <div key={idx} className="mini-cell item-border"><img src={it.displayImg} className="mini-img" alt="" /><div><strong>{it.name}</strong><p>{it.description}</p></div></div>)}
                      </div>
                    </div>
                  ))}
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