import * as React from 'react';
import { invoke } from '@tauri-apps/api/core';

import type { ItemData, MatchHistoryRecord, MonsterData, SearchItemLite, TabType, Toast } from '../../types';
import { MainShell } from './MainShell';
import { AppSettingsPanel } from '../settings/AppSettingsPanel';

const SearchFiltersPanelLazy = React.lazy(async () => ({
  default: (await import('../search/SearchFiltersPanel')).SearchFiltersPanel,
}));

const VirtualSearchResultsLazy = React.lazy(async () => ({
  default: (await import('../search/VirtualSearchResults')).VirtualSearchResults,
}));

const HistoryViewLazy = React.lazy(async () => ({
  default: (await import('../../views/HistoryView')).HistoryView,
}));

const MonsterTabViewLazy = React.lazy(async () => ({
  default: (await import('../../views/MonsterTabView')).MonsterTabView,
}));

const CardRecognitionViewLazy = React.lazy(async () => ({
  default: (await import('../../views/CardRecognitionView')).CardRecognitionView,
}));

const ItemsViewLazy = React.lazy(async () => ({
  default: (await import('../../views/ItemsView')).ItemsView,
}));

interface MainContentSectionProps {
  isCollapsed: boolean;
  activeTab: TabType;
  setActiveTab: React.Dispatch<React.SetStateAction<TabType>>;
  showSettings: boolean;
  setShowSettings: React.Dispatch<React.SetStateAction<boolean>>;
  settingsPanelProps: React.ComponentProps<typeof AppSettingsPanel>;
  isSearchFilterCollapsed: boolean;
  setIsSearchFilterCollapsed: React.Dispatch<React.SetStateAction<boolean>>;
  matchMode: 'all' | 'any';
  setMatchMode: React.Dispatch<React.SetStateAction<'all' | 'any'>>;
  searchFilterHeight: number;
  searchQuery: {
    keyword: string;
    item_type: string;
    size: string;
    start_tier: string;
    hero: string;
    tags: string;
    hidden_tags: string;
  };
  setSearchQuery: React.Dispatch<
    React.SetStateAction<{
      keyword: string;
      item_type: string;
      size: string;
      start_tier: string;
      hero: string;
      tags: string;
      hidden_tags: string;
    }>
  >;
  setIsInputFocused: React.Dispatch<React.SetStateAction<boolean>>;
  lastItemSize: string;
  setLastItemSize: React.Dispatch<React.SetStateAction<string>>;
  selectedTags: string[];
  setSelectedTags: React.Dispatch<React.SetStateAction<string[]>>;
  selectedHiddenTags: string[];
  setSelectedHiddenTags: React.Dispatch<React.SetStateAction<string[]>>;
  hiddenTagIcons: Record<string, string>;
  isSearching: boolean;
  searchResults: SearchItemLite[];
  setResizeStartY: React.Dispatch<React.SetStateAction<number>>;
  setResizeStartHeight: React.Dispatch<React.SetStateAction<number>>;
  setIsResizingFilter: React.Dispatch<React.SetStateAction<boolean>>;
  isResizingFilter: boolean;
  scrollAreaRef: React.MutableRefObject<HTMLDivElement | null>;
  handleScroll: (e: React.UIEvent<HTMLDivElement>) => void;
  searchScrollTop: number;
  searchViewportHeight: number;
  wrapRef: React.MutableRefObject<HTMLDivElement | null>;
  matchHistory: MatchHistoryRecord[];
  isLoadingHistory: boolean;
  loadMatchHistory: () => Promise<void>;
  selectedDay: string;
  setSelectedDay: React.Dispatch<React.SetStateAction<string>>;
  isRecognizing: boolean;
  handleAutoRecognition: (day: number | null) => Promise<void>;
  showToast: (message: string, type?: Toast['type']) => void;
  templateLoading: { loaded: number; total: number; is_complete: boolean; current_name: string };
  manualMonsters: MonsterData[];
  identifiedNames: string[];
  expandedMonsters: Set<string>;
  toggleMonsterExpand: (name: string) => void;
  renderTierInfo: (tierInfo: any) => React.ReactNode;
  recognizedCards: ItemData[];
  isRecognizingCard: boolean;
  expandedItems: Set<string>;
  toggleExpand: (itemId: string) => void;
  handleRecognizeCard: (forceRefresh?: boolean) => Promise<void>;
  renderUnifiedItemCard: (
    item: ItemData,
    isPinned: boolean,
    onPin: (e: React.MouseEvent) => void,
    imageSrcOverride?: string,
  ) => React.ReactNode;
  handItems: ItemData[];
  stashItems: ItemData[];
  pinnedItems: Map<string, number>;
  togglePin: (itemId: string, e: React.MouseEvent) => void;
  getSortedItems: (items: ItemData[]) => ItemData[];
}

