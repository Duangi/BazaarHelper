import React, { useEffect, useMemo, useState } from 'react';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { MatchHistoryRecord } from '../types';
import { getImg } from '../utils/helpers';
import './HistoryView.css';

interface HistoryViewProps {
  records: MatchHistoryRecord[];
  isLoading: boolean;
  onReload: () => void;
  showToast: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
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

const formatDate = (raw?: string) => {
  if (!raw) return '--';
  const trimmed = raw.trim();
  if (/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) {
    return trimmed;
  }
  if (trimmed.includes(' ')) {
    return trimmed.split(' ')[0];
  }
  return trimmed;
};

export const HistoryView: React.FC<HistoryViewProps> = ({ records, isLoading, onReload, showToast }) => {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [previewImage, setPreviewImage] = useState<string | null>(null);
  const [battleSortDesc, setBattleSortDesc] = useState(true);
  const [captureDelayMs, setCaptureDelayMs] = useState(0);
  const [lineupThumbs, setLineupThumbs] = useState<Record<string, string>>({});
  const [analyzingBattles, setAnalyzingBattles] = useState<Set<string>>(new Set());
  const [capturingBattles, setCapturingBattles] = useState<Set<string>>(new Set());
  const [screenshotVersions, setScreenshotVersions] = useState<Record<string, number>>({});
  const [analyzeProgress, setAnalyzeProgress] = useState<Record<string, { phase: string; done: number; total: number }>>({});

  const battleKeyOf = (record: MatchHistoryRecord, battle: { day: number; start_time?: string }) =>
    `${record.match_id}::${battle.day}::${battle.start_time || ''}`;

  const screenshotSrcOf = (screenshotPath: string, battleKey: string) => {
    const base = convertFileSrc(screenshotPath);
    const version = screenshotVersions[battleKey] || 0;
    return `${base}?v=${version}`;
  };

  useEffect(() => {
    let disposed = false;
    const unlisten = listen<{
      match_id?: string;
      battle_day?: number;
      battle_start_time?: string;
      phase?: string;
      done?: number;
      total?: number;
    }>('manual-lineup-progress', (event) => {
      if (disposed) return;
      const payload = event.payload || {};
      const matchId = `${payload.match_id || ''}`.trim();
      const day = Number(payload.battle_day || 0);
      const start = `${payload.battle_start_time || ''}`.trim();
      if (!matchId || !day || !start) return;
      const key = `${matchId}::${day}::${start}`;
      setAnalyzeProgress((prev) => ({
        ...prev,
        [key]: {
          phase: `${payload.phase || 'matching'}`,
          done: Math.max(0, Number(payload.done || 0)),
          total: Math.max(0, Number(payload.total || 0)),
        },
      }));
    });

    return () => {
      disposed = true;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  const normalizeItemSize = (raw?: string | null) => {
    const token = (raw || '').split(' / ')[0].trim().toLowerCase();
    if (token === 'small' || token === 'medium' || token === 'large') return token;
    return 'medium';
  };

  useEffect(() => {
    const uniquePaths = new Set<string>();
    records.forEach((record) => {
      (record.pvp_battles || []).forEach((battle) => {
        (battle.lineup_cards || []).forEach((card) => {
          if (card?.template_id) {
            uniquePaths.add(`images/${card.template_id}.webp`);
          }
        });
        (battle.enemy_lineup_cards || []).forEach((card) => {
          if (card?.template_id) {
            uniquePaths.add(`images/${card.template_id}.webp`);
          }
        });
      });
    });

    const targets = [...uniquePaths];
    if (targets.length === 0) return;

    const missing = targets.filter((path) => !lineupThumbs[path]);
    if (missing.length === 0) return;

    let cancelled = false;
    (async () => {
      const next: Record<string, string> = {};
      try {
        const mapped = await invoke<Record<string, string>>('get_search_thumbnail_paths', {
          resourcePaths: missing,
        });
        missing.forEach((resourcePath) => {
          const thumbPath = mapped?.[resourcePath];
          if (thumbPath) {
            next[resourcePath] = convertFileSrc(thumbPath);
          }
        });
      } catch {
        // fallback below
      }

      for (const resourcePath of missing) {
        if (next[resourcePath]) continue;
        try {
          const fallback = await getImg(resourcePath);
          if (fallback) next[resourcePath] = fallback;
        } catch {
          // ignore fallback failure
        }
      }

      if (cancelled || Object.keys(next).length === 0) return;
      setLineupThumbs((prev) => ({ ...prev, ...next }));
    })();

    return () => {
      cancelled = true;
    };
  }, [records, lineupThumbs]);

  useEffect(() => {
    let active = true;
    invoke<number>('get_screenshot_capture_delay_ms')
      .then((delay) => {
        if (active) setCaptureDelayMs(Math.max(0, Math.min(3000, Number(delay) || 0)));
      })
      .catch((error) => {
        console.warn('[History] get_screenshot_capture_delay_ms failed:', error);
      });
    return () => {
      active = false;
    };
  }, []);

  const ongoingRecords = useMemo(
    () => records.filter((record) => !record.is_finished),
    [records],
  );
  const finishedRecords = useMemo(
    () => records.filter((record) => record.is_finished),
    [records],
  );
  const latestGameDate = useMemo(() => {
    const dates = records
      .map((record) => formatDate(record.game_date))
      .filter((date) => /^\d{4}-\d{2}-\d{2}$/.test(date));
    if (dates.length === 0) return null;
    return dates.reduce((max, curr) => (curr > max ? curr : max));
  }, [records]);
  const renderRecordCard = (record: MatchHistoryRecord, idx: number, total: number) => {
    const wins = (record.pvp_battles || []).filter((b) => b.victory).length;
    const losses = (record.pvp_battles || []).length - wins;
    const sortedBattles = [...(record.pvp_battles || [])].sort((a, b) =>
      battleSortDesc ? b.day - a.day : a.day - b.day,
    );
    const opened = expanded.has(record.match_id);
    const heroAvatar = getHeroAvatar(record.hero);
    const statusText = record.victory ? '胜利' : record.is_finished ? '失败' : '进行中';
    const latestBattleDay = sortedBattles.reduce((max, b) => Math.max(max, Number(b.day) || 0), 0);
    const isLatestDateRecord = !!latestGameDate && formatDate(record.game_date) === latestGameDate;

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
                {statusText} ({wins}胜) · #{total - idx}
              </div>
              <div className="history-sub">
                {record.hero || 'Unknown'} · {formatDate(record.game_date)} {formatTime(record.start_time)} · Day {record.days}
              </div>
            </div>
          </div>
          <div className="history-card-right">
            <span className="history-score">{wins} 胜 - {losses} 负</span>
            <div className="history-battle-flow">
              {sortedBattles.slice(0, 15).map((battle, battleIdx) => (
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
            {sortedBattles.length === 0 && <div className="history-empty-row">无小局记录</div>}
            {sortedBattles.map((battle, battleIdx) => {
              const battleKey = battleKeyOf(record, battle);
              return (
              <div key={`${record.match_id}-${battleIdx}`} className="history-round-block">
                {(() => {
                  const isAnalyzing = analyzingBattles.has(battleKey);
                  const isCapturing = capturingBattles.has(battleKey);
                  const hasScreenshot = Boolean(battle.screenshot && `${battle.screenshot}`.trim().length > 0);
                  const canManualRecapture = !record.is_finished && isLatestDateRecord && battle.day === latestBattleDay;
                  const progress = analyzeProgress[battleKey];
                  const progressPercent = (() => {
                    if (!isAnalyzing) return 0;
                    if (!progress) return 12;
                    if (progress.phase === 'yolo') {
                      const p = progress.total > 0 ? progress.done / progress.total : 0;
                      return Math.max(8, Math.min(20, Math.round(p * 20)));
                    }
                    if (progress.phase === 'matching') {
                      if (progress.total <= 0) return 40;
                      const p = progress.done / progress.total;
                      return Math.max(20, Math.min(96, Math.round(20 + p * 76)));
                    }
                    if (progress.phase === 'done') return 100;
                    return 12;
                  })();

                  return (
                <div className="history-round-row">
                  <span className="history-round-day">DAY {battle.day}</span>
                  <span className={`history-round-result ${battle.victory ? 'win' : 'loss'}`}>
                    {battle.victory ? '胜利' : '失败'}
                  </span>
                  <span className="history-round-time">{formatTime(battle.start_time)}</span>
                  <span className="history-round-duration">
                    {battle.duration ? `${battle.duration.toFixed(1)}s` : '--'}
                  </span>
                  <div className="history-round-actions">
                    {canManualRecapture ? (
                      <button
                        className="history-round-capture"
                        disabled={isCapturing}
                        title="仅重新截取《The Bazaar》游戏窗口，不会截全屏"
                        onClick={async (event) => {
                          event.stopPropagation();
                          if (isCapturing) return;
                          if (!record.start_time || !battle.start_time) {
                            showToast('缺少对局时间信息，无法重新截图', 'warning');
                            return;
                          }

                          setCapturingBattles((prev) => {
                            const next = new Set(prev);
                            next.add(battleKey);
                            return next;
                          });

                          try {
                            await invoke<{ screenshot_path: string }>('capture_battle_screenshot_manual', {
                              req: {
                                match_start_time: record.start_time,
                                battle_day: battle.day,
                                battle_start_time: battle.start_time,
                                victory: battle.victory,
                                duration: battle.duration ?? null,
                              },
                            });
                            setScreenshotVersions((prev) => ({
                              ...prev,
                              [battleKey]: (prev[battleKey] || 0) + 1,
                            }));
                            showToast('已重新截图（仅游戏窗口）', 'success');
                            onReload();
                          } catch (error) {
                            const message = typeof error === 'string' ? error : (error as any)?.toString?.() || '未知错误';
                            showToast(`重新截图失败：${message}`, 'error');
                          } finally {
                            setCapturingBattles((prev) => {
                              const next = new Set(prev);
                              next.delete(battleKey);
                              return next;
                            });
                          }
                        }}
                      >
                        {isCapturing ? '截图中...' : '重新截图'}
                      </button>
                    ) : null}

                    <button
                      className="history-round-analyze"
                      disabled={!hasScreenshot || isAnalyzing}
                      title={hasScreenshot ? '手动分析这场战斗截图中的卡牌' : '无截图可分析'}
                      onClick={async (event) => {
                        event.stopPropagation();
                        if (!hasScreenshot || isAnalyzing) return;
                        const screenshotPath = `${battle.screenshot || ''}`.trim();
                        if (!screenshotPath) {
                          showToast('该小局没有可分析的截图', 'warning');
                          return;
                        }
                        if (!record.start_time || !battle.start_time) {
                          showToast('缺少对局时间信息，无法分析该小局', 'warning');
                          return;
                        }

                        setAnalyzingBattles((prev) => {
                          const next = new Set(prev);
                          next.add(battleKey);
                          return next;
                        });

                        try {
                          const result = await invoke<{ self_count?: number; enemy_count?: number }>('analyze_battle_lineup_from_screenshot', {
                            req: {
                              match_id: record.match_id,
                              match_start_time: record.start_time,
                              battle_day: battle.day,
                              battle_start_time: battle.start_time,
                              victory: battle.victory,
                              duration: battle.duration ?? null,
                              screenshot_path: screenshotPath,
                            },
                          });
                          const selfCount = Number(result?.self_count || 0);
                          const enemyCount = Number(result?.enemy_count || 0);
                          if (selfCount + enemyCount > 0) {
                            showToast(`分析完成：我方${selfCount}张，对方${enemyCount}张`, 'success');
                          } else {
                            showToast('分析完成，但未识别到可用卡牌', 'warning');
                          }
                        } catch (error) {
                          console.warn('[History] analyze_battle_lineup_from_screenshot failed:', error);
                          const message = typeof error === 'string' ? error : (error as any)?.toString?.() || '未知错误';
                          showToast(`分析失败：${message}`, 'error');
                        } finally {
                          setAnalyzingBattles((prev) => {
                            const next = new Set(prev);
                            next.delete(battleKey);
                            return next;
                          });
                          setTimeout(() => {
                            setAnalyzeProgress((prev) => {
                              const next = { ...prev };
                              delete next[battleKey];
                              return next;
                            });
                          }, 500);
                        }
                      }}
                    >
                      {isAnalyzing ? (
                        <>
                          <span
                            className="history-progress-ring"
                            style={{
                              background: `conic-gradient(rgba(212,175,55,0.95) ${progressPercent}%, rgba(212,175,55,0.2) ${progressPercent}% 100%)`,
                            }}
                          >
                            <span className="history-progress-hole" />
                          </span>
                          分析中 {progressPercent}%
                        </>
                      ) : hasScreenshot ? (
                        '分析卡牌'
                      ) : (
                        '无截图'
                      )}
                    </button>
                  </div>
                </div>
                  );
                })()}
                {battle.screenshot ? (
                  <div className="history-round-shot-row">
                    <button
                      className="history-round-shot"
                      title="点击查看战斗截图"
                      onClick={(event) => {
                        event.stopPropagation();
                        setPreviewImage(screenshotSrcOf(battle.screenshot as string, battleKey));
                      }}
                    >
                      <img src={screenshotSrcOf(battle.screenshot, battleKey)} alt={`Round ${battleIdx + 1} screenshot`} loading="lazy" />
                    </button>
                  </div>
                ) : null}
                {((battle.lineup_cards || []).length > 0 || (battle.enemy_lineup_cards || []).length > 0) ? (
                  <>
                    {(battle.enemy_lineup_cards || []).length > 0 ? (
                      <div className="history-lineup-row">
                        <span className="history-lineup-label">对方:</span>
                        <div className="history-lineup-cards">
                          {(battle.enemy_lineup_cards || []).map((card, cardIdx) => (
                            <div key={`${record.match_id}-${battleIdx}-enemy-${card.instance_id || card.template_id}-${cardIdx}`} className="history-lineup-item">
                              <div className={`image-box size-${normalizeItemSize(card.size)}`}>
                                {lineupThumbs[`images/${card.template_id}.webp`] ? (
                                  <img
                                    src={lineupThumbs[`images/${card.template_id}.webp`]}
                                    alt={card.name_cn || card.name_en || card.template_id}
                                    loading="lazy"
                                  />
                                ) : (
                                  <div className="history-lineup-placeholder" />
                                )}
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : null}

                    <div className="history-lineup-row">
                      <span className="history-lineup-label">我方:</span>
                      <div className="history-lineup-cards">
                        {(battle.lineup_cards || []).map((card, cardIdx) => (
                          <div key={`${record.match_id}-${battleIdx}-self-${card.instance_id || card.template_id}-${cardIdx}`} className="history-lineup-item">
                            <div className={`image-box size-${normalizeItemSize(card.size)}`}>
                              {lineupThumbs[`images/${card.template_id}.webp`] ? (
                                <img
                                  src={lineupThumbs[`images/${card.template_id}.webp`]}
                                  alt={card.name_cn || card.name_en || card.template_id}
                                  loading="lazy"
                                />
                              ) : (
                                <div className="history-lineup-placeholder" />
                              )}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  </>
                ) : null}
              </div>
              );
            })}
          </div>
        )}
      </div>
    );
  };

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
        <div className="history-actions">
          <div className="history-delay-wrap" title="回合结束后延迟截图，避免战后动画遮挡">
            <span className="history-delay-label">延迟截图 {((captureDelayMs || 0) / 1000).toFixed(1)}s</span>
            <input
              className="history-delay-slider"
              type="range"
              min={0}
              max={3000}
              step={100}
              value={captureDelayMs}
              onChange={async (event) => {
                const raw = Number(event.target.value);
                const value = Math.max(0, Math.min(3000, Number.isFinite(raw) ? raw : 0));
                setCaptureDelayMs(value);
                try {
                  const saved = await invoke<number>('set_screenshot_capture_delay_ms', { delayMs: value });
                  setCaptureDelayMs(Math.max(0, Math.min(3000, Number(saved) || value)));
                } catch (error) {
                  console.warn('[History] set_screenshot_capture_delay_ms failed:', error);
                }
              }}
            />
          </div>
          <button className="bulk-btn" onClick={() => setBattleSortDesc((prev) => !prev)}>
            {battleSortDesc ? '排序: 新→旧' : '排序: 旧→新'}
          </button>
          <button className="bulk-btn" onClick={onReload}>
            {isLoading ? '刷新中...' : '刷新'}
          </button>
        </div>
      </div>

      {records.length === 0 && !isLoading && (
        <div className="empty-tip" style={{ marginTop: 24 }}>
          暂无历史战绩，请先进行游戏并生成日志
        </div>
      )}

      <div className="history-list">
        {ongoingRecords.length > 0 && (
          <div className="history-section-divider">
            <span>当前进行中</span>
          </div>
        )}
        {ongoingRecords.map((record, idx) => renderRecordCard(record, idx, ongoingRecords.length))}

        {ongoingRecords.length > 0 && finishedRecords.length > 0 && (
          <div className="history-section-divider muted">
            <span>历史对局</span>
          </div>
        )}
        {finishedRecords.map((record, idx) => renderRecordCard(record, idx, finishedRecords.length))}
      </div>

      {previewImage ? (
        <div className="history-image-modal" onClick={() => setPreviewImage(null)}>
          <div className="history-image-modal-inner" onClick={(event) => event.stopPropagation()}>
            <button className="history-image-modal-close" onClick={() => setPreviewImage(null)}>
              ×
            </button>
            <img src={previewImage} alt="Battle Screenshot" />
          </div>
        </div>
      ) : null}
    </div>
  );
};
