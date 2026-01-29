import React from 'react';
import { ItemData } from '../types';

interface CardRecognitionViewProps {
  recognizedCards: ItemData[];
  isRecognizing: boolean;
  expandedItems: Set<string>;
  onToggleExpand: (uuid: string) => void;
  onRecognize: () => void;
  renderItemCard: (item: ItemData, isPinned: boolean, onPin: (e: React.MouseEvent) => void) => React.ReactNode;
}

export const CardRecognitionView: React.FC<CardRecognitionViewProps> = ({
  recognizedCards,
  isRecognizing,
  // expandedItems,
  // onToggleExpand,
  onRecognize,
  renderItemCard
}) => {
  return (
    <div className="card-recognition-view">
      <div className="recognition-controls">
        <button 
          className="recognize-btn" 
          onClick={onRecognize}
          disabled={isRecognizing}
        >
          {isRecognizing ? '识别中...' : '识别鼠标下的卡牌'}
        </button>
        <div className="hint-text">
          将鼠标悬停在游戏中的卡牌上，然后点击按钮识别
        </div>
      </div>

      <div className="recognized-cards-list">
        {recognizedCards.length === 0 ? (
          <div className="empty-hint">
            尚未识别任何卡牌
          </div>
        ) : (
          recognizedCards.map(card => 
            renderItemCard(card, false, () => {})
          )
        )}
      </div>
    </div>
  );
};