const buildSearchSkeletonItem = (item: SearchItemLite): ItemData => ({
  uuid: item.uuid,
  name: item.name,
  name_cn: item.name_cn,
  tier: item.tier,
  available_tiers: item.available_tiers || '',
  size: item.size,
  tags: item.tags || '',
  hidden_tags: item.hidden_tags || '',
  processed_tags: item.processed_tags || [],
  heroes: item.heroes || [],
  cooldown_tiers: '',
  damage_tiers: '',
  heal_tiers: '',
  shield_tiers: '',
  ammo_tiers: '',
  crit_tiers: '',
  multicast_tiers: '',
  burn_tiers: '',
  poison_tiers: '',
  regen_tiers: '',
  lifesteal_tiers: '',
  skills: [],
  enchantments: [],
  description: '',
  image: '',
  displayImg: item.displayImg,
});

function MainContentSectionImpl(props: MainContentSectionProps) {
  const {
    isCollapsed,
    activeTab,
    setActiveTab,
    showSettings,
    setShowSettings,
    settingsPanelProps,
    isSearchFilterCollapsed,
    setIsSearchFilterCollapsed,
    matchMode,
    setMatchMode,
    searchFilterHeight,
    searchQuery,
    setSearchQuery,
    setIsInputFocused,
    lastItemSize,
    setLastItemSize,
    selectedTags,
    setSelectedTags,
    selectedHiddenTags,
    setSelectedHiddenTags,
    hiddenTagIcons,
    isSearching,
    searchResults,
    setResizeStartY,
    setResizeStartHeight,
    setIsResizingFilter,
    isResizingFilter,
    scrollAreaRef,
    handleScroll,
    searchScrollTop,
    searchViewportHeight,
    wrapRef,
    matchHistory,
    isLoadingHistory,
    loadMatchHistory,
    selectedDay,
    setSelectedDay,
    isRecognizing,
    handleAutoRecognition,
    showToast,
    templateLoading,
    manualMonsters,
    identifiedNames,
    expandedMonsters,
    toggleMonsterExpand,
    renderTierInfo,
    recognizedCards,
    isRecognizingCard,
    expandedItems,
    toggleExpand,
    handleRecognizeCard,
    renderUnifiedItemCard,
    handItems,
    stashItems,
    pinnedItems,
    togglePin,
    getSortedItems,
  } = props;

  const [searchDetailMap, setSearchDetailMap] = React.useState<Record<string, ItemData>>({});
  const [searchDetailLoading, setSearchDetailLoading] = React.useState<Set<string>>(new Set());
  const [searchDetailMissing, setSearchDetailMissing] = React.useState<Set<string>>(new Set());

  React.useEffect(() => {
    if (activeTab !== 'search') return;
    const currentIds = new Set(searchResults.map((item) => item.uuid));
    setSearchDetailMap((prev) => {
      const next: Record<string, ItemData> = {};
      Object.entries(prev).forEach(([id, detail]) => {
        if (currentIds.has(id)) next[id] = detail;
      });
      return next;
    });
    setSearchDetailLoading((prev) => {
      const next = new Set<string>();
      prev.forEach((id) => {
        if (currentIds.has(id)) next.add(id);
      });
      return next;
    });
    setSearchDetailMissing((prev) => {
      const next = new Set<string>();
      prev.forEach((id) => {
        if (currentIds.has(id)) next.add(id);
      });
      return next;
    });
  }, [activeTab, searchResults]);

  React.useEffect(() => {
    if (activeTab === 'search') return;
    if (Object.keys(searchDetailMap).length > 0) {
      setSearchDetailMap({});
    }
    if (searchDetailLoading.size > 0) {
      setSearchDetailLoading(new Set());
    }
    if (searchDetailMissing.size > 0) {
      setSearchDetailMissing(new Set());
    }
  }, [activeTab, searchDetailLoading, searchDetailMap, searchDetailMissing]);

  React.useEffect(() => {
    if (activeTab !== 'search') return;
    const expandedSearchIds = searchResults
      .filter((item) => expandedItems.has(item.uuid))
      .map((item) => item.uuid)
      .filter((id) => !searchDetailMap[id] && !searchDetailLoading.has(id) && !searchDetailMissing.has(id));

    if (expandedSearchIds.length === 0) return;

    setSearchDetailLoading((prev) => {
      const next = new Set(prev);
      expandedSearchIds.forEach((id) => next.add(id));
      return next;
    });

    let cancelled = false;
    const fetchDetails = async () => {
      const settled = await Promise.all(
        expandedSearchIds.map(async (id) => {
          try {
            const detail = await invoke<ItemData | null>('get_item_info', { id });
            return { id, detail };
          } catch (error) {
            console.error('[Search] load detail failed:', id, error);
            return { id, detail: null };
          }
        }),
      );

      if (cancelled) {
        setSearchDetailLoading((prev) => {
          const next = new Set(prev);
          expandedSearchIds.forEach((id) => next.delete(id));
          return next;
        });
        return;
      }

      const missingIds: string[] = [];
      const foundIds: string[] = [];

      setSearchDetailMap((prev) => {
        const next = { ...prev };
        settled.forEach(({ id, detail }) => {
          if (detail) {
            next[id] = detail;
            foundIds.push(id);
          } else {
            missingIds.push(id);
          }
        });
        return next;
      });

      setSearchDetailLoading((prev) => {
        const next = new Set(prev);
        expandedSearchIds.forEach((id) => next.delete(id));
        return next;
      });

      if (missingIds.length > 0 || foundIds.length > 0) {
        setSearchDetailMissing((prev) => {
          const next = new Set(prev);
          foundIds.forEach((id) => next.delete(id));
          missingIds.forEach((id) => next.add(id));
          return next;
        });
      }
    };

    void fetchDetails();
    return () => {
      cancelled = true;
      setSearchDetailLoading((prev) => {
        const next = new Set(prev);
        expandedSearchIds.forEach((id) => next.delete(id));
        return next;
      });
    };
  }, [activeTab, expandedItems, searchResults]);

  if (isCollapsed) return null;

  return (
    <MainShell
      activeTab={activeTab}
      onTabChange={(tab) => {
        setShowSettings(false);
        setActiveTab(tab);
      }}
      onOpenSettings={() => setShowSettings(true)}
    >
      {!showSettings && activeTab === 'search' && (
        <React.Suspense fallback={null}>
          <SearchFiltersPanelLazy
            isSearchFilterCollapsed={isSearchFilterCollapsed}
            setIsSearchFilterCollapsed={setIsSearchFilterCollapsed}
            matchMode={matchMode}
            setMatchMode={setMatchMode}
            searchFilterHeight={searchFilterHeight}
            searchQuery={searchQuery}
            setSearchQuery={setSearchQuery}
            setIsInputFocused={setIsInputFocused}
            lastItemSize={lastItemSize}
            setLastItemSize={setLastItemSize}
            selectedTags={selectedTags}
            setSelectedTags={setSelectedTags}
            selectedHiddenTags={selectedHiddenTags}
            setSelectedHiddenTags={setSelectedHiddenTags}
            hiddenTagIcons={hiddenTagIcons}
            isSearching={isSearching}
            searchResultsCount={searchResults.length}
            setResizeStartY={setResizeStartY}
            setResizeStartHeight={setResizeStartHeight}
            setIsResizingFilter={setIsResizingFilter}
            isResizingFilter={isResizingFilter}
          />
        </React.Suspense>
      )}

      {showSettings ? (
        <div className="scroll-area" ref={scrollAreaRef} data-no-drag>
          <div className="items" ref={wrapRef}>
            <AppSettingsPanel {...settingsPanelProps} visible inline />
          </div>
        </div>
      ) : (
        <div className="scroll-area" ref={scrollAreaRef} onScroll={handleScroll} data-no-drag>
          <div className="items" ref={wrapRef}>
            {activeTab === 'history' ? (
              <React.Suspense fallback={<div className="empty-tip">加载历史中...</div>}>
                <HistoryViewLazy records={matchHistory} isLoading={isLoadingHistory} onReload={loadMatchHistory} showToast={showToast} />
              </React.Suspense>
            ) : activeTab === 'monster' ? (
              <React.Suspense fallback={<div className="empty-tip">加载野怪页面...</div>}>
                <MonsterTabViewLazy
                  selectedDay={selectedDay}
                  setSelectedDay={setSelectedDay}
                  isRecognizing={isRecognizing}
                  handleAutoRecognition={handleAutoRecognition}
                  showToast={showToast}
                  templateLoading={templateLoading}
                  manualMonsters={manualMonsters}
                  identifiedNames={identifiedNames}
                  expandedMonsters={expandedMonsters}
                  toggleMonsterExpand={toggleMonsterExpand}
                  renderTierInfo={renderTierInfo}
                />
              </React.Suspense>
            ) : (
              <>
                {activeTab === 'card' && (
                  <React.Suspense fallback={<div className="empty-tip">加载卡牌识别...</div>}>
                    <CardRecognitionViewLazy
                      recognizedCards={recognizedCards}
                      isRecognizing={isRecognizingCard}
                      expandedItems={expandedItems}
                      onToggleExpand={toggleExpand}
                      onRecognize={() => handleRecognizeCard(false)}
                      renderItemCard={renderUnifiedItemCard}
                    />
                  </React.Suspense>
                )}

                {activeTab === 'items' && (
                  <React.Suspense fallback={<div className="empty-tip">加载手头物品...</div>}>
                    <ItemsViewLazy
                      handItems={handItems}
                      stashItems={stashItems}
                      pinnedItems={pinnedItems}
                      expandedItems={expandedItems}
                      onTogglePin={togglePin}
                      onToggleExpand={toggleExpand}
                      renderItemCard={renderUnifiedItemCard}
                      getSortedItems={getSortedItems}
                    />
                  </React.Suspense>
                )}

                {activeTab === 'search' && (
                  <div className="card-list">
                    {searchResults.length > 0 ? (
                      <React.Suspense fallback={<div className="empty-tip">加载搜索结果...</div>}>
                        <VirtualSearchResultsLazy
                          items={searchResults}
                          scrollTop={searchScrollTop}
                          viewportHeight={searchViewportHeight}
                          isItemExpanded={(item) => expandedItems.has(item.uuid)}
                          renderItem={(item) => {
                            const detail = searchDetailMap[item.uuid];
                            const isLoadingDetail = searchDetailLoading.has(item.uuid);
                            const isMissingDetail = searchDetailMissing.has(item.uuid);
                            const renderItemData = detail
                              ? { ...detail, displayImg: item.displayImg || detail.displayImg }
                              : {
                                  ...buildSearchSkeletonItem(item),
                                  description: isLoadingDetail
                                    ? '正在加载详情...'
                                    : (isMissingDetail ? '暂无该条目的详细数据' : ''),
                                };
                            return renderUnifiedItemCard(
                              renderItemData,
                              pinnedItems.has(item.uuid),
                              (e) => togglePin(item.uuid, e),
                            );
                          }}
                        />
                      </React.Suspense>
                    ) : null}
                    {searchResults.length === 0 && <div className="empty-tip">未找到结果</div>}
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      )}
    </MainShell>
  );
}

export const MainContentSection = React.memo(MainContentSectionImpl);
