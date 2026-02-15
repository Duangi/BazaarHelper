import { useEffect, useRef, useState, type UIEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';

import type { ItemData, TabType } from '../../types';
import { getImg } from '../../utils/helpers';

interface UseSearchPanelStateOptions {
  activeTab: TabType;
  selectedDay: string;
  skillsArtMap: Record<string, string>;
}

export const useSearchPanelState = ({
  activeTab,
  selectedDay,
  skillsArtMap,
}: UseSearchPanelStateOptions) => {
  const [searchQuery, setSearchQuery] = useState({
    keyword: '',
    item_type: 'all',
    size: '',
    start_tier: '',
    hero: '',
    tags: '',
    hidden_tags: '',
  });
  const [searchResults, setSearchResults] = useState<ItemData[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [isSearchFilterCollapsed, setIsSearchFilterCollapsed] = useState(false);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [selectedHiddenTags, setSelectedHiddenTags] = useState<string[]>([]);
  const [matchMode, setMatchMode] = useState<'all' | 'any'>('all');
  const [isInputFocused, setIsInputFocused] = useState(false);
  const [lastItemSize, setLastItemSize] = useState('');
  const [searchFilterHeight, setSearchFilterHeight] = useState(300);
  const [isResizingFilter, setIsResizingFilter] = useState(false);
  const [resizeStartY, setResizeStartY] = useState(0);
  const [resizeStartHeight, setResizeStartHeight] = useState(0);
  const [visibleCount, setVisibleCount] = useState(50);
  const scrollAreaRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (activeTab !== 'search') return;
    setVisibleCount(50);
    window.requestAnimationFrame(() => {
      if (scrollAreaRef.current) {
        scrollAreaRef.current.scrollTop = 0;
      }
    });
  }, [activeTab, searchQuery, selectedTags, selectedHiddenTags, matchMode, selectedDay]);

  const handleScroll = (e: UIEvent<HTMLDivElement>) => {
    const { scrollTop, scrollHeight, clientHeight } = e.currentTarget;
    if (scrollHeight - scrollTop - clientHeight < 200) {
      setVisibleCount((prev) => prev + 20);
    }
  };

  useEffect(() => {
    const handler = setTimeout(async () => {
      if (activeTab === 'search') {
        setIsSearching(true);
        try {
          const res = await invoke<ItemData[]>('search_items', { query: searchQuery });

          let filtered = res.filter(
            (item) =>
              item.name_cn &&
              item.name_cn.trim() !== '' &&
              !item.name_cn.includes('中型包裹') &&
              !item.name.includes('Medium Package'),
          );

          if (selectedTags.length > 0) {
            filtered = filtered.filter((item) => {
              const itemTags = item.tags.toLowerCase();
              if (matchMode === 'all') {
                return selectedTags.every((tag) => itemTags.includes(tag.toLowerCase()));
              }
              return selectedTags.some((tag) => itemTags.includes(tag.toLowerCase()));
            });
          }

          if (selectedHiddenTags.length > 0) {
            filtered = filtered.filter((item) => {
              const itemHiddenTags = item.hidden_tags.toLowerCase();
              if (matchMode === 'all') {
                return selectedHiddenTags.every((tag) => itemHiddenTags.includes(tag.toLowerCase()));
              }
              return selectedHiddenTags.some((tag) => itemHiddenTags.includes(tag.toLowerCase()));
            });
          }

          const patched = await Promise.all(
            filtered.map(async (item) => {
              let imgPath = '';
              const art = item.uuid ? skillsArtMap[item.uuid] : undefined;

              if (art) {
                const base = art.split('/').pop() || art;
                const nameNoExt = base.replace(/\.[^/.]+$/, '');
                imgPath = `images/skill/${nameNoExt}.webp`;
              } else {
                imgPath = `images/${item.uuid}.webp`;
              }

              const url = await getImg(imgPath);

              if ((item as any).skills_passive || (item as any).quests) {
                console.log('[DEBUG] Item with skills_passive/quests:', item.name_cn, {
                  skills_passive: (item as any).skills_passive,
                  quests: (item as any).quests,
                });
              }

              return { ...item, displayImg: url };
            }),
          );

          setSearchResults(patched);
        } catch (e) {
          console.error('Search failed:', e);
        } finally {
          setIsSearching(false);
        }
      }
    }, 300);

    return () => clearTimeout(handler);
  }, [searchQuery, activeTab, skillsArtMap, selectedTags, selectedHiddenTags, matchMode]);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isResizingFilter) {
        const deltaY = e.clientY - resizeStartY;
        const newHeight = resizeStartHeight + deltaY;
        setSearchFilterHeight(Math.max(200, Math.min(newHeight, window.innerHeight * 0.6)));
      }
    };

    const handleMouseUp = () => {
      setIsResizingFilter(false);
    };

    if (isResizingFilter) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingFilter, resizeStartY, resizeStartHeight]);

  return {
    searchQuery,
    setSearchQuery,
    searchResults,
    isSearching,
    isSearchFilterCollapsed,
    setIsSearchFilterCollapsed,
    selectedTags,
    setSelectedTags,
    selectedHiddenTags,
    setSelectedHiddenTags,
    matchMode,
    setMatchMode,
    isInputFocused,
    setIsInputFocused,
    lastItemSize,
    setLastItemSize,
    searchFilterHeight,
    isResizingFilter,
    setResizeStartY,
    setResizeStartHeight,
    setIsResizingFilter,
    visibleCount,
    scrollAreaRef,
    handleScroll,
  };
};
