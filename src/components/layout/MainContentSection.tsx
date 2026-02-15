import * as React from 'react';

import type { ItemData, MatchHistoryRecord, MonsterData, TabType, Toast } from '../../types';
import { MainShell } from './MainShell';
import { SearchFiltersPanel } from '../search/SearchFiltersPanel';
import { AppSettingsPanel } from '../settings/AppSettingsPanel';
import { HistoryView } from '../../views/HistoryView';
import { MonsterTabView } from '../../views/MonsterTabView';
import { CardRecognitionView } from '../../views/CardRecognitionView';
import { ItemsView } from '../../views/ItemsView';

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
  searchResults: ItemData[];
  setResizeStartY: React.Dispatch<React.SetStateAction<number>>;
  setResizeStartHeight: React.Dispatch<React.SetStateAction<number>>;
  setIsResizingFilter: React.Dispatch<React.SetStateAction<boolean>>;
  isResizingFilter: boolean;
  scrollAreaRef: React.MutableRefObject<HTMLDivElement | null>;
  handleScroll: (e: React.UIEvent<HTMLDivElement>) => void;
  wrapRef: React.MutableRefObject<HTMLDivElement | null>;
  matchHistory: MatchHistoryRecord[];
  isLoadingHistory: boolean;
  loadMatchHistory: () => Promise<void>;
  selectedDay: string;
  setSelectedDay: React.Dispatch<React.SetStateAction<string>>;
  handleDayChange: (day: number) => Promise<void>;
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
  renderUnifiedItemCard: (item: ItemData, isPinned: boolean, onPin: (e: React.MouseEvent) => void) => React.ReactNode;
  handItems: ItemData[];
  stashItems: ItemData[];
  pinnedItems: Map<string, number>;
  togglePin: (itemId: string, e: React.MouseEvent) => void;
  getSortedItems: (items: ItemData[]) => ItemData[];
  visibleCount: number;
}

export function MainContentSection(props: MainContentSectionProps) {
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
    wrapRef,
    matchHistory,
    isLoadingHistory,
    loadMatchHistory,
    selectedDay,
    setSelectedDay,
    handleDayChange,
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
    visibleCount,
  } = props;

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
        <SearchFiltersPanel
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
              <HistoryView records={matchHistory} isLoading={isLoadingHistory} onReload={loadMatchHistory} />
            ) : activeTab === 'monster' ? (
              <MonsterTabView
                selectedDay={selectedDay}
                setSelectedDay={setSelectedDay}
                handleDayChange={handleDayChange}
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
            ) : (
              <>
                {activeTab === 'card' && (
                  <CardRecognitionView
                    recognizedCards={recognizedCards}
                    isRecognizing={isRecognizingCard}
                    expandedItems={expandedItems}
                    onToggleExpand={toggleExpand}
                    onRecognize={() => handleRecognizeCard(false)}
                    renderItemCard={renderUnifiedItemCard}
                  />
                )}

                {activeTab === 'items' && (
                  <ItemsView
                    handItems={handItems}
                    stashItems={stashItems}
                    pinnedItems={pinnedItems}
                    expandedItems={expandedItems}
                    onTogglePin={togglePin}
                    onToggleExpand={toggleExpand}
                    renderItemCard={renderUnifiedItemCard}
                    getSortedItems={getSortedItems}
                  />
                )}

                {activeTab === 'search' && (
                  <div className="card-list">
                    {searchResults.slice(0, visibleCount).map((item) =>
                      renderUnifiedItemCard(
                        item,
                        pinnedItems.has(item.instance_id || item.uuid),
                        (e) => togglePin(item.instance_id || item.uuid, e),
                      ),
                    )}
                    {searchResults.length === 0 && <div className="empty-tip">未找到结果</div>}
                    {searchResults.length > visibleCount && (
                      <div style={{ textAlign: 'center', padding: '20px', color: '#888' }}>
                        显示 {visibleCount} / {searchResults.length} 项，向下滚动加载更多...
                      </div>
                    )}
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
