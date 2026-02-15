import React from 'react';
import { invoke } from '@tauri-apps/api/core';

interface SearchQueryState {
  keyword: string;
  item_type: string;
  size: string;
  start_tier: string;
  hero: string;
  tags: string;
  hidden_tags: string;
}

interface SearchFiltersPanelProps {
  isSearchFilterCollapsed: boolean;
  setIsSearchFilterCollapsed: (value: boolean) => void;
  matchMode: 'all' | 'any';
  setMatchMode: (value: 'all' | 'any') => void;
  searchFilterHeight: number;
  searchQuery: SearchQueryState;
  setSearchQuery: React.Dispatch<React.SetStateAction<SearchQueryState>>;
  setIsInputFocused: (value: boolean) => void;
  lastItemSize: string;
  setLastItemSize: (value: string) => void;
  selectedTags: string[];
  setSelectedTags: (value: string[]) => void;
  selectedHiddenTags: string[];
  setSelectedHiddenTags: (value: string[]) => void;
  hiddenTagIcons: Record<string, string>;
  isSearching: boolean;
  searchResultsCount: number;
  setResizeStartY: (value: number) => void;
  setResizeStartHeight: (value: number) => void;
  setIsResizingFilter: (value: boolean) => void;
  isResizingFilter: boolean;
}

