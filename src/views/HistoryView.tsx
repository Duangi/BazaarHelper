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

interface GameIdentityInfo {
  username: string;
  account_id: string;
}

interface CheckUploadedResponse {
  existingMatchIds?: string[];
  existingBattleKeys?: string[];
}

interface MatchUploadProgress {
  done: number;
  total: number;
  uploaded: number;
  skipped: number;
  failed: number;
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

const DEFAULT_UPLOAD_API_BASE = 'https://www.duang.work';
const UPLOAD_API_BASE_KEY = 'community-upload-api-base';
const UPLOAD_PLUGIN_KEY = 'community-upload-plugin-key';

const normalizeApiBase = (raw: string) => {
  const trimmed = raw.trim();
  if (!trimmed) return '';
  const withProtocol = /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const url = new URL(withProtocol);
    if (url.hostname === 'duang.work') {
      url.hostname = 'www.duang.work';
    }
    return `${url.protocol}//${url.host}`;
  } catch {
    return withProtocol.replace(/\/+$/, '');
  }
};

const parseJsonSafe = async (response: Response) => {
  const text = await response.text();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
};

const loadImageFromBlob = (blob: Blob): Promise<HTMLImageElement> =>
  new Promise((resolve, reject) => {
    const blobUrl = URL.createObjectURL(blob);
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(blobUrl);
      resolve(img);
    };
    img.onerror = () => {
      URL.revokeObjectURL(blobUrl);
      reject(new Error('图片解码失败'));
    };
    img.src = blobUrl;
  });

const convertImageBlobToWebp = async (blob: Blob, quality = 0.8): Promise<Blob> => {
  if (!blob.type.startsWith('image/')) return blob;
  try {
    const img = await loadImageFromBlob(blob);
    const width = Math.max(1, img.naturalWidth || img.width || 1);
    const height = Math.max(1, img.naturalHeight || img.height || 1);
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d');
    if (!ctx) return blob;
    ctx.drawImage(img, 0, 0, width, height);
    const webpBlob = await new Promise<Blob | null>((resolve) => {
      canvas.toBlob((b) => resolve(b), 'image/webp', quality);
    });
    return webpBlob || blob;
  } catch {
    return blob;
  }
};

const sanitizeUploadFolderName = (raw: string): string => {
  const text = String(raw || '')
    .trim()
    .replace(/\s+/g, '_')
    .replace(/[^a-zA-Z0-9_\-\u4e00-\u9fa5]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 40);
  return text || 'anonymous';
};

const buildBattleKey = (matchId: string, day: number, startTime: string, result: 'win' | 'lose') =>
  `${matchId}::${day}::${startTime || ''}::${result}`;

