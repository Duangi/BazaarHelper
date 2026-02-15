import React, { useMemo, useState } from 'react';
import type { MatchHistoryRecord } from '../types';
import './HistoryView.css';

interface HistoryViewProps {
  records: MatchHistoryRecord[];
  isLoading: boolean;
  onReload: () => void;
}

const HERO_AVATAR_MAP: Record<string, string> = {
  pygmalien: '/images/heroes/pygmalien.webp',
  jules: '/images/heroes/jules.webp',
  vanessa: '/images/heroes/vanessa.webp',
  mak: '/images/heroes/mak.webp',
  dooley: '/images/heroes/dooley.webp',
  stelle: '/images/heroes/stelle.webp',
};

const getHeroAvatar = (hero?: string) => {
  if (!hero) return undefined;
  return HERO_AVATAR_MAP[hero.trim().toLowerCase()];
};

const formatTime = (raw?: string) => {
  if (!raw) return '--';
  if (raw.includes(' ')) {
    const parts = raw.split(' ');
    if (parts.length >= 2) {
      return parts[1].slice(0, 5);
    }
  }
  return raw.slice(0, 5);
};

export const HistoryView: React.FC<HistoryViewProps> = ({ records, isLoading, onReload }) => {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const overall = useMemo(() => {
    let wins = 0;
    let losses = 0;

    for (const match of records) {
      for (const battle of match.pvp_battles || []) {
        if (battle.victory) wins += 1;
        else losses += 1;
      }
    }

    return { wins, losses, total: wins + losses };
  }, [records]);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="history-view">
      <div className="history-header">
        <div>
          <h2>历史战绩</h2>
          <div className="history-summary">
            小局战绩: {overall.wins} 胜 {overall.losses} 负 ({overall.total} 场)
          </div>
        </div>
        <button className="bulk-btn" onClick={onReload}>
          {isLoading ? '刷新中...' : '刷新'}
        </button>
      </div>

      {records.length === 0 && !isLoading && (
        <div className="empty-tip" style={{ marginTop: 24 }}>
          暂无历史战绩，请先进行游戏并生成日志
        </div>
      )}

      <div className="history-list">
        {records.map((record, idx) => {
          const wins = (record.pvp_battles || []).filter((b) => b.victory).length;
          const losses = (record.pvp_battles || []).length - wins;
          const opened = expanded.has(record.match_id);
          const heroAvatar = getHeroAvatar(record.hero);
          const statusText = record.victory ? '胜利' : record.is_finished ? '失败' : '进行中';

          return (
            <div key={record.match_id || `${idx}`} className={`history-card ${opened ? 'expanded' : ''}`}>
              <button className="history-card-head" onClick={() => toggle(record.match_id)}>
                <div className="history-card-left">
                  <div className="history-hero-avatar">
                    {heroAvatar ? (
                      <img src={heroAvatar} alt={record.hero || 'Unknown'} />
                    ) : (
                      <span>{(record.hero || '?').slice(0, 1).toUpperCase()}</span>
                    )}
                  </div>
                  <div className="history-title-block">
                    <div className="history-title">
                      {statusText} ({wins}胜) · #{records.length - idx}
                    </div>
                    <div className="history-sub">
                      {record.hero || 'Unknown'} · {formatTime(record.start_time)} · Day {record.days}
                    </div>
                  </div>
                </div>
                <div className="history-card-right">
                  <span className="history-score">{wins} 胜 - {losses} 负</span>
                  <div className="history-battle-flow">
                    {(record.pvp_battles || []).slice(0, 15).map((battle, battleIdx) => (
                      <span
                        key={`${record.match_id}-flow-${battleIdx}`}
                        className={`history-flow-dot ${battle.victory ? 'win' : 'loss'}`}
                        title={`Day ${battle.day} ${battle.victory ? '胜利' : '失败'}`}
                      >
                        {battle.victory ? '✓' : '✗'}
                      </span>
                    ))}
                    {(record.pvp_battles || []).length > 15 && (
                      <span className="history-flow-more">+{(record.pvp_battles || []).length - 15}</span>
                    )}
                  </div>
                  <span className="history-arrow">{opened ? '▴' : '▾'}</span>
                </div>
              </button>

              {opened && (
                <div className="history-card-body">
                  {(record.pvp_battles || []).length === 0 && <div className="history-empty-row">无小局记录</div>}
                  {(record.pvp_battles || []).map((battle, battleIdx) => (
                    <div key={`${record.match_id}-${battleIdx}`} className="history-round-row">
                      <span className="history-round-day">ROUND {battleIdx + 1}</span>
                      <span className={`history-round-result ${battle.victory ? 'win' : 'loss'}`}>
                        {battle.victory ? '胜利' : '失败'}
                      </span>
                      <span className="history-round-time">Day {battle.day}</span>
                      <span className="history-round-duration">
                        {battle.duration ? `${battle.duration.toFixed(1)}s` : formatTime(battle.start_time)}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