export const SearchFiltersPanel: React.FC<SearchFiltersPanelProps> = ({
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
  searchResultsCount,
  setResizeStartY,
  setResizeStartHeight,
  setIsResizingFilter,
  isResizingFilter,
}) => {
  return (
    <div className="search-box-container" data-no-drag style={{
      zIndex: 10,
      borderBottom: '1px solid rgba(255,255,255,0.1)',
      background: '#2b2621',
      boxShadow: '0 4px 6px rgba(0,0,0,0.3)',
      display: 'flex',
      flexDirection: 'column',
      height: isSearchFilterCollapsed ? 'auto' : `${searchFilterHeight}px`,
      position: 'relative',
    }}>
      <div data-no-drag style={{
        padding: '12px',
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
        overflowY: 'auto',
        flex: 1,
        scrollbarWidth: 'thin',
        scrollbarColor: '#ffcd19 rgba(0,0,0,0.3)',
      }} className="custom-scrollbar">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <div style={{ fontSize: '12px', color: '#ffcd19', fontWeight: 'bold' }}>搜索过滤器</div>
            <div style={{ display: 'flex', gap: '4px' }}>
              <button
                onClick={() => setMatchMode('all')}
                className={`toggle-btn ${matchMode === 'all' ? 'active' : ''}`}
                style={{
                  padding: '4px 8px',
                  fontSize: '11px',
                  borderRadius: '4px',
                  background: matchMode === 'all' ? '#ffcd19' : 'transparent',
                  color: matchMode === 'all' ? '#1e1b18' : '#ffcd19',
                  border: '1px solid #ffcd19',
                  cursor: 'pointer',
                }}
                title="所有筛选项必须同时满足"
              >
                匹配所有
              </button>
              <button
                onClick={() => setMatchMode('any')}
                className={`toggle-btn ${matchMode === 'any' ? 'active' : ''}`}
                style={{
                  padding: '4px 8px',
                  fontSize: '11px',
                  borderRadius: '4px',
                  background: matchMode === 'any' ? '#ffcd19' : 'transparent',
                  color: matchMode === 'any' ? '#1e1b18' : '#ffcd19',
                  border: '1px solid #ffcd19',
                  cursor: 'pointer',
                }}
                title="满足任意一个筛选项即可"
              >
                匹配任一
              </button>
            </div>
          </div>
          <button
            onClick={() => setIsSearchFilterCollapsed(!isSearchFilterCollapsed)}
            style={{
              background: 'transparent',
              border: '1px solid rgba(255,205,25,0.3)',
              color: '#ffcd19',
              padding: '4px 8px',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '11px',
            }}
          >
            {isSearchFilterCollapsed ? '展开 ▼' : '收起 ▲'}
          </button>
        </div>

        {!isSearchFilterCollapsed && (
          <>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              <input
                className="search-input"
                placeholder="搜索名称 / 描述..."
                value={searchQuery.keyword}
                onChange={(e) => setSearchQuery({ ...searchQuery, keyword: e.target.value })}
                onFocus={() => {
                  setIsInputFocused(true);
                  invoke('set_overlay_ignore_cursor', { ignore: false }).catch(() => {});
                }}
                onBlur={() => {
                  setIsInputFocused(false);
                }}
                style={{
                  flex: 1,
                  minWidth: '200px',
                  background: '#1e1b18',
                  border: '1px solid #48413a',
                  color: '#eee',
                  padding: '8px 12px',
                  borderRadius: '4px',
                  fontSize: '14px',
                }}
              />
            </div>

            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                {[
                  { val: 'item', label: '物品' },
                  { val: 'skill', label: '技能' },
                ].map((opt) => (
                  <button
                    key={opt.val}
                    className={`toggle-btn ${searchQuery.item_type === opt.val ? 'active' : ''}`}
                    onClick={() => {
                      if (searchQuery.item_type === opt.val) {
                        setSearchQuery({ ...searchQuery, item_type: 'all', size: opt.val === 'skill' ? lastItemSize : searchQuery.size });
                      } else if (opt.val === 'skill') {
                        setLastItemSize(searchQuery.size);
                        setSearchQuery({ ...searchQuery, item_type: opt.val, size: '' });
                      } else {
                        const restoredSize = searchQuery.item_type === 'skill' ? lastItemSize : searchQuery.size;
                        setSearchQuery({ ...searchQuery, item_type: opt.val, size: restoredSize });
                      }
                    }}
                    style={{ padding: '6px 10px', borderRadius: 6 }}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>

              {searchQuery.item_type !== 'skill' && (
                <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                  {[
                    { val: 'small', label: '小' },
                    { val: 'medium', label: '中' },
                    { val: 'large', label: '大' },
                  ].map((opt) => (
                    <button
                      key={opt.val}
                      className={`toggle-btn ${searchQuery.size === opt.val ? 'active' : ''}`}
                      onClick={() => setSearchQuery({ ...searchQuery, size: searchQuery.size === opt.val ? '' : opt.val })}
                      style={{ padding: '6px 10px', borderRadius: 6 }}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                {[
                  { val: 'bronze', label: '青铜', color: '#cd7f32' },
                  { val: 'silver', label: '白银', color: '#c0c0c0' },
                  { val: 'gold', label: '黄金', color: '#ffd700' },
                  { val: 'diamond', label: '钻石', color: '#b9f2ff' },
                  { val: 'legendary', label: '传说', color: '#ff4500' },
                ].map((opt) => (
                  <button
                    key={opt.val}
                    className={`toggle-btn ${searchQuery.start_tier === opt.val ? 'active' : ''}`}
                    onClick={() => setSearchQuery({ ...searchQuery, start_tier: searchQuery.start_tier === opt.val ? '' : opt.val })}
                    style={{ padding: '6px 10px', borderRadius: 6, color: opt.color }}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>

              <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                {[
                  { val: 'Common', label: '通用', avatar: '' },
                  { val: 'Pygmalien', label: '猪', avatar: '/images/heroes/pygmalien.webp' },
                  { val: 'Jules', label: '朱尔斯', avatar: '/images/heroes/jules.webp' },
                  { val: 'Vanessa', label: '瓦内莎', avatar: '/images/heroes/vanessa.webp' },
                  { val: 'Mak', label: '马克', avatar: '/images/heroes/mak.webp' },
                  { val: 'Dooley', label: '多利', avatar: '/images/heroes/dooley.webp' },
                  { val: 'Stelle', label: '斯黛尔', avatar: '/images/heroes/stelle.webp' },
                ].map((opt) => (
                  <button
                    key={opt.val}
                    className={`toggle-btn ${opt.avatar ? 'hero-btn' : ''} ${searchQuery.hero === opt.val ? 'active' : ''}`}
                    onClick={() => setSearchQuery({ ...searchQuery, hero: searchQuery.hero === opt.val ? '' : opt.val })}
                    title={opt.label}
                  >
                    {opt.avatar ? <img src={opt.avatar} alt={opt.label} /> : opt.label}
                  </button>
                ))}
              </div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <div style={{ fontSize: '11px', color: '#888' }}>标签 (可多选)</div>
              <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
                {[
                  ['Drone', '无人机'],
                  ['Property', '地产'],
                  ['Ray', '射线'],
                  ['Tool', '工具'],
                  ['Dinosaur', '恐龙'],
                  ['Loot', '战利品'],
                  ['Apparel', '服饰'],
                  ['Core', '核心'],
                  ['Weapon', '武器'],
                  ['Aquatic', '水系'],
                  ['Toy', '玩具'],
                  ['Tech', '科技'],
                  ['Potion', '药水'],
                  ['Reagent', '原料'],
                  ['Vehicle', '载具'],
                  ['Relic', '遗物'],
                  ['Food', '食物'],
                  ['Dragon', '龙'],
                  ['Friend', '伙伴'],
                ].sort((a, b) => a[1].localeCompare(b[1], 'zh-CN')).map(([val, label]) => (
                  <button
                    key={val}
                    className={`toggle-btn ${selectedTags.includes(val) ? 'active' : ''}`}
                    onClick={() => {
                      if (selectedTags.includes(val)) {
                        setSelectedTags(selectedTags.filter((t) => t !== val));
                      } else {
                        setSelectedTags([...selectedTags, val]);
                      }
                    }}
                    style={{ padding: '6px 10px', borderRadius: 6, fontSize: '12px' }}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <div style={{ fontSize: '11px', color: '#888' }}>隐藏标签 (可多选)</div>
              <div style={{ display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
                {(() => {
                  const tagGroups = [
                    { tags: [['Ammo', '弹药'], ['AmmoRef', '弹药相关']], icon: 'Ammo', color: 'var(--c-ammo)' },
                    { tags: [['Burn', '灼烧'], ['BurnRef', '灼烧相关']], icon: 'Burn', color: 'var(--c-burn)' },
                    { tags: [['Charge', '充能']], icon: 'Charge', color: 'var(--c-charge)' },
                    { tags: [['Cooldown', '冷却'], ['CooldownReference', '冷却相关']], icon: 'Cooldown', color: 'var(--c-cooldown)' },
                    { tags: [['Crit', '暴击'], ['CritRef', '暴击相关']], icon: 'CritChance', color: 'var(--c-crit)' },
                    { tags: [['Damage', '伤害'], ['DamageRef', '伤害相关']], icon: 'Damage', color: 'var(--c-damage)' },
                    { tags: [['EconomyRef', '经济相关'], ['Gold', '金币']], icon: 'Income', color: 'var(--c-golden)' },
                    { tags: [['Flying', '飞行'], ['FlyingRef', '飞行相关']], icon: 'Flying', color: 'var(--c-fly)' },
                    { tags: [['Freeze', '冻结'], ['FreezeRef', '冻结相关']], icon: 'Freeze', color: 'var(--c-freeze)' },
                    { tags: [['Haste', '加速'], ['HasteRef', '加速相关']], icon: 'Haste', color: 'var(--c-haste)' },
                    { tags: [['Heal', '治疗'], ['HealRef', '治疗相关']], icon: 'Health', color: 'var(--c-heal)' },
                    { tags: [['Health', '生命值'], ['HealthRef', '生命值相关']], icon: 'MaxHPHeart', color: 'var(--c-heal)' },
                    { tags: [['Lifesteal', '生命偷取']], icon: 'Lifesteal', color: 'var(--c-lifesteal)' },
                    { tags: [['Poison', '剧毒'], ['PoisonRef', '剧毒相关']], icon: 'Poison', color: 'var(--c-poison)' },
                    { tags: [['Quest', '任务']], icon: null, color: '#9098fe' },
                    { tags: [['Regen', '再生'], ['RegenRef', '再生相关']], icon: 'Regen', color: 'var(--c-regen)' },
                    { tags: [['Shield', '护盾'], ['ShieldRef', '护盾相关']], icon: 'Shield', color: 'var(--c-shield)' },
                    { tags: [['Slow', '减速'], ['SlowRef', '减速相关']], icon: 'Slowness', color: 'var(--c-slow)' },
                  ];

                  return tagGroups.map((group, groupIndex) => (
                    <div key={groupIndex} style={{ display: 'flex', gap: '2px', alignItems: 'center' }}>
                      {group.tags.map(([val, label], index) => (
                        <button
                          key={val}
                          className={`toggle-btn ${selectedHiddenTags.includes(val) ? 'active' : ''}`}
                          onClick={() => {
                            if (selectedHiddenTags.includes(val)) {
                              setSelectedHiddenTags(selectedHiddenTags.filter((t) => t !== val));
                            } else {
                              setSelectedHiddenTags([...selectedHiddenTags, val]);
                            }
                          }}
                          style={{
                            padding: '6px 10px',
                            borderRadius: 6,
                            fontSize: '12px',
                            color: group.color,
                            display: 'flex',
                            alignItems: 'center',
                            gap: '4px',
                          }}
                        >
                          {index === 0 && group.icon && hiddenTagIcons[group.icon] && (
                            <img
                              src={hiddenTagIcons[group.icon]}
                              alt=""
                              style={{ width: '14px', height: '14px', display: 'inline-block' }}
                            />
                          )}
                          {label}
                        </button>
                      ))}
                    </div>
                  ));
                })()}
              </div>
            </div>
          </>
        )}
      </div>

      <div style={{
        padding: '8px 12px',
        borderTop: '1px solid rgba(255,255,255,0.05)',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        background: 'rgba(0,0,0,0.2)',
      }}>
        <div style={{ fontSize: '13px', color: '#a0937d' }}>
          {isSearching ? (
            <><span style={{ color: '#d4af37' }}>🔍</span> 搜索中...</>
          ) : (
            <>找到 <span style={{ color: '#ffcc00', fontWeight: 'bold' }}>{searchResultsCount}</span> 个结果</>
          )}
        </div>
        <button
          className="bulk-btn"
          style={{ fontSize: '11px', padding: '4px 8px' }}
          onClick={() => {
            setSearchQuery({ keyword: '', item_type: 'all', size: '', start_tier: '', hero: '', tags: '', hidden_tags: '' });
            setSelectedTags([]);
            setSelectedHiddenTags([]);
          }}
        >
          重置
        </button>
      </div>

      {!isSearchFilterCollapsed && (
        <div
          onMouseDown={(e) => {
            e.preventDefault();
            setResizeStartY(e.clientY);
            setResizeStartHeight(searchFilterHeight);
            setIsResizingFilter(true);
          }}
          style={{
            position: 'absolute',
            bottom: '0',
            left: '0',
            right: '0',
            height: '8px',
            cursor: 'ns-resize',
            background: 'linear-gradient(to bottom, transparent, rgba(255,205,25,0.1))',
            borderTop: '1px solid rgba(255,205,25,0.2)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'background 0.2s',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = 'linear-gradient(to bottom, transparent, rgba(255,205,25,0.2))';
          }}
          onMouseLeave={(e) => {
            if (!isResizingFilter) {
              e.currentTarget.style.background = 'linear-gradient(to bottom, transparent, rgba(255,205,25,0.1))';
            }
          }}
        >
          <div style={{
            width: '40px',
            height: '3px',
            borderRadius: '2px',
            background: 'rgba(255,205,25,0.4)',
          }} />
        </div>
      )}
    </div>
  );
};
