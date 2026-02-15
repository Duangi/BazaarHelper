import { useCallback } from 'react';
import type { MouseEvent } from 'react';

import { UnifiedItemCard } from '../../components/UnifiedItemCard';
import type { ItemData } from '../../types';

interface UseUnifiedItemCardRendererOptions {
  expandedItems: Set<string>;
  activeTab: string;
  recognizedCards: ItemData[];
  allTags: string[];
  toggleExpand: (key: string) => void;
}

export const useUnifiedItemCardRenderer = ({
  expandedItems,
  activeTab,
  recognizedCards,
  allTags,
  toggleExpand,
}: UseUnifiedItemCardRendererOptions) => {
  return useCallback(
    (item: ItemData, isPinned: boolean, onPin: (e: MouseEvent) => void) => {
      const uniqueKey = item.instance_id || item.uuid;
      const expansionKey = item.instance_id || item.uuid;
      const isExpanded = expandedItems.has(expansionKey);
      const isRecognized = activeTab === 'card';
      const isTopMatch =
        recognizedCards.length > 0 &&
        (item === recognizedCards[0] || item.uuid === recognizedCards[0].uuid);

      return (
        <UnifiedItemCard
          key={uniqueKey}
          item={item}
          allTags={allTags}
          isExpanded={isExpanded}
          isPinned={isPinned}
          isRecognized={isRecognized}
          isTopMatch={isTopMatch}
          showPin={activeTab !== 'search'}
          showExpandChevron
          onToggleExpand={() => toggleExpand(expansionKey)}
          onPin={onPin}
        />
      );
    },
    [activeTab, allTags, expandedItems, recognizedCards, toggleExpand],
  );
};
