import { useCallback, useEffect, useRef, useState, type UIEvent } from 'react';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';

import type { SearchItemLite, TabType } from '../../types';
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
  const SEARCH_IMAGE_CACHE_MAX = 240;
  const SEARCH_IMAGE_CACHE_WARM = 80;
  const SEARCH_THUMB_CACHE_MAX = 320;
  const SEARCH_THUMB_CACHE_WARM = 120;
  const SEARCH_IMAGE_BATCH_SIZE = 16;
  const imageCacheRef = useRef<Map<string, string>>(new Map());
  const imageLoadingRef = useRef<Set<string>>(new Set());
  const thumbUrlCacheRef = useRef<Map<string, string>>(new Map());
  const scrollRafRef = useRef<number | null>(null);
  const pendingScrollRef = useRef<{ top: number; height: number } | null>(null);
  const searchResultsRef = useRef<SearchItemLite[]>([]);
  const lastQuerySignatureRef = useRef<string>('');
  const [searchQuery, setSearchQuery] = useState({
    keyword: '',
    item_type: 'all',
    size: '',
    start_tier: '',
    hero: '',
    tags: '',
    hidden_tags: '',
  });
  const [searchResults, setSearchResults] = useState<SearchItemLite[]>([]);
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
  const [searchScrollTop, setSearchScrollTop] = useState(0);
  const [searchViewportHeight, setSearchViewportHeight] = useState(0);
  const scrollAreaRef = useRef<HTMLDivElement | null>(null);

  const resolveSearchImagePath = useCallback(
    (item: SearchItemLite) => {
      const art = item.uuid ? skillsArtMap[item.uuid] : undefined;
      if (art) {
        const base = art.split('/').pop() || art;
        const nameNoExt = base.replace(/\.[^/.]+$/, '');
        return `images/skill/${nameNoExt}.webp`;
      }
      return `images/${item.uuid}.webp`;
    },
    [skillsArtMap],
  );

  const resolveSearchThumbUrlsBatch = useCallback(async (resourcePaths: string[]) => {
    const resolved = new Map<string, string>();
    const unique = [...new Set(resourcePaths.filter((p) => !!p))];

    unique.forEach((path) => {
      const cached = thumbUrlCacheRef.current.get(path);
      if (cached) {
        resolved.set(path, cached);
      }
    });

    const missing = unique.filter((path) => !resolved.has(path));
    if (missing.length > 0) {
      try {
        const mapped = await invoke<Record<string, string>>('get_search_thumbnail_paths', {
          resourcePaths: missing,
        });
        missing.forEach((path) => {
          const thumbPath = mapped?.[path];
          if (!thumbPath) return;
          const thumbUrl = convertFileSrc(thumbPath);
          thumbUrlCacheRef.current.set(path, thumbUrl);
          resolved.set(path, thumbUrl);
        });
      } catch {
        // keep unresolved and fallback below
      }
    }

    const unresolved = missing.filter((path) => !resolved.has(path));
    if (unresolved.length > 0) {
      await Promise.all(
        unresolved.map(async (path) => {
          try {
            const fallback = await getImg(path);
            if (fallback) {
              thumbUrlCacheRef.current.set(path, fallback);
              resolved.set(path, fallback);
            }
          } catch {
            // ignore per-image fallback failure
          }
        }),
      );
    }

    if (thumbUrlCacheRef.current.size > SEARCH_THUMB_CACHE_MAX) {
      const keys = [...thumbUrlCacheRef.current.keys()];
      for (const staleKey of keys.slice(0, keys.length - SEARCH_THUMB_CACHE_WARM)) {
        thumbUrlCacheRef.current.delete(staleKey);
      }
    }

    return resolved;
  }, []);

  const areSearchListsEquivalent = useCallback((left: SearchItemLite[], right: SearchItemLite[]) => {
    if (left.length !== right.length) return false;
    for (let i = 0; i < left.length; i += 1) {
      const l = left[i];
      const r = right[i];
      if (l.uuid !== r.uuid) return false;
      if (l.displayImg !== r.displayImg) return false;
      if (l.name_cn !== r.name_cn) return false;
      if (l.tier !== r.tier) return false;
    }
    return true;
  }, []);

  useEffect(() => {
    searchResultsRef.current = searchResults;
  }, [searchResults]);

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    if (activeTab !== 'search') return;

    const timer = window.setInterval(() => {
      console.debug(
        `[SearchDiag] results=${searchResultsRef.current.length} ` +
        `imgCache=${imageCacheRef.current.size} ` +
        `imgLoading=${imageLoadingRef.current.size} ` +
        `thumbCache=${thumbUrlCacheRef.current.size}`,
      );
    }, 15_000);

    return () => {
      window.clearInterval(timer);
    };
  }, [activeTab]);

  useEffect(() => {
    if (activeTab !== 'search') return;
    window.requestAnimationFrame(() => {
      if (scrollAreaRef.current) {
        scrollAreaRef.current.scrollTop = 0;
        setSearchScrollTop(0);
        setSearchViewportHeight(scrollAreaRef.current.clientHeight);
      }
    });
  }, [activeTab, searchQuery, selectedTags, selectedHiddenTags, matchMode, selectedDay]);

  const handleScroll = (e: UIEvent<HTMLDivElement>) => {
    if (activeTab !== 'search') return;
    const target = e.currentTarget;
    pendingScrollRef.current = {
      top: target.scrollTop,
      height: target.clientHeight,
    };

    if (scrollRafRef.current !== null) return;
    scrollRafRef.current = window.requestAnimationFrame(() => {
      scrollRafRef.current = null;
      const pending = pendingScrollRef.current;
      if (!pending) return;
      setSearchScrollTop(pending.top);
      if (pending.height !== searchViewportHeight) {
        setSearchViewportHeight(pending.height);
      }
      pendingScrollRef.current = null;
    });
  };

  useEffect(() => {
    return () => {
      if (scrollRafRef.current !== null) {
        window.cancelAnimationFrame(scrollRafRef.current);
        scrollRafRef.current = null;
      }
      pendingScrollRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (activeTab !== 'search') {
      if (scrollRafRef.current !== null) {
        window.cancelAnimationFrame(scrollRafRef.current);
        scrollRafRef.current = null;
      }
      pendingScrollRef.current = null;
    }
  }, [activeTab]);

  useEffect(() => {
    if (activeTab !== 'search') return;
    const node = scrollAreaRef.current;
    if (!node) return;

    const updateViewport = () => {
      setSearchViewportHeight(node.clientHeight);
      setSearchScrollTop(node.scrollTop);
    };

    updateViewport();
    window.addEventListener('resize', updateViewport);
    return () => window.removeEventListener('resize', updateViewport);
  }, [activeTab, isSearchFilterCollapsed, searchFilterHeight, searchResults.length]);

  useEffect(() => {
    const handler = setTimeout(async () => {
      if (activeTab === 'search') {
        const querySignature = JSON.stringify({
          searchQuery,
          selectedTags,
          selectedHiddenTags,
          matchMode,
        });
        if (
          querySignature === lastQuerySignatureRef.current
          && searchResultsRef.current.length > 0
        ) {
          return;
        }

        // Dev-only guardrail:
        // avoid loading the full encyclopedia list on an empty broad query, which causes
        // large transient allocations while profiling in tauri dev.
        const broadQuery =
          !searchQuery.keyword.trim()
          && searchQuery.item_type === 'all'
          && !searchQuery.size
          && !searchQuery.start_tier
          && !searchQuery.hero
          && selectedTags.length === 0
          && selectedHiddenTags.length === 0;
        if (import.meta.env.DEV && broadQuery) {
          lastQuerySignatureRef.current = querySignature;
          setSearchResults((prev) => (prev.length === 0 ? prev : []));
          return;
        }

        setIsSearching(true);
        try {
          const res = await invoke<SearchItemLite[]>('search_items_light', { query: searchQuery });

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

          const patched = filtered.map((item) => {
            const key = item.uuid || item.name;
            const cached = imageCacheRef.current.get(key);
            return cached ? { ...item, displayImg: cached } : item;
          });

          lastQuerySignatureRef.current = querySignature;
          setSearchResults((prev) => (areSearchListsEquivalent(prev, patched) ? prev : patched));
        } catch (e) {
          console.error('Search failed:', e);
        } finally {
          setIsSearching(false);
        }
      }
    }, 300);

    return () => clearTimeout(handler);
  }, [searchQuery, activeTab, selectedTags, selectedHiddenTags, matchMode, areSearchListsEquivalent]);

  useEffect(() => {
    if (activeTab !== 'search' || searchResults.length === 0) return;

    const estimatedRowHeight = 132;
    const anchorIndex = Math.max(
      0,
      Math.min(searchResults.length - 1, Math.floor(searchScrollTop / estimatedRowHeight)),
    );

    // Priority queue around current viewport anchor.
    // This is more robust than strict [start, end] slicing when users scroll fast.
    const candidateRadius = 240;
    const nearStart = Math.max(0, anchorIndex - candidateRadius);
    const nearEnd = Math.min(searchResults.length, anchorIndex + candidateRadius);
    const nearCandidates = searchResults
      .slice(nearStart, nearEnd)
      .map((item, localIdx) => ({
        item,
        index: nearStart + localIdx,
      }));

    const farCandidates = searchResults
      .map((item, idx) => ({ item, index: idx }))
      .filter(({ index }) => index < nearStart || index >= nearEnd);

    const missing = [...nearCandidates, ...farCandidates]
      .filter(({ item }) => {
        const key = item.uuid || item.name;
        if (item.displayImg) return false;
        if (imageCacheRef.current.has(key)) return false;
        if (imageLoadingRef.current.has(key)) return false;
        return true;
      })
      .sort((a, b) => {
        const da = Math.abs(a.index - anchorIndex);
        const db = Math.abs(b.index - anchorIndex);
        return da - db;
      });

    const pendingItems = missing.slice(0, 96).map(({ item }) => item);
    if (pendingItems.length === 0) return;

    let cancelled = false;
    const run = async () => {
      const loadedPairs: Array<{ key: string; url: string }> = [];
      for (let i = 0; i < pendingItems.length; i += SEARCH_IMAGE_BATCH_SIZE) {
        if (cancelled) break;
        const batch = pendingItems.slice(i, i + SEARCH_IMAGE_BATCH_SIZE);
        const keyedBatch = batch.map((item) => ({
          key: item.uuid || item.name,
          resourcePath: resolveSearchImagePath(item),
        }));
        keyedBatch.forEach(({ key }) => imageLoadingRef.current.add(key));

        try {
          const resolved = await resolveSearchThumbUrlsBatch(
            keyedBatch.map(({ resourcePath }) => resourcePath),
          );
          if (cancelled) break;
          keyedBatch.forEach(({ key, resourcePath }) => {
            const url = resolved.get(resourcePath);
            if (!url) return;
            imageCacheRef.current.set(key, url);
            loadedPairs.push({ key, url });
          });
        } catch (error) {
          console.warn('[Search] lazy image load failed:', error);
        } finally {
          keyedBatch.forEach(({ key }) => imageLoadingRef.current.delete(key));
        }

        if (imageCacheRef.current.size > SEARCH_IMAGE_CACHE_MAX) {
          const keys = [...imageCacheRef.current.keys()];
          for (const staleKey of keys.slice(0, keys.length - SEARCH_IMAGE_CACHE_WARM)) {
            imageCacheRef.current.delete(staleKey);
          }
        }
      }

      if (cancelled || loadedPairs.length === 0) return;

      setSearchResults((prev) => {
        const patchMap = new Map<string, string>();
        loadedPairs.forEach(({ key, url }) => patchMap.set(key, url));
        let changed = false;
        const next = prev.map((entry) => {
          const entryKey = entry.uuid || entry.name;
          const patchedUrl = patchMap.get(entryKey);
          if (!patchedUrl || entry.displayImg === patchedUrl) return entry;
          changed = true;
          return { ...entry, displayImg: patchedUrl };
        });
        return changed ? next : prev;
      });
    };

    void run();

    return () => {
      cancelled = true;
    };
  }, [activeTab, resolveSearchImagePath, resolveSearchThumbUrlsBatch, searchResults, searchScrollTop, searchViewportHeight]);

  useEffect(() => {
    if (activeTab === 'search') return;

    const cleanupTimer = window.setTimeout(() => {
      imageLoadingRef.current.clear();
      lastQuerySignatureRef.current = '';
      setSearchResults((prev) => (prev.length === 0 ? prev : []));
      setSearchScrollTop(0);
      setSearchViewportHeight(0);
      if (imageCacheRef.current.size > 40) {
        imageCacheRef.current.clear();
      }
      if (thumbUrlCacheRef.current.size > 80) {
        thumbUrlCacheRef.current.clear();
      }
    }, 10_000);

    return () => window.clearTimeout(cleanupTimer);
  }, [activeTab]);

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
    scrollAreaRef,
    handleScroll,
    searchScrollTop,
    searchViewportHeight,
  };
};
