import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface UseCurrentDayBootstrapOptions {
  currentDay: number | null;
  setCurrentDay: (day: number) => void;
  updateDayTabSelection: (day: number) => void;
}

export const useCurrentDayBootstrap = ({
  currentDay,
  setCurrentDay,
  updateDayTabSelection,
}: UseCurrentDayBootstrapOptions) => {
  useEffect(() => {
    let mounted = true;
    const fetchDay = async () => {
      try {
        const d = await invoke<number>('get_current_day', { hours_per_day: 6, retro: true });
        if (mounted && d !== currentDay) {
          setCurrentDay(d);
          updateDayTabSelection(d);
        }
      } catch (e) {
        console.warn('get_current_day failed:', e);
      }
    };
    void fetchDay();
    return () => {
      mounted = false;
    };
  }, []);
};
