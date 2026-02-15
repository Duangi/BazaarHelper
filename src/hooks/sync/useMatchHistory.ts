import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { MatchHistoryRecord } from '../../types';

export const useMatchHistory = () => {
  const [matchHistory, setMatchHistory] = useState<MatchHistoryRecord[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);

  const loadMatchHistory = useCallback(async () => {
    setIsLoadingHistory(true);
    try {
      const payload = await invoke<{ matches?: MatchHistoryRecord[] }>('get_match_history');
      setMatchHistory(Array.isArray(payload?.matches) ? payload.matches : []);
    } catch (error) {
      console.error('[History] Failed to load match history:', error);
      setMatchHistory([]);
    } finally {
      setIsLoadingHistory(false);
    }
  }, []);

  useEffect(() => {
    void loadMatchHistory();
  }, [loadMatchHistory]);

  useEffect(() => {
    let disposed = false;
    const unlisten = listen('match-history-updated', () => {
      if (!disposed) {
        void loadMatchHistory();
      }
    });

    return () => {
      disposed = true;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [loadMatchHistory]);

  return {
    matchHistory,
    isLoadingHistory,
    loadMatchHistory,
  };
};
