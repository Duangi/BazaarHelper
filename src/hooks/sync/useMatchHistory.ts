import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { MatchHistoryRecord } from '../../types';

interface UseMatchHistoryOptions {
  enabled?: boolean;
}

export const useMatchHistory = ({ enabled = true }: UseMatchHistoryOptions = {}) => {
  const [matchHistory, setMatchHistory] = useState<MatchHistoryRecord[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);
  const [dirtyWhileHidden, setDirtyWhileHidden] = useState(false);
  const inFlightRef = useRef(false);
  const hasLoadedOnceRef = useRef(false);

  const loadMatchHistory = useCallback(async (options?: { rebuild?: boolean; force?: boolean }) => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    setIsLoadingHistory(true);
    try {
      if (options?.rebuild) {
        await invoke('rebuild_match_history', { force: Boolean(options.force) });
      }
      const payload = await invoke<{ matches?: MatchHistoryRecord[] }>('get_match_history');
      const nextMatches = Array.isArray(payload?.matches) ? payload.matches : [];
      setMatchHistory((prev) => {
        if (prev.length !== nextMatches.length) return nextMatches;
        for (let i = 0; i < prev.length; i += 1) {
          const a = prev[i];
          const b = nextMatches[i];
          if (
            a.match_id !== b.match_id
            || a.hero !== b.hero
            || a.game_date !== b.game_date
            || a.days !== b.days
            || a.victory !== b.victory
            || Boolean(a.is_finished) !== Boolean(b.is_finished)
            || (a.pvp_battles?.length || 0) !== (b.pvp_battles?.length || 0)
          ) {
            return nextMatches;
          }

          const aBattles = a.pvp_battles || [];
          const bBattles = b.pvp_battles || [];
          for (let j = 0; j < aBattles.length; j += 1) {
            const ba = aBattles[j];
            const bb = bBattles[j];
            if (!bb) return nextMatches;
            if (
              ba.day !== bb.day
              || ba.start_time !== bb.start_time
              || ba.victory !== bb.victory
              || (ba.duration ?? null) !== (bb.duration ?? null)
              || (ba.screenshot ?? null) !== (bb.screenshot ?? null)
              || JSON.stringify(ba.lineup_cards ?? []) !== JSON.stringify(bb.lineup_cards ?? [])
              || JSON.stringify(ba.enemy_lineup_cards ?? []) !== JSON.stringify(bb.enemy_lineup_cards ?? [])
            ) {
              return nextMatches;
            }
          }
        }
        return prev;
      });
      setDirtyWhileHidden(false);
    } catch (error) {
      console.error('[History] Failed to load match history:', error);
      setMatchHistory([]);
    } finally {
      setIsLoadingHistory(false);
      inFlightRef.current = false;
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      hasLoadedOnceRef.current = false;
      setMatchHistory((prev) => (prev.length === 0 ? prev : []));
      setDirtyWhileHidden(false);
      return;
    }
    if (!hasLoadedOnceRef.current || dirtyWhileHidden) {
      hasLoadedOnceRef.current = true;
      void loadMatchHistory({ rebuild: true, force: true });
    }
  }, [dirtyWhileHidden, enabled, loadMatchHistory]);

  useEffect(() => {
    let disposed = false;
    const unlisten = listen('match-history-updated', () => {
      if (disposed) return;
      if (enabled) {
        void loadMatchHistory({ rebuild: false, force: false });
      } else {
        setDirtyWhileHidden(true);
      }
    });

    return () => {
      disposed = true;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [enabled, loadMatchHistory]);

  const forceReload = useCallback(async () => {
    void loadMatchHistory({ rebuild: true, force: true });
  }, [loadMatchHistory]);

  return {
    matchHistory,
    isLoadingHistory,
    loadMatchHistory: forceReload,
  };
};
