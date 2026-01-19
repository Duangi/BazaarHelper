import * as React from "react";
import { useEffect, useState, useRef, Fragment } from "react";
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { resolveResource } from "@tauri-apps/api/path";
import "./App.css";

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

export default function OverlayApp() {
    const [yoloResult, setYoloResult] = useState<{type: 'item' | 'monster', data: any} | null>(null);
    const [identifying, setIdentifying] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);
    const yoloResultRef = useRef(yoloResult);
    const imgCache = useRef<Map<string, string>>(new Map());

    // Drag and Resize states
    const [pos, setPos] = useState({ x: -1, y: -1 });
    const [isDragging, setIsDragging] = useState(false);
    const dragOffset = useRef({ x: 0, y: 0 });
    const [size, setSize] = useState({ w: 420, h: -1 });
    const [isResizing, setIsResizing] = useState(false);

    useEffect(() => {
        const move = (e: MouseEvent) => {
            if (isDragging) {
                setPos({ x: e.clientX - dragOffset.current.x, y: e.clientY - dragOffset.current.y });
            } else if (isResizing) {
                const rect = containerRef.current!.getBoundingClientRect();
                setSize({
                    w: Math.max(380, e.clientX - rect.left),
                    h: Math.max(200, e.clientY - rect.top)
                });
            }
        };
        const stop = () => {
            if (isDragging || isResizing) {
                setIsDragging(false);
                setIsResizing(false);
                // After drag/resize stop, if mouse is not over container, restore ignore
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
    }, [isDragging, isResizing]);

    const winMousePos = useRef({ x: 0, y: 0 });
    useEffect(() => {
        const track = (e: MouseEvent) => { winMousePos.current = { x: e.clientX, y: e.clientY }; };
        window.addEventListener('mousemove', track);
        return () => window.removeEventListener('mousemove', track);
    }, []);

    useEffect(() => {
        invoke("set_overlay_ignore_cursor", { ignore: true }).catch(console.error);
    }, []);

    useEffect(() => { yoloResultRef.current = yoloResult; }, [yoloResult]);

    const getImg = async (path: string | null | undefined) => {
        if (!path) return "";
        if (imgCache.current.has(path)) return imgCache.current.get(path)!;
        try {
            const fullPath = await resolveResource(`resources/${path}`);
            const assetUrl = convertFileSrc(fullPath);
            imgCache.current.set(path, assetUrl);
            return assetUrl;
        } catch { return ""; }
    };

    const processMonsterImages = async (m: MonsterData) => {
        let filename = m.image ? m.image.split('/').pop() || `${m.name_zh}.webp` : `${m.name_zh}.webp`;
        let displayImg = await getImg(`images_monster_char/${filename}`);
        if (!displayImg && m.name_zh.includes(' ')) {
            const spacePos = m.name_zh.lastIndexOf(' ');
            const baseName = m.name_zh.substring(spacePos + 1);
            const fallbackFilename = `${baseName}.webp`;
            const fallbackImg = await getImg(`images_monster_char/${fallbackFilename}`);
            if (fallbackImg) { displayImg = fallbackImg; filename = fallbackFilename; }
        }
        let bgFilename = filename;
        if (m.name_zh === '绿洲守护神') bgFilename = '绿洲守护神_Day9.webp';
        const displayImgBg = await getImg(`images_monster_bg/${bgFilename}`);

        return {
            ...m,
            displayImg,
            displayImgBg,
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

    useEffect(() => {
        const unlistenPromise = listen<{x: number, y: number}>("global-right-click", async (event) => {
            const coords = event.payload;
            if (yoloResultRef.current) {
                setYoloResult(null);
                setPos({ x: -1, y: -1 });
                invoke("set_overlay_ignore_cursor", { ignore: true }).catch(() => {});
            } else {
                try {
                    setIdentifying(true);
                    const res = await invoke<{type: 'item' | 'monster', data: any} | null>("handle_overlay_right_click", { 
                        x: Math.round(coords.x), 
                        y: Math.round(coords.y) 
                    });
                    
                    if (res) {
                        if (res.type === 'item') {
                            const item = res.data as ItemData;
                            item.displayImg = await getImg(`images/${item.uuid || item.name}.webp`);
                            setYoloResult({ type: 'item', data: item });
                        } else if (res.type === 'monster') {
                            const monster = await processMonsterImages(res.data as MonsterData);
                            setYoloResult({ type: 'monster', data: monster });
                        }
                    }
                } catch (err) {
                    console.error("Right-click handle error:", err);
                } finally {
                    setIdentifying(false);
                }
            }
        });
        return () => { unlistenPromise.then(u => u()); };
    }, []);

    // Reporting bounds for "Auto-Sensing Door"
    useEffect(() => {
        const timer = setInterval(() => {
            if (containerRef.current && yoloResult) {
                const rect = containerRef.current.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) {
                    invoke("update_overlay_bounds", { 
                        bounds: [{ x: rect.left, y: rect.top, w: rect.width, h: rect.height }] 
                    });
                } else {
                    invoke("update_overlay_bounds", { bounds: [] });
                }
            } else if (!yoloResult) {
                 invoke("update_overlay_bounds", { bounds: [] });
            }
        }, 100);
        return () => clearInterval(timer);
    }, [yoloResult]);

    const handleMouseEnter = () => {
        invoke("set_overlay_ignore_cursor", { ignore: false }).catch(console.error);
    };

    const handleMouseLeave = () => {
        if (!isDragging && !isResizing) {
            invoke("set_overlay_ignore_cursor", { ignore: true }).catch(console.error);
            invoke("restore_game_focus").catch(console.error);
        }
    };

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

    const formatDescription = (text: string) => {
        if (!text) return null;
        const parts = text.split(/(\[Locked\]|Quest:)/g);
        return parts.map((part, i) => {
            if (part === "[Locked]") return <span key={i} className="icon-locked" title="Locked" style={{ color: '#ff4d4d' }}>🔒</span>;
            if (part === "Quest:") return <span key={i} className="icon-quest" title="Quest" style={{ color: '#ffd700' }}>📜</span>;
            return part;
        });
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
            const cdSecs = item.cooldown_tiers.split('/').map(v => (parseFloat(v)/1000).toFixed(1)).join(' / ');
            stats.push({ label: 'CD', value: cdSecs + 's' });
        }
        check(item.ammo_tiers, '弹药');
        check(item.burn_tiers, '灼烧');
        check(item.poison_tiers, '中毒');
        check(item.multicast_tiers, '多重');
        check(item.regen_tiers, '回复');

        if (stats.length === 0) return null;

        return (
            <div style={{ marginTop: '10px', background: 'rgba(0,0,0,0.3)', padding: '8px', borderRadius: '4px' }}>
                <div style={{ fontSize: '11px', color: 'var(--c-golden)', marginBottom: '6px', borderBottom: '1px solid rgba(255,205,25,0.2)' }}>属性成长 (Tier Progression)</div>
                {stats.map(s => (
                    <div key={s.label} style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '2px' }}>
                        <span style={{ color: '#888' }}>{s.label}</span>
                        <span style={{ color: '#ddd' }}>{s.value}</span>
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
                pointerEvents: 'none',
                overflow: 'hidden'
            }}
        >
            {identifying && (
                <div style={{
                    position: 'absolute',
                    top: '20px',
                    left: '50%',
                    transform: 'translateX(-50%)',
                    background: 'rgba(0,0,0,0.7)',
                    color: '#fff',
                    padding: '8px 20px',
                    borderRadius: '20px',
                    border: '1px solid var(--c-golden)',
                    fontSize: '14px',
                    zIndex: 1000,
                    boxShadow: '0 0 15px rgba(255, 205, 25, 0.5)'
                }}>
                    🔍 正在识别卡牌/怪物...
                </div>
            )}

            {yoloResult && (
                <div 
                    ref={containerRef}
                    onMouseEnter={handleMouseEnter}
                    onMouseLeave={handleMouseLeave}
                    style={{
                        position: 'absolute',
                        left: pos.x !== -1 ? `${pos.x}px` : undefined,
                        top: pos.y !== -1 ? `${pos.y}px` : '10%',
                        right: pos.x === -1 ? '2%' : undefined,
                        width: `${size.w}px`,
                        height: size.h !== -1 ? `${size.h}px` : 'auto',
                        pointerEvents: 'auto',
                        zIndex: 999,
                        overflow: 'hidden',
                        minWidth: '380px',
                        maxHeight: '85vh',
                        background: 'rgba(20,15,10,0.98)',
                        border: '1px solid var(--c-golden)',
                        borderRadius: '12px',
                        boxShadow: '0 15px 45px rgba(0,0,0,0.9), 0 0 20px rgba(255,205,25,0.1)',
                        display: 'flex',
                        flexDirection: 'column'
                    }}
                >
                    {/* Drag Handle */}
                    <div 
                        onMouseDown={(e) => {
                            setIsDragging(true);
                            const rect = containerRef.current!.getBoundingClientRect();
                            dragOffset.current = { x: e.clientX - rect.left, y: e.clientY - rect.top };
                        }}
                        style={{
                            height: '32px',
                            background: 'linear-gradient(90deg, rgba(255, 205, 25, 0.1), rgba(255, 205, 25, 0.2), rgba(255, 205, 25, 0.1))',
                            cursor: 'move',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            fontSize: '10px',
                            color: 'rgba(255,205,25,0.8)',
                            letterSpacing: '5px',
                            flexShrink: 0,
                            borderBottom: '1px solid rgba(255,205,25,0.2)',
                            userSelect: 'none',
                            fontWeight: 'bold',
                            textShadow: '0 0 5px rgba(0,0,0,0.5)'
                        }}
                    >
                        ::: 手动识别结果 :::
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
                                                    <div className={`image-box size-${sizeClass}`}>
                                                        <img src={item.displayImg} alt={item.name} />
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
                                                            {formatDescription(s)}
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
                                                                    <span className="enchant-effect">{formatDescription(effect)}</span>
                                                                </div>
                                                            );
                                                        }
                                                        return <div key={idx} className="enchant-item">{formatDescription(enc)}</div>;
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
                        ) : (
                            <div className="monster-card-v2 expanded" style={{ border: 'none', boxShadow: 'none', background: 'transparent', margin: 0, padding: 0 }}>
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
                                                        <div className="section-title">物品 (Items)</div>
                                                        {m.items.map((it) => renderTierInfo(it))}
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
                        左键拖动顶部识别结果 | 右键空白处关闭 | 右下角调整大小
                    </div>
                </div>
            )}
        </div>
    );
}

// REST OF FILE OMITTED (to satisfy the tool's limit)

