import { useEffect, useState, useRef, Fragment } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import "./App.css";

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

const HERO_COLORS: Record<string, string> = {
  "Vanessa": '#FF6B6B',
  "Pygmalien": '#5BA3FF',
  "Jules": '#D77EFF',
  "Mak": '#D4FF85',
  "Dooley": '#FFC048',
  "Stelle": '#FFE74C',
  "Common": '#E0E0E0'
};

// 类型定义
interface SkillText {
    en: string;
    cn: string;
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
    enchantments: string[];
    description: string;
    image: string;
    displayImg?: string;
}

type DetailData = ItemData;
type DetailType = 'item';

export default function DetailPopup() {
    const [data, setData] = useState<{ type: DetailType; data: DetailData } | null>(null);
    const [scale, setScale] = useState(0);
    const [isVisible, setIsVisible] = useState(false);
    const [allTags] = useState<string[]>([]);
    const imgCache = useRef<Map<string, string>>(new Map());
    const containerRef = useRef<HTMLDivElement>(null);
    const MAX_CACHE_SIZE = 200;

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

    // 渲染附魔文本（照抄 App.tsx）
    const renderEnchantText = (content: string) => {
        if (!content) return null;
        
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
                  
                  const val = parseFloat(n);
                  const displayVal = (!isNaN(val) && val > 100) ? (val / 1000).toFixed(1) : n;
                  
                  return (
                    <span key={idx}>
                      <span style={{ color: TIER_COLORS[colorIdx] || '#fff', fontWeight: 'bold' }}>{displayVal}</span>
                      {idx < nums.length - 1 && <span style={{ color: '#fff' }}>/</span>}
                    </span>
                  );
                })}
              </span>
            );
          }
    
          let processedPart = part;
          processedPart = processedPart.replace(/\b(\d{3,})\b/g, (match) => {
              const val = parseInt(match, 10);
              return val > 100 ? (val / 1000).toFixed(1) : match;
          });
    
          const keywords = Object.keys(KEYWORD_COLORS);
          const allMatches = [...new Set([...keywords, ...allTags])].filter(k => k.length > 0);
          
          if (allMatches.length === 0) return processedPart;
          
          const regex = new RegExp(`(${allMatches.join('|')})`, 'g');
          const subParts = processedPart.split(regex);
          
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
                    const { type, data: itemData } = event.payload;
                    
                    // 和 OverlayApp.tsx 一样，使用 uuid 或 name 来构建图片路径
                    console.log("[DetailPopup] Item uuid:", itemData.uuid, "name:", itemData.name);
                    let imageUrl = "";
                    if (itemData.uuid || itemData.name) {
                        const imagePath = `images/${itemData.uuid || itemData.name}.webp`;
                        imageUrl = await getImg(imagePath);
                    }
                    
                    const processedData = {
                        ...itemData,
                        displayImg: imageUrl || itemData.displayImg
                    };
                    
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
                setScale(0);
                
                setTimeout(async () => {
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

    const item = data.data;
    
    // 计算等级相关
    const tierMap: Record<string, string> = { 'Bronze': '青铜', 'Silver': '白银', 'Gold': '黄金', 'Diamond': '钻石', 'Legendary': '传奇' };
    const tierNameZh = tierMap[item.tier] || item.tier;
    const tierClass = (item.tier || '').toLowerCase();
    const sizeClass = (item.size || 'medium').toLowerCase();
    
    // 英雄信息
    const heroMap: Record<string, string> = { Pygmalien: '皮格', Jules: '朱尔斯', Vanessa: '凡妮莎', Mak: '麦克', Dooley: '杜利', Stelle: '斯黛拉', Common: '通用' };
    const rawHero = item.heroes && item.heroes[0] ? item.heroes[0] : 'Common';
    const heroKey = rawHero.split(' / ')[0];
    const heroZh = heroMap[heroKey] || heroKey;
    const heroColor = HERO_COLORS[heroKey] || undefined;
    const heroAvatarMap: Record<string, string> = {
        'Pygmalien': '/images/heroes/pygmalien.webp',
        'Jules': '/images/heroes/jules.webp',
        'Vanessa': '/images/heroes/vanessa.webp',
        'Mak': '/images/heroes/mak.webp',
        'Dooley': '/images/heroes/dooley.webp',
        'Stelle': '/images/heroes/stelle.webp'
    };
    const avatar = heroAvatarMap[heroKey];

    return (
        <div 
            ref={containerRef}
            onClick={(e) => {
                // 点击背景关闭
                if (e.target === e.currentTarget || (e.target as HTMLElement).classList.contains('detail-popup-container')) {
                    invoke("hide_detail_popup");
                }
            }}
            onContextMenu={(e) => {
                e.preventDefault();
                // 右键点击背景也能关闭
                if (e.target === e.currentTarget || (e.target as HTMLElement).classList.contains('detail-popup-container')) {
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
                transition: 'opacity 0.2s ease-out'
            }}
        >
            {/* 拖动条 */}
            <div
                data-tauri-drag-region
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
            
            {/* 内容区域 */}
            <div
                className="custom-scrollbar"
                style={{
                    flex: 1,
                    overflow: 'auto',
                    padding: '10px'
                    // 移除内部的 opacity，改为由外层统一控制
                }}
            >
                <div className="item-card-container expanded">
                <div className={`item-card tier-${tierClass}`}>
                    <div className="card-left">
                        <div className={`image-box size-${sizeClass}`}>
                            <img 
                                src={item.displayImg || ''} 
                                alt={item.name}
                                onError={(e) => {
                                    console.error("[DetailPopup] Image load failed. displayImg:", item.displayImg, "original image:", item.image);
                                    e.currentTarget.style.display = 'none';
                                }}
                                onLoad={() => {
                                    console.log("[DetailPopup] Image loaded successfully. displayImg:", item.displayImg);
                                }}
                            />
                        </div>
                    </div>

                    <div className="card-center">
                        <div className="name-line">
                            <span className="name-cn">{item.name_cn}</span>
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
                            {avatar ? 
                                <img src={avatar} alt={heroZh} className="hero-avatar-badge" title={heroZh} /> : 
                                <span className="hero-badge" style={{marginRight: 0, color: heroColor}}>{heroZh}</span>
                            }
                        </div>
                        <div className="expand-chevron">▴</div>
                    </div>
                </div>

                <div className="item-details-v2">
                    {(() => {
                        try {
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
                                {renderText(s)}
                            </div>
                        ))}
                    </div>
                </div>

                {item.enchantments && item.enchantments.length > 0 && (
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
                                        <span className="enchant-effect">{renderEnchantText(effect)}</span>
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

                {item.description && (
                    <div className="item-description-row">
                        <div className="description-text">
                            {renderText(item.description)}
                        </div>
                    </div>
                )}
                </div>
            </div>
        </div>
    );
}
