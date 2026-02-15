import React from 'react';
import type { MonsterData } from '../types';

interface LoadingState {
  loaded: number;
  total: number;
  is_complete: boolean;
  current_name: string;
}

interface MonsterTabViewProps {
  selectedDay: string;
  setSelectedDay: (day: string) => void;
  handleDayChange: (day: number) => void;
  isRecognizing: boolean;
  handleAutoRecognition: (day: number | null) => Promise<void>;
  showToast: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
  templateLoading: LoadingState;
  manualMonsters: MonsterData[];
  identifiedNames: string[];
  expandedMonsters: Set<string>;
  toggleMonsterExpand: (nameZh: string) => void;
  renderTierInfo: (item: any) => React.ReactNode;
}

export const MonsterTabView: React.FC<MonsterTabViewProps> = ({
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
}) => {
  return (
    <>
      <div className="monster-controls">
        <div className="day-tabs">
          <div className="day-row">
            {["Day 1", "Day 2", "Day 3", "Day 4", "Day 5"].map((d) => (
              <div
                key={d}
                className={`day-tab ${selectedDay === d ? 'active' : ''}`}
                onClick={() => {
                  setSelectedDay(d);
                  const dayNum = parseInt(d.split(' ')[1]);
                  handleDayChange(dayNum);
                }}
              >
                {d}
              </div>
            ))}
          </div>
          <div className="day-row">
            {["Day 6", "Day 7", "Day 8", "Day 9", "Day 10+"].map((d) => (
              <div
                key={d}
                className={`day-tab ${selectedDay === d ? 'active' : ''}`}
                onClick={() => {
                  setSelectedDay(d);
                  const dayNum = d === 'Day 10+' ? 10 : parseInt(d.split(' ')[1]);
                  handleDayChange(dayNum);
                }}
              >
                {d}
              </div>
            ))}
          </div>
        </div>

        <button
          className="bulk-btn"
          disabled={isRecognizing}
          style={{
            width: '100%',
            padding: '12px',
            marginTop: '12px',
            background: isRecognizing ? '#333' : 'linear-gradient(135deg, rgba(255, 205, 25, 0.2), rgba(255, 180, 25, 0.15))',
            border: '2px solid rgba(255, 205, 25, 0.5)',
            borderRadius: '8px',
            color: isRecognizing ? '#666' : '#ffcd19',
            fontSize: '16px',
            fontWeight: 'bold',
            cursor: isRecognizing ? 'not-allowed' : 'pointer',
            transition: 'all 0.2s',
            textShadow: '0 2px 4px rgba(0, 0, 0, 0.5)',
            boxShadow: '0 4px 12px rgba(255, 205, 25, 0.2)',
          }}
          onClick={async () => {
            try {
              const dayNum = selectedDay === 'Day 10+' ? 10 : parseInt(selectedDay.split(' ')[1]);
              showToast(`正在识别 Day ${dayNum} 的野怪...`, 'info');
              await handleAutoRecognition(dayNum);
              showToast('一键识别完成', 'success');
            } catch (err: any) {
              console.error('[Monster Recognition] Failed:', err);
              showToast('识别失败: ' + err, 'error');
            }
          }}
          onMouseEnter={(e) => {
            if (isRecognizing) return;
            e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 205, 25, 0.3), rgba(255, 180, 25, 0.2))';
            e.currentTarget.style.borderColor = 'rgba(255, 205, 25, 0.8)';
            e.currentTarget.style.boxShadow = '0 6px 16px rgba(255, 205, 25, 0.3)';
          }}
          onMouseLeave={(e) => {
            if (isRecognizing) return;
            e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 205, 25, 0.2), rgba(255, 180, 25, 0.15))';
            e.currentTarget.style.borderColor = 'rgba(255, 205, 25, 0.5)';
            e.currentTarget.style.boxShadow = '0 4px 12px rgba(255, 205, 25, 0.2)';
          }}
        >
          {isRecognizing ? '⏳ 正在识别中...' : '🎯 一键识别当前野怪'}
        </button>

        {!templateLoading.is_complete && templateLoading.total > 0 && (
          <div className="loading-progress">
            <div className="progress-text">加载怪物模板: {templateLoading.loaded}/{templateLoading.total}</div>
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${templateLoading.total > 0 ? (templateLoading.loaded / templateLoading.total * 100) : 0}%` }}
              />
            </div>
          </div>
        )}
      </div>

      <div className="monster-list-v2">
        {manualMonsters
          .sort((a, b) => {
            const aIdentified = identifiedNames.includes(a.name_zh);
            const bIdentified = identifiedNames.includes(b.name_zh);
            if (aIdentified && !bIdentified) return -1;
            if (!aIdentified && bIdentified) return 1;
            return 0;
          })
          .map((m, i) => {
            const isIdentified = identifiedNames.includes(m.name_zh);
            const isExpanded = expandedMonsters.has(m.name_zh);

            return (
              <div
                id={`monster-${m.name_zh}`}
                key={i}
                className={`monster-card-v2 ${isIdentified ? 'identified-glow' : ''} ${isExpanded ? 'expanded' : ''}`}
                onClick={() => toggleMonsterExpand(m.name_zh)}
              >
                <div className="monster-header-v2">
                  <div className="avatar-wrap">
                    <div className="monster-image-layers">
                      <img src={m.displayImgBg} className="monster-layer-bg" alt="" />
                      <img src={m.displayImg} className="monster-layer-char" alt="" />
                    </div>
                  </div>
                  <div className="monster-info-v2">
                    <div className="monster-name-zh">
                      {m.name_zh}
                      {isIdentified && <span className="id-badge">MATCH</span>}
                    </div>
                    <div className="monster-health">❤️ {m.health?.toString() || m.health}</div>
                  </div>
                  <div className="monster-available-tag">
                    {m.available}
                    <span className="expand-indicator" style={{ marginLeft: '8px' }}>{isExpanded ? '▴' : '▾'}</span>
                  </div>
                </div>

                {isExpanded && (
                  <div className="monster-assets-grid">
                    <div className="assets-section">
                      <div className="section-title">技能 (Skills)</div>
                      {m.skills?.map((s, idx) => <div key={idx}>{renderTierInfo(s)}</div>)}
                    </div>
                    <div className="assets-section">
                      <div className="section-title">物品 (Items)</div>
                      {m.items?.map((it, idx) => <div key={idx}>{renderTierInfo(it)}</div>)}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        {manualMonsters.length === 0 && <div className="empty-tip">该天数下暂无怪物数据</div>}
      </div>
    </>
  );
};
