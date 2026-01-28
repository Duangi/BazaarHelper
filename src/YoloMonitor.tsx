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
            data-tauri-drag-region
            style={{
                background: 'rgba(0, 0, 0, 0.85)',
                border: isPolling ? '2px solid #00ffcc' : '2px solid #444',
                borderRadius: '12px',
                color: '#00ffcc',
                fontFamily: 'Consolas, monospace',
                fontSize: '11px',
                backdropFilter: 'blur(10px)',
                boxShadow: '0 4px 20px rgba(0, 255, 204, 0.2)',
                transition: 'border-color 0.2s ease',
                pointerEvents: 'auto',
                overflow: 'auto',
                cursor: 'move',
                userSelect: 'none',
                width: '100%',
                height: '100%',
                boxSizing: 'border-box',
                display: 'flex',
                flexDirection: 'column'
            }}
            onContextMenu={(e) => e.preventDefault()}
        >
                {/* 标题栏 */}
                <div 
                    data-tauri-drag-region
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        padding: '12px 12px 10px 12px',
                        marginBottom: '10px',
                        borderBottom: '1px solid rgba(0, 255, 204, 0.3)'
                    }}>
                    <span 
                        data-tauri-drag-region
                        style={{
                            fontWeight: 'bold',
                            fontSize: '12px',
                            textTransform: 'uppercase',
                            letterSpacing: '1px',
                            pointerEvents: 'none'
                        }}>
                        🎯 YOLO Monitor
                    </span>
                    <div 
                        data-tauri-drag-region
                        style={{
                            width: '8px',
                            height: '8px',
                            borderRadius: '50%',
                            background: isPolling ? '#00ff00' : '#555',
                            boxShadow: isPolling ? '0 0 10px #00ff00' : 'none',
                            transition: 'all 0.2s ease',
                            pointerEvents: 'none'
                        }} />
                </div>

                {yoloStats ? (
                    <div 
                        data-tauri-drag-region
                        style={{ display: 'flex', flexDirection: 'column', gap: '6px', padding: '0 12px 12px 12px' }}>
                        <div data-tauri-drag-region style={{ display: 'flex', justifyContent: 'space-between' }}>
                            <span data-tauri-drag-region style={{ color: '#888', pointerEvents: 'none' }}>Total:</span>
                            <span data-tauri-drag-region style={{ fontWeight: 'bold', color: '#fff', pointerEvents: 'none' }}>{yoloStats.total}</span>
                        </div>
                        <div data-tauri-drag-region style={{ display: 'flex', justifyContent: 'space-between' }}>
                            <span data-tauri-drag-region style={{ color: '#888', pointerEvents: 'none' }}>Items:</span>
                            <span data-tauri-drag-region style={{ color: '#4ecdc4', pointerEvents: 'none' }}>{yoloStats.items}</span>
                        </div>
                        <div data-tauri-drag-region style={{ display: 'flex', justifyContent: 'space-between' }}>
                            <span data-tauri-drag-region style={{ color: '#888', pointerEvents: 'none' }}>Skills:</span>
                            <span data-tauri-drag-region style={{ color: '#95e1d3', pointerEvents: 'none' }}>{yoloStats.skills}</span>
                        </div>
                        <div data-tauri-drag-region style={{ display: 'flex', justifyContent: 'space-between' }}>
                            <span data-tauri-drag-region style={{ color: '#888', pointerEvents: 'none' }}>Events:</span>
                            <span data-tauri-drag-region style={{ color: '#f38181', pointerEvents: 'none' }}>{yoloStats.events}</span>
                        </div>
                        <div data-tauri-drag-region style={{ display: 'flex', justifyContent: 'space-between' }}>
                            <span data-tauri-drag-region style={{ color: '#888', pointerEvents: 'none' }}>Monsters:</span>
                            <span data-tauri-drag-region style={{ color: '#aa96da', pointerEvents: 'none' }}>{yoloStats.monsters}</span>
                        </div>
                    </div>
                ) : (
                    <div 
                        data-tauri-drag-region
                        style={{ color: '#666', fontSize: '10px', textAlign: 'center', padding: '0 12px 12px 12px' }}>
                        等待扫描...
                    </div>
                )}

                {realtimeList.length > 0 && (
                    <div style={{
                        marginTop: '12px',
                        paddingTop: '8px',
                        borderTop: '1px solid rgba(0, 255, 204, 0.2)',
                        maxHeight: '120px',
                        overflowY: 'auto'
                    }}>
                        <div style={{
                            fontSize: '10px',
                            color: '#888',
                            marginBottom: '4px',
                            textTransform: 'uppercase'
                        }}>
                            Recent:
                        </div>
                        {realtimeList.slice(0, 5).map((name, i) => (
                            <div key={i} style={{
                                fontSize: '10px',
                                color: '#00ffcc',
                                padding: '2px 4px',
                                background: 'rgba(0, 255, 204, 0.1)',
                                borderRadius: '4px',
                                marginBottom: '2px',
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap'
                            }}>
                                {name}
                            </div>
                        ))}
                    </div>
                )}
        </div>
    );
}
