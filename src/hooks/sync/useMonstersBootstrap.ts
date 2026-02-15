import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { Dispatch, SetStateAction } from 'react';

import type { MonsterData } from '../../types';

interface UseMonstersBootstrapOptions {
  setAllMonsters: Dispatch<SetStateAction<Record<string, MonsterData>>>;
}

export const useMonstersBootstrap = ({ setAllMonsters }: UseMonstersBootstrapOptions) => {
  useEffect(() => {
    let cancelled = false;
    let retryCount = 0;

    const loadAllMonsters = async () => {
      if (cancelled) return;
      try {
        const res: Record<string, MonsterData> = await invoke('get_all_monsters');
        if (res && Object.keys(res).length > 0) {
          console.log(`[Init] Loaded ${Object.keys(res).length} monsters from backend.`);
          setAllMonsters(res);
          return;
        }

        retryCount += 1;
        if (retryCount % 5 === 0) {
          console.log(`[Init] Monsters DB still empty, retrying... (attempt ${retryCount})`);
        }
        setTimeout(loadAllMonsters, 1000);
      } catch (e) {
        console.error('加载全量怪物失败:', e);
        setTimeout(loadAllMonsters, 1500);
      }
    };

    void loadAllMonsters();
    return () => {
      cancelled = true;
    };
  }, [setAllMonsters]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setup = async () => {
      try {
        unlisten = await listen('monsters-db-ready', async (event: any) => {
          try {
            console.log('[Event] monsters-db-ready payload:', event.payload);
            const res: Record<string, MonsterData> = await invoke('get_all_monsters');
            setAllMonsters(res);
          } catch (e) {
            console.error('Failed to reload monsters after monsters-db-ready:', e);
          }
        });
      } catch (e) {
        console.warn('Failed to listen for monsters-db-ready:', e);
      }
    };

    void setup();
    return () => {
      if (unlisten) unlisten();
    };
  }, [setAllMonsters]);
};
