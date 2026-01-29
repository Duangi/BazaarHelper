import React from 'react';
import { ItemData } from '../types';

interface ItemsViewProps {
  handItems: ItemData[];
  stashItems: ItemData[];
  pinnedItems: Map<string, number>;
  expandedItems: Set<string>;
  onTogglePin: (id: string, e: React.MouseEvent) => void;
  onToggleExpand: (uuid: string) => void;
  renderItemCard: (item: ItemData, isPinned: boolean, onPin: (e: React.MouseEvent) => void) => React.ReactNode;
  getSortedItems: (items: ItemData[]) => ItemData[];
}

export const ItemsView: React.FC<ItemsViewProps> = ({
  handItems,
  stashItems,
  pinnedItems,
  // expandedItems, // Unused for now
  onTogglePin,
  // onToggleExpand, // Unused for now
  renderItemCard,
  getSortedItems
}) => {
  const sortedHand = getSortedItems(handItems);
  const sortedStash = getSortedItems(stashItems);

  return (
    <div className="items-container">
      <div className="section">
        <h3 className="section-title">手牌 ({sortedHand.length})</h3>
        <div className="items-grid">
          {sortedHand.map(item => {
            const itemId = item.instance_id || item.uuid;
            const isPinned = pinnedItems.has(itemId) || pinnedItems.has(item.uuid);
            return (
              <div key={itemId}>
                {renderItemCard(item, isPinned, (e) => onTogglePin(itemId, e))}
              </div>
            );
          })}
        </div>
      </div>

      <div className="section">
        <h3 className="section-title">仓库 ({sortedStash.length})</h3>
        <div className="items-grid">
          {sortedStash.map(item => {
            const itemId = item.instance_id || item.uuid;
            const isPinned = pinnedItems.has(itemId) || pinnedItems.has(item.uuid);
            return (
              <div key={itemId}>
                {renderItemCard(item, isPinned, (e) => onTogglePin(itemId, e))}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
