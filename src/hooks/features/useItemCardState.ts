import { useCallback, useState } from 'react';
import type React from 'react';

import type { ItemData } from '../../types';

export const useItemCardState = () => {
  const [pinnedItems, setPinnedItems] = useState<Map<string, number>>(new Map());
  const [pinnedCounter, setPinnedCounter] = useState(0);
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set());

  const togglePin = useCallback((id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setPinnedItems((prev) => {
      const newPinned = new Map(prev);
      if (newPinned.has(id)) {
        newPinned.delete(id);
      } else {
        setPinnedCounter((c) => c + 1);
        newPinned.set(id, pinnedCounter + 1);
      }
      return newPinned;
    });
  }, [pinnedCounter]);

  const toggleExpand = useCallback((id: string) => {
    setExpandedItems((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const getSortedItems = useCallback((items: ItemData[]) => {
    const sorted = [...items].sort((a, b) => {
      const aId = a.instance_id || a.uuid;
      const bId = b.instance_id || b.uuid;
      const aPin = pinnedItems.get(aId) || pinnedItems.get(a.uuid);
      const bPin = pinnedItems.get(bId) || pinnedItems.get(b.uuid);

      if (aPin && bPin) return bPin - aPin;
      if (aPin) return -1;
      if (bPin) return 1;
      return 0;
    });

    const seen = new Set<string>();
    return sorted.filter((item) => {
      const key = item.instance_id || item.uuid;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }, [pinnedItems]);

  return {
    pinnedItems,
    expandedItems,
    setExpandedItems,
    togglePin,
    toggleExpand,
    getSortedItems,
  };
};
