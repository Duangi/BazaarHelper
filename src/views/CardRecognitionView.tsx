import React, { useEffect, useState } from 'react';
import { ItemData } from '../types';
import { invoke } from '@tauri-apps/api/core';

interface CardRecognitionViewProps {
  recognizedCards: ItemData[];
  isRecognizing: boolean;
  expandedItems: Set<string>;
  onToggleExpand: (uuid: string) => void;
  onRecognize: () => void;
  renderItemCard: (
    item: ItemData,
    isPinned: boolean,
    onPin: (e: React.MouseEvent) => void,
    imageSrcOverride?: string,
  ) => React.ReactNode;
}

export const CardRecognitionView: React.FC<CardRecognitionViewProps> = ({
  recognizedCards,
  isRecognizing,
  expandedItems,
  onToggleExpand,
  onRecognize,
  renderItemCard
}) => {
  const [hotkeyLabel, setHotkeyLabel] = useState("Loading...");

  useEffect(() => {
      invoke<number | null>('get_card_detection_hotkey').then(code => {
          if (code) {
             const getLabel = (c: number) => {
                if (c >= 112 && c <= 123) return `F${c - 111}`;
                if (c >= 65 && c <= 90) return String.fromCharCode(c);
                if (c >= 48 && c <= 57) return String.fromCharCode(c);
                switch(c) {
                    case 18: return 'Alt';
                    case 17: return 'Ctrl';
                    case 16: return 'Shift';
                    case 9: return 'Tab';
                    case 20: return 'CapsLock';
                    case 32: return 'Space';
                    case 27: return 'Esc';
                    case 8: return 'Backspace';
                    case 13: return 'Enter';
                    case 192: return '~';
                    case 81: return 'Q';
                    case 2: return 'Right Click'; // Add mouse buttons just in case
                    case 1: return 'Left Click';
                    case 4: return 'Middle Click';
                }
                return `Key ${c}`;
             };
             setHotkeyLabel(getLabel(code));
          } else {
             setHotkeyLabel("未设置");
          }
      });
  }, []);

  return (
    <div className="card-recognition-view">
      <div className="recognition-header" style={{
          padding: '24px 16px',
          textAlign: 'center',
          color: '#8b949e',
          background: 'rgba(0,0,0,0.2)',
          borderBottom: '1px solid rgba(255,255,255,0.05)',
          marginBottom: '16px'
      }}>
        <div style={{ fontSize: '15px', marginBottom: '12px', color: '#e6edf3', fontWeight: 500 }}>
            🃏 卡牌识别
        </div>
        <div style={{ fontSize: '13px', lineHeight: '1.6', textAlign: 'left', display: 'inline-block' }}>
            1. 在游戏中<strong>右键点击</strong>卡牌查看大图<br/>
            2. 将鼠标悬停在卡牌中心位置<br/>
            3. 按下 <span style={{ 
                background: 'rgba(56, 139, 253, 0.15)', 
                border: '1px solid rgba(56, 139, 253, 0.4)',
                padding: '2px 8px', 
                borderRadius: '6px',
                color: '#58a6ff',
                fontFamily: 'monospace',
                fontWeight: 'bold'
            }}>{hotkeyLabel}</span> 进行识别
        </div>
        <div style={{ marginTop: '14px' }}>
          <button
            className="bulk-btn"
            onClick={onRecognize}
            disabled={isRecognizing}
            style={{ minWidth: '140px' }}
          >
            {isRecognizing ? '识别中...' : '手动识别一次'}
          </button>
        </div>
        {isRecognizing && (
            <div style={{ marginTop: '16px', color: '#58a6ff', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
                <span className="loader-spinner" style={{width: 16, height: 16, border: '2px solid #58a6ff', borderBottomColor: 'transparent', borderRadius: '50%', display: 'inline-block', animation: 'spin 1s linear infinite'}}></span>
                <span>正在分析画面...</span>
            </div>
        )}
      </div>

      <div className="recognized-cards-list" style={{ padding: '0 12px' }}>
        {recognizedCards.length === 0 ? (
          <div className="empty-hint" style={{ 
              textAlign: 'center', 
              marginTop: '60px', 
              color: '#484f58',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              gap: '12px'
          }}>
            <div style={{ fontSize: '24px', opacity: 0.5 }}>🔍</div>
            <div>等待识别结果...</div>
          </div>
        ) : (
          <div className="fade-in">
              <div style={{ 
                  fontSize: '12px', 
                  color: '#8b949e', 
                  marginBottom: '12px',
                  paddingLeft: '4px',
                  borderLeft: '2px solid #58a6ff'
              }}>
                  识别结果 (按置信度排序)
              </div>
              <div className="cards-grid" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                {recognizedCards.map((card, index) => (
                    <div key={card.uuid || index} className="recognition-result-item" style={{ position: 'relative' }}>
                        {card.matchLabel && (
                            <div style={{
                                position: 'absolute',
                                top: '8px',
                                right: '8px',
                                zIndex: 10,
                                background: index === 0 ? 'rgba(46, 160, 67, 0.9)' : 'rgba(255, 191, 0, 0.9)',
                                color: '#fff',
                                padding: '4px 10px',
                                borderRadius: '12px',
                                fontSize: '11px',
                                fontWeight: 'bold',
                                boxShadow: '0 2px 8px rgba(0,0,0,0.3)',
                                display: 'flex',
                                alignItems: 'center',
                                gap: '4px'
                            }}>
                                {card.matchLabel}
                                {card.matchConfidence && (
                                    <span style={{ fontSize: '10px', opacity: 0.9 }}>
                                        ({(card.matchConfidence * 100).toFixed(0)}%)
                                    </span>
                                )}
                            </div>
                        )}
                        {renderItemCard(card, false, () => {}, card.displayImg)}
                        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '8px' }}>
                          <button
                            className="bulk-btn"
                            style={{ padding: '4px 10px', fontSize: '12px' }}
                            onClick={() => onToggleExpand(card.instance_id || card.uuid)}
                          >
                            {expandedItems.has(card.instance_id || card.uuid) ? '收起详情' : '展开详情'}
                          </button>
                        </div>
                    </div>
                ))}
              </div>
          </div>
        )}
      </div>
      <style>{`
        @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
        .fade-in { animation: fadeIn 0.3s ease-out; }
        @keyframes fadeIn { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: translateY(0); } }
      `}</style>
    </div>
  );
};
