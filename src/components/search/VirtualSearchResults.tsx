import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';

import type { SearchItemLite } from '../../types';

interface VirtualSearchResultsProps {
  items: SearchItemLite[];
  scrollTop: number;
  viewportHeight: number;
  renderItem: (item: SearchItemLite) => ReactNode;
  isItemExpanded: (item: SearchItemLite) => boolean;
  overscanPx?: number;
}

const DEFAULT_ROW_HEIGHT = 132;
const DEFAULT_EXPANDED_ROW_HEIGHT = 360;
const DEFAULT_OVERSCAN_PX = 320;

const getItemKey = (item: SearchItemLite, index: number) => item.uuid || `${item.name}-${index}`;

const findPrefixIndex = (prefix: number[], target: number): number => {
  let left = 0;
  let right = prefix.length - 1;

  while (left < right) {
    const mid = Math.floor((left + right + 1) / 2);
    if (prefix[mid] <= target) left = mid;
    else right = mid - 1;
  }

  return Math.max(0, Math.min(left, prefix.length - 2));
};

export function VirtualSearchResults({
  items,
  scrollTop,
  viewportHeight,
  renderItem,
  isItemExpanded,
  overscanPx = DEFAULT_OVERSCAN_PX,
}: VirtualSearchResultsProps) {
  const measuredHeightsRef = useRef<Map<string, number>>(new Map());
  const [measureVersion, setMeasureVersion] = useState(0);

  useEffect(() => {
    if (items.length === 0) {
      if (measuredHeightsRef.current.size > 0) {
        measuredHeightsRef.current.clear();
        setMeasureVersion((v) => v + 1);
      }
      return;
    }

    const validKeys = new Set<string>();
    items.forEach((item, index) => validKeys.add(getItemKey(item, index)));

    if (measuredHeightsRef.current.size <= validKeys.size + 24) return;

    let pruned = false;
    for (const key of [...measuredHeightsRef.current.keys()]) {
      if (!validKeys.has(key)) {
        measuredHeightsRef.current.delete(key);
        pruned = true;
      }
    }
    if (pruned) {
      setMeasureVersion((v) => v + 1);
    }
  }, [items]);

  const measureRow = useCallback((key: string) => {
    return (node: HTMLDivElement | null) => {
      if (!node) return;
      const height = Math.ceil(node.getBoundingClientRect().height);
      const prev = measuredHeightsRef.current.get(key);
      if (prev === undefined || Math.abs(prev - height) > 1) {
        measuredHeightsRef.current.set(key, height);
        setMeasureVersion((v) => v + 1);
      }
    };
  }, []);

  const layout = useMemo(() => {
    if (items.length === 0) {
      return { prefix: [0], totalHeight: 0 };
    }

    const prefix = new Array<number>(items.length + 1);
    prefix[0] = 0;

    for (let i = 0; i < items.length; i += 1) {
      const item = items[i];
      const key = getItemKey(item, i);
      const measured = measuredHeightsRef.current.get(key);
      const estimated = isItemExpanded(item) ? DEFAULT_EXPANDED_ROW_HEIGHT : DEFAULT_ROW_HEIGHT;
      prefix[i + 1] = prefix[i] + (measured || estimated);
    }

    return {
      prefix,
      totalHeight: prefix[items.length],
    };
  }, [items, isItemExpanded, measureVersion]);

  const { startIndex, endIndex, topSpacerHeight, bottomSpacerHeight } = useMemo(() => {
    if (items.length === 0) {
      return { startIndex: 0, endIndex: -1, topSpacerHeight: 0, bottomSpacerHeight: 0 };
    }

    const { prefix, totalHeight } = layout;
    const viewHeight = viewportHeight > 0 ? viewportHeight : 900;
    const windowTop = Math.max(0, scrollTop - overscanPx);
    const windowBottom = Math.max(0, Math.min(totalHeight, scrollTop + viewHeight + overscanPx));

    const start = findPrefixIndex(prefix, windowTop);
    const end = Math.min(items.length - 1, findPrefixIndex(prefix, windowBottom));

    return {
      startIndex: start,
      endIndex: end,
      topSpacerHeight: prefix[start],
      bottomSpacerHeight: Math.max(0, totalHeight - prefix[end + 1]),
    };
  }, [items, layout, scrollTop, viewportHeight, overscanPx]);

  if (items.length === 0) return null;

  const visible = items.slice(startIndex, endIndex + 1);

  return (
    <div className="virtual-search-results">
      {topSpacerHeight > 0 ? <div style={{ height: topSpacerHeight }} /> : null}
      {visible.map((item, idx) => {
        const absoluteIndex = startIndex + idx;
        const key = getItemKey(item, absoluteIndex);
        return (
          <div key={key} ref={measureRow(key)}>
            {renderItem(item)}
          </div>
        );
      })}
      {bottomSpacerHeight > 0 ? <div style={{ height: bottomSpacerHeight }} /> : null}
    </div>
  );
}
