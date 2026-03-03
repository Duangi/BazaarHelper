import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Dispatch, SetStateAction } from 'react';

import type { ItemData, SyncPayload } from '../../types';

interface UseInitialSyncOptions {
  enabled?: boolean;
  templateLoadingComplete: boolean;
  processItems: (items: ItemData[]) => Promise<ItemData[]>;
  setCurrentDay: (day: number) => void;
  setSelectedDay: (day: string) => void;
  setSyncData: Dispatch<SetStateAction<SyncPayload & { monster: any[] }>>;
}

export const useInitialSync = ({
  enabled = true,
  templateLoadingComplete,
  processItems,
  setCurrentDay,
  setSelectedDay,
  setSyncData,
}: UseInitialSyncOptions) => {
  useEffect(() => {
    if (!enabled) return;
    if (!templateLoadingComplete) return;

    const doInitialSync = async () => {
      try {
        console.log('[App] Templates loaded, fetching initial sync state...');
        const state: any = await invoke('get_sync_state');
        console.log('[App] Initial state:', state);
        if (!state) return;

        if (state.day !== undefined) {
          setCurrentDay(state.day);
          setSelectedDay(state.day >= 10 ? 'Day 10+' : `Day ${state.day}`);
        }

        const [hand, stash] = await Promise.all([
          processItems(state.hand_items || []),
          processItems(state.stash_items || []),
        ]);

        setSyncData((prev) => ({
          ...prev,
          hand_items: hand,
          stash_items: stash,
          all_tags: state.all_tags || [],
        }));
      } catch (e) {
        console.error('[App] Initial sync failed', e);
      }
    };

    void doInitialSync();
  }, [enabled, processItems, setCurrentDay, setSelectedDay, setSyncData, templateLoadingComplete]);
};