const getDisplayDay = (record: MatchHistoryRecord): number => {
  const maxBattleDay = (record.pvp_battles || []).reduce((max, b) => Math.max(max, Number(b.day) || 0), 0);
  const fallbackDay = Math.max(1, Number(record.days) || 1);
  if (maxBattleDay <= 0) return fallbackDay;
  if (record.is_finished) return maxBattleDay;
  return Math.max(maxBattleDay + 1, fallbackDay);
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
  const [uploadApiBase, setUploadApiBase] = useState<string>(() => {
    if (typeof window === 'undefined') return DEFAULT_UPLOAD_API_BASE;
    const saved = localStorage.getItem(UPLOAD_API_BASE_KEY) || DEFAULT_UPLOAD_API_BASE;
    return normalizeApiBase(saved) || DEFAULT_UPLOAD_API_BASE;
  });
  const [uploadPluginKey, setUploadPluginKey] = useState<string>(() => {
    if (typeof window === 'undefined') return '';
    return (localStorage.getItem(UPLOAD_PLUGIN_KEY) || '').trim();
  });
  const [uploadingMatches, setUploadingMatches] = useState<Set<string>>(new Set());
  const [uploadProgressByMatch, setUploadProgressByMatch] = useState<Record<string, MatchUploadProgress>>({});
  const [bulkUploading, setBulkUploading] = useState(false);
  const [bulkProgress, setBulkProgress] = useState<{ done: number; total: number }>({ done: 0, total: 0 });
  const [identity, setIdentity] = useState<GameIdentityInfo | null>(null);

  const battleKeyOf = (record: MatchHistoryRecord, battle: { day: number; start_time?: string }) =>
    `${record.match_id}::${battle.day}::${battle.start_time || ''}`;

  const screenshotSrcOf = (screenshotPath: string, battleKey: string) => {
    const base = convertFileSrc(screenshotPath);
    const version = screenshotVersions[battleKey] || 0;
    return `${base}?v=${version}`;
  };

  useEffect(() => {
    if (typeof window === 'undefined') return;
    localStorage.setItem(UPLOAD_API_BASE_KEY, uploadApiBase);
  }, [uploadApiBase]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    localStorage.setItem(UPLOAD_PLUGIN_KEY, uploadPluginKey);
  }, [uploadPluginKey]);

  const ensureIdentity = async (): Promise<GameIdentityInfo> => {
    if (identity?.account_id && identity?.username) return identity;
    const info = await invoke<GameIdentityInfo>('get_game_identity');
    setIdentity(info);
    return info;
  };

  const buildAuthHeaders = () => {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    const token = uploadPluginKey.trim();
    if (token) headers['x-plugin-key'] = token;
    return headers;
  };

  const checkUploadedMatches = async (authorUserId: string, matchIds: string[]) => {
    const base = normalizeApiBase(uploadApiBase);
    if (!base) throw new Error('请先配置上传服务地址');
    try {
      const response = await fetch(`${base}/api/game-records/check`, {
        method: 'POST',
        headers: buildAuthHeaders(),
        body: JSON.stringify({ authorUserId, matchIds }),
      });
      const json = (await parseJsonSafe(response)) as CheckUploadedResponse | null;
      if (!response.ok) {
        throw new Error((json as any)?.error || `查重失败 (${response.status} ${response.statusText})`);
      }
      return {
        existingMatchIds: new Set((json?.existingMatchIds || []).map((x) => String(x || '').trim()).filter(Boolean)),
        existingBattleKeys: new Set((json?.existingBattleKeys || []).map((x) => String(x || '').trim()).filter(Boolean)),
      };
    } catch (err: any) {
      const msg = err?.message || '';
      if (msg.includes('Failed to fetch')) {
        throw new Error(`无法连接服务器 (${base})，请检查网络或地址配置`);
      }
      throw err;
    }
  };

  const uploadMatchBattles = async (
    record: MatchHistoryRecord,
    author: GameIdentityInfo,
    knownExistingBattleKeys?: Set<string>,
    onProgress?: (progress: MatchUploadProgress) => void,
  ): Promise<{ uploaded: number; skipped: number; failed: number }> => {
    const base = normalizeApiBase(uploadApiBase);
    if (!base) throw new Error('请先配置上传服务地址');

    const localBattles = (record.pvp_battles || [])
      .filter((battle) => Boolean(battle.screenshot && `${battle.screenshot}`.trim().length > 0))
      .map((battle) => ({
        battle,
        key: buildBattleKey(
          record.match_id,
          Number(battle.day || 0),
          String(battle.start_time || ''),
          battle.victory ? 'win' : 'lose',
        ),
      }));

    const existingKeys = knownExistingBattleKeys || new Set<string>();
    let uploaded = 0;
    let skipped = 0;
    let failed = 0;
    let done = 0;
    const total = localBattles.length;

    const emitProgress = () => {
      if (!onProgress) return;
      onProgress({ done, total, uploaded, skipped, failed });
    };

    emitProgress();

    for (const entry of localBattles) {
      if (existingKeys.has(entry.key)) {
        skipped += 1;
        done += 1;
        emitProgress();
        continue;
      }

      try {
        const screenshotPath = String(entry.battle.screenshot || '').trim();
        const localRes = await fetch(convertFileSrc(screenshotPath));
        if (!localRes.ok) throw new Error(`读取本地截图失败: ${localRes.status}`);
        const sourceBlob = await localRes.blob();
        const blob = await convertImageBlobToWebp(sourceBlob, 0.8);
        const safeStart = String(entry.battle.start_time || 'unknown').replace(/[^\d]/g, '').slice(0, 14) || 'unknown';
        const fileName = `${record.match_id}-d${entry.battle.day}-${entry.battle.victory ? 'win' : 'lose'}-${safeStart}.webp`;
        const authorFolder = sanitizeUploadFolderName(author.username || author.account_id || 'anonymous');

        const presignRes = await fetch(`${base}/api/r2/presign`, {
          method: 'POST',
          headers: buildAuthHeaders(),
          body: JSON.stringify({
            fileName,
            contentType: 'image/webp',
            folder: `match-records/${authorFolder}`,
          }),
        });
        const presignJson = await parseJsonSafe(presignRes);
        if (!presignRes.ok || !presignJson?.uploadUrl || !presignJson?.publicUrl) {
          throw new Error((presignJson as any)?.error || `获取上传签名失败 (${presignRes.status})`);
        }

        const putRes = await fetch(String(presignJson.uploadUrl), {
          method: 'PUT',
          headers: {
            'Content-Type': 'image/webp',
          },
          body: blob,
        });
        if (!putRes.ok) {
          throw new Error(`上传到 R2 失败 (${putRes.status})`);
        }

        const totalWins = (record.pvp_battles || []).filter((b) => b.victory).length;
        const totalLosses = Math.max(0, (record.pvp_battles || []).length - totalWins);

        const payload = {
          authorUserId: author.account_id,
          authorName: author.username,
          playedOn: record.game_date || new Date().toISOString().slice(0, 10),
          result: entry.battle.victory ? 'win' : 'lose',
          dayIndex: Number(entry.battle.day || 1),
          screenshotUrl: String(presignJson.publicUrl),
          note: '',
          meta: {
            match_id: record.match_id,
            hero: record.hero || '',
            start_time: record.start_time || '',
            end_time: record.end_time || '',
            game_date: record.game_date || '',
            is_finished: !!record.is_finished,
            match_victory: !!record.victory,
            match_days: getDisplayDay(record),
            match_total_wins: totalWins,
            match_total_losses: totalLosses,
            battle_start_time: entry.battle.start_time || '',
            duration: entry.battle.duration ?? null,
            lineup_cards: entry.battle.lineup_cards || [],
            enemy_lineup_cards: entry.battle.enemy_lineup_cards || [],
          },
        };

        const saveRes = await fetch(`${base}/api/game-records`, {
          method: 'POST',
          headers: buildAuthHeaders(),
          body: JSON.stringify(payload),
        });
        const saveJson = await parseJsonSafe(saveRes);
        if (!saveRes.ok) {
          throw new Error((saveJson as any)?.error || `写入战绩失败 (${saveRes.status})`);
        }
        if ((saveJson as any)?.duplicated) {
          skipped += 1;
          existingKeys.add(entry.key);
          continue;
        }

        uploaded += 1;
        existingKeys.add(entry.key);
        done += 1;
        emitProgress();
      } catch (error: any) {
        failed += 1;
        done += 1;
        emitProgress();
        console.warn('[HistoryUpload] upload battle failed:', error);
      }
    }

    return { uploaded, skipped, failed }
  };

  const handleUploadSingleMatch = async (record: MatchHistoryRecord) => {
    if (!record?.match_id) {
      showToast('缺少 match_id，无法上传该局', 'error');
      return;
    }
    setUploadingMatches((prev) => new Set(prev).add(record.match_id));
    setUploadProgressByMatch((prev) => ({
      ...prev,
      [record.match_id]: { done: 0, total: 0, uploaded: 0, skipped: 0, failed: 0 },
    }));
    try {
      const author = await ensureIdentity();
      const check = await checkUploadedMatches(author.account_id, [record.match_id]);
      const localKeys = (record.pvp_battles || [])
        .filter((battle) => Boolean(battle.screenshot && `${battle.screenshot}`.trim().length > 0))
        .map((battle) => buildBattleKey(record.match_id, Number(battle.day || 0), String(battle.start_time || ''), battle.victory ? 'win' : 'lose'));
      if (localKeys.length > 0 && localKeys.every((k) => check.existingBattleKeys.has(k))) {
        showToast('该对局已上传，无需重复上传', 'info');
        return;
      }
      const result = await uploadMatchBattles(record, author, check.existingBattleKeys, (progress) => {
        setUploadProgressByMatch((prev) => ({ ...prev, [record.match_id]: progress }));
      });
      if (result.uploaded > 0) {
        showToast(`上传完成：新增 ${result.uploaded}，跳过 ${result.skipped}，失败 ${result.failed}`, result.failed > 0 ? 'warning' : 'success');
      } else if (result.failed > 0) {
        showToast(`上传失败：失败 ${result.failed}，请检查服务配置`, 'error');
      } else {
        showToast('该对局已上传，无需重复上传', 'info');
      }
    } catch (error: any) {
      const message = typeof error === 'string' ? error : (error?.message || '未知错误');
      showToast(`上传失败：${message}`, 'error');
    } finally {
      setUploadingMatches((prev) => {
        const next = new Set(prev);
        next.delete(record.match_id);
        return next;
      });
      setUploadProgressByMatch((prev) => {
        const next = { ...prev };
        delete next[record.match_id];
        return next;
      });
    }
  };

  const handleUploadAllMatches = async () => {
    if (bulkUploading) return;
    const uploadTargets = records.filter((r) => String(r.match_id || '').trim().length > 0);
    const matchIds = uploadTargets.map((r) => String(r.match_id || '').trim());
    if (matchIds.length === 0) {
      showToast('暂无可上传的对局', 'warning');
      return;
    }
    setBulkUploading(true);
    setBulkProgress({ done: 0, total: matchIds.length });
    try {
      const author = await ensureIdentity();
      const check = await checkUploadedMatches(author.account_id, matchIds);
      const mutableExisting = new Set(check.existingBattleKeys);
      let totalUploaded = 0;
      let totalSkipped = 0;
      let totalFailed = 0;

      for (let idx = 0; idx < uploadTargets.length; idx += 1) {
        const record = uploadTargets[idx];
        const matchId = String(record.match_id || '').trim();
        setUploadingMatches((prev) => new Set(prev).add(matchId));
        setUploadProgressByMatch((prev) => ({
          ...prev,
          [matchId]: { done: 0, total: 0, uploaded: 0, skipped: 0, failed: 0 },
        }));
        const result = await uploadMatchBattles(record, author, mutableExisting, (progress) => {
          setUploadProgressByMatch((prev) => ({ ...prev, [matchId]: progress }));
        });
        totalUploaded += result.uploaded;
        totalSkipped += result.skipped;
        totalFailed += result.failed;
        setUploadingMatches((prev) => {
          const next = new Set(prev);
          next.delete(matchId);
          return next;
        });
        setUploadProgressByMatch((prev) => {
          const next = { ...prev };
          delete next[matchId];
          return next;
        });
        setBulkProgress({ done: idx + 1, total: matchIds.length });
      }

      if (totalUploaded > 0) {
        showToast(`批量上传完成：新增 ${totalUploaded}，跳过 ${totalSkipped}，失败 ${totalFailed}`, totalFailed > 0 ? 'warning' : 'success');
      } else if (totalFailed > 0) {
        showToast(`批量上传失败：失败 ${totalFailed}`, 'error');
      } else {
        showToast('全部对局均已上传，无需重复上传', 'info');
      }
    } catch (error: any) {
      const message = typeof error === 'string' ? error : (error?.message || '未知错误');
      showToast(`批量上传失败：${message}`, 'error');
    } finally {
      setBulkUploading(false);
      setBulkProgress({ done: 0, total: 0 });
      setUploadingMatches(new Set());
      setUploadProgressByMatch({});
    }
  };

  const handleConfigureUpload = () => {
    const nextBase = window.prompt('上传服务地址（例如 https://www.duang.work）', uploadApiBase);
    if (nextBase === null) return;
    const normalized = normalizeApiBase(nextBase);
    if (!normalized) {
      showToast('上传服务地址不能为空', 'warning');
      return;
    }
    const nextKey = window.prompt('插件密钥（可空）', uploadPluginKey);
    if (nextKey === null) return;
    setUploadApiBase(normalized);
    setUploadPluginKey(nextKey.trim());
    showToast('上传配置已保存', 'success');
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
    const uploadProgress = uploadProgressByMatch[record.match_id];
    const uploading = uploadingMatches.has(record.match_id);
    const uploadPercent = uploadProgress && uploadProgress.total > 0
      ? Math.round((uploadProgress.done / uploadProgress.total) * 100)
      : 0;

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
                {record.hero || 'Unknown'} · {formatDate(record.game_date)} {formatTime(record.start_time)} · Day {getDisplayDay(record)}
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
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 }}>
              <span style={{ color: '#8f8f8f', fontSize: 12 }}>MatchId: {record.match_id}</span>
              <button
                className="bulk-btn"
                style={{ padding: '4px 10px', fontSize: 12 }}
                disabled={uploading || bulkUploading}
                onClick={() => void handleUploadSingleMatch(record)}
              >
                {uploading ? `上传中 ${uploadProgress?.done || 0}/${uploadProgress?.total || 0}` : '上传本局'}
              </button>
            </div>
            {uploading && uploadProgress && (
              <div className="history-upload-progress">
                <div className="history-upload-progress-track">
                  <div className="history-upload-progress-fill" style={{ width: `${uploadPercent}%` }} />
                </div>
                <div className="history-upload-progress-text">
                  {uploadPercent}% · 成功 {uploadProgress.uploaded} / 跳过 {uploadProgress.skipped} / 失败 {uploadProgress.failed}
                </div>
              </div>
            )}
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
        <div className="history-title-row">
          <div className="history-title-left">
            <h2>历史战绩</h2>
            <div className="history-summary">
              小局战绩: {overall.wins} 胜 {overall.losses} 负 ({overall.total} 场)
            </div>
          </div>
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
        </div>
        
        <div className="history-toolbar">
          <div className="history-actions-group">
            <button className="bulk-btn" onClick={() => setBattleSortDesc((prev) => !prev)}>
              {battleSortDesc ? '排序: 新→旧' : '排序: 旧→新'}
            </button>
            <button className="bulk-btn" onClick={onReload}>
              {isLoading ? '刷新中...' : '刷新'}
            </button>
          </div>
          <div className="history-actions-group">
            <button className="bulk-btn secondary" onClick={handleConfigureUpload}>
              配置上传
            </button>
            <button className="bulk-btn primary" disabled={bulkUploading} onClick={() => void handleUploadAllMatches()}>
              {bulkUploading ? `上传中 ${bulkProgress.done}/${bulkProgress.total}` : '一键上传全部'}
            </button>
          </div>
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
