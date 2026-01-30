import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

interface YoloStats {
    total: number;
    items: number;
    events: number;
    monsters: number;
    skills: number;
}

export default function YoloMonitor() {
    const [yoloStats, setYoloStats] = useState<YoloStats | null>(null);
    const [isPolling, setIsPolling] = useState(false);
    const [realtimeList, setRealtimeList] = useState<string[]>([]);

    useEffect(() => {
        // 监听 YOLO 统计更新
        const statsUnlisten = listen("yolo-stats-updated", (event: any) => {
            setYoloStats(prev => {
                if (JSON.stringify(prev) === JSON.stringify(event.payload)) return prev;
                return event.payload;
            });
        });

        // 监听实时识别列表
        const listUnlisten = listen("realtime-yolo-list", (event: any) => {
            const payload = event.payload as { list: string[] };
            setRealtimeList(payload.list || []);
            setIsPolling(true);
            setTimeout(() => setIsPolling(false), 300);
        });

        return () => {
            statsUnlisten.then(fn => fn());
            listUnlisten.then(fn => fn());
        };
    }, []);

    return (
        <div 
            style={{
                background: 'rgba(15, 15, 20, 0.92)',
                border: isPolling ? '1px solid #00ffcc' : '1px solid rgba(100, 100, 100, 0.5)',
                borderRadius: '8px',
                color: '#e0e0e0',
                fontFamily: 'Consolas, monospace',
                fontSize: '12px',
                backdropFilter: 'blur(12px)',
                boxShadow: '0 8px 32px rgba(0, 0, 0, 0.6)',
                transition: 'border-color 0.2s ease',
                pointerEvents: 'auto',
                width: '100%',
                height: '100%',
                boxSizing: 'border-box',
                display: 'flex',
                flexDirection: 'column',
                overflow: 'hidden'
            }}
            onContextMenu={(e) => e.preventDefault()}
        >
                {/* 标题栏 - 仅此处可拖动 */}
                <div 
                    data-tauri-drag-region
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        padding: '10px 14px',
                        background: 'linear-gradient(to right, rgba(255,255,255,0.05), transparent)',
                        borderBottom: '1px solid rgba(255, 255, 255, 0.08)',
                        cursor: 'move',
                        flexShrink: 0
                    }}>
                    <div data-tauri-drag-region style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span data-tauri-drag-region style={{ fontSize: '14px' }}>🎯</span>
                        <span 
                            data-tauri-drag-region
                            style={{
                                fontWeight: '600',
                                fontSize: '13px',
                                color: '#00ffcc',
                                letterSpacing: '0.5px',
                                textShadow: '0 0 10px rgba(0, 255, 204, 0.3)',
                                pointerEvents: 'none'
                            }}>
                            YOLO MONITOR
                        </span>
                    </div>
                    <div 
                        data-tauri-drag-region
                        style={{
                            width: '8px',
                            height: '8px',
                            borderRadius: '50%',
                            background: isPolling ? '#00ff00' : '#444',
                            boxShadow: isPolling ? '0 0 8px #00ff00' : 'none',
                            transition: 'all 0.2s ease',
                            pointerEvents: 'none'
                        }} />
                </div>

                {/* 内容区域 - 可用于调整大小 (因为没有 data-tauri-drag-region) */}
                <div style={{ 
                    flex: 1, 
                    overflow: 'auto', 
                    padding: '14px',
                    display: 'flex', 
                    flexDirection: 'column', 
                    gap: '12px',
                    cursor: 'default'
                }}>
                    {yoloStats ? (
                        <div style={{ 
                            display: 'grid', 
                            gridTemplateColumns: '1fr 1fr', 
                            gap: '10px' 
                        }}>
                             <StatItem label="TOTAL" value={yoloStats.total} color="#fff" />
                             <StatItem label="ITEMS" value={yoloStats.items} color="#4ecdc4" />
                             <StatItem label="SKILLS" value={yoloStats.skills} color="#95e1d3" />
                             <StatItem label="EVENTS" value={yoloStats.events} color="#ff6b6b" />
                             <StatItem label="MONSTERS" value={yoloStats.monsters} color="#aa96da" />
                        </div>
                    ) : (
                        <div style={{ color: '#666', fontSize: '12px', textAlign: 'center', padding: '20px 0', fontStyle: 'italic' }}>
                            Waiting for scan...
                        </div>
                    )}

                    {realtimeList.length > 0 && (
                        <div style={{
                            marginTop: '4px',
                            paddingTop: '10px',
                            borderTop: '1px solid rgba(255, 255, 255, 0.08)',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: '6px'
                        }}>
                            <div style={{ fontSize: '10px', color: '#666', fontWeight: 'bold', letterSpacing: '1px' }}>LATEST DETECTIONS</div>
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                                {realtimeList.slice(0, 6).map((name, i) => (
                                    <div key={i} style={{
                                        fontSize: '11px',
                                        color: '#ccc',
                                        whiteSpace: 'nowrap',
                                        overflow: 'hidden',
                                        textOverflow: 'ellipsis',
                                        paddingLeft: '6px',
                                        borderLeft: '2px solid rgba(0, 255, 204, 0.3)'
                                    }}>
                                        {name}
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}
                </div>
        </div>
    );
}

const StatItem = ({ label, value, color }: { label: string, value: number, color: string }) => (
    <div style={{ 
        display: 'flex', 
        justifyContent: 'space-between', 
        alignItems: 'center', 
        background: 'rgba(255,255,255,0.05)', 
        padding: '6px 10px', 
        borderRadius: '6px',
        border: '1px solid rgba(255,255,255,0.05)'
    }}>
        <span style={{ color: '#aaa', fontSize: '11px' }}>{label}</span>
        <span style={{ color: color, fontWeight: 'bold', fontSize: '14px', fontFamily: 'Arial, sans-serif' }}>{value}</span>
    </div>
);
