import { useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalPosition, LogicalSize, currentMonitor } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import "./App.css";

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

  // 4. 窗口定位与尺寸控制 (贴边、去阴影、Y=0)
  useEffect(() => {
    const syncLayout = async () => {
      const monitor = await currentMonitor();
      if (!monitor) return;
      const appWindow = getCurrentWindow();
      const scale = monitor.scaleFactor;
      
      if (appWindow.setShadow) await appWindow.setShadow(false);

      const currentWidth = expandedWidth;
      const currentHeight = isCollapsed ? 45 : (monitor.size.height / scale) - 200;
      const targetX = (monitor.size.width / scale) - currentWidth;

      try {
        await appWindow.setSize(new LogicalSize(currentWidth, currentHeight));
        await appWindow.setPosition(new LogicalPosition(targetX, 0)); // 严格贴顶
        await appWindow.setAlwaysOnTop(true);
      } catch (e) { console.error(e); }
    };
    syncLayout();
  }, [expandedWidth, isCollapsed, syncData, activeTab, manualMonsters]);

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

  return (
    <div className={`overlay ${isCollapsed ? 'collapsed' : 'expanded'}`}>
      {!isCollapsed && <div className="resize-handle" onMouseDown={handleResize} />}
      
      <div className="collapse-btn" onClick={() => setIsCollapsed(!isCollapsed)}>
        {isCollapsed ? "展开插件 ▾" : "收起插件 ▴"}
      </div>

      {!isCollapsed && (
        <>
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