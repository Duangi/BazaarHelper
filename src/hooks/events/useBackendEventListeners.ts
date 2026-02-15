import { useEffect, type Dispatch, type MutableRefObject, type SetStateAction } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

import type { SyncPayload } from '../../types';
import type { IslandStatusType } from '../../types';

interface UseBackendEventListenersOptions {
  isResizing: MutableRefObject<boolean>;
  processSyncPayload: (payload: SyncPayload) => Promise<void>;
  handleAutoRecognition: (day: number | null) => Promise<void>;
  handleRecognizeCard: (fromHotkey?: boolean) => void;
  setIsCollapsed: Dispatch<SetStateAction<boolean>>;
  setCurrentDay: Dispatch<SetStateAction<number | null>>;
  setSelectedDay: Dispatch<SetStateAction<string>>;
  setIdentifiedNames: Dispatch<SetStateAction<string[]>>;
  setExpandedMonsters: Dispatch<SetStateAction<Set<string>>>;
  setActiveTab: (tab: 'monster') => void;
  setDetectionHotkey: Dispatch<SetStateAction<number | null>>;
  setCardDetectionHotkey: Dispatch<SetStateAction<number | null>>;
  setToggleCollapseHotkey: Dispatch<SetStateAction<number | null>>;
  setDetailDisplayHotkey: Dispatch<SetStateAction<number | null>>;
  updateIslandStatus: (message: string, type?: IslandStatusType) => void;
}

export const useBackendEventListeners = ({
  isResizing,
  processSyncPayload,
  handleAutoRecognition,
  handleRecognizeCard,
  setIsCollapsed,
  setCurrentDay,
  setSelectedDay,
  setIdentifiedNames,
  setExpandedMonsters,
  setActiveTab,
  setDetectionHotkey,
  setCardDetectionHotkey,
  setToggleCollapseHotkey,
  setDetailDisplayHotkey,
  updateIslandStatus,
}: UseBackendEventListenersOptions) => {
  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      const safeListen = async <T,>(event: string, callback: (payload: T) => void | Promise<void>) => {
        try {
          const unlisten = await listen<T>(event, (e) => {
            if (isMounted) {
              void callback(e.payload);
            }
          });

          if (isMounted) {
            unlisteners.push(unlisten);
          } else {
            unlisten();
          }
        } catch (err) {
          console.error(`Failed to listen to ${event}:`, err);
        }
      };

      await safeListen<SyncPayload>('sync-items', async (payload) => {
        await processSyncPayload(payload);
      });

      await safeListen<number | null>('trigger-monster-recognition', (dayNum) => {
        console.log('收到自动识别触发事件, Day:', dayNum);
        if (dayNum) {
          const dayLabel = dayNum >= 10 ? 'Day 10+' : `Day ${dayNum}`;
          setSelectedDay(dayLabel);
          setCurrentDay(dayNum);
        }
        setTimeout(() => {
          if (isMounted) void handleAutoRecognition(dayNum);
        }, 500);
      });

      await safeListen<void>('hotkey-card', () => {
        console.log('收到卡牌识别触发事件');
        handleRecognizeCard(true);
      });

      await safeListen<void>('hotkey-collapse', () => {
        isResizing.current = false;
        setIsCollapsed((prev) => !prev);
      });

      await safeListen<{ day: number; monster_name: string }>('auto-jump-to-monster', (payload) => {
        const { day, monster_name } = payload;
        const names = monster_name.includes('|') ? monster_name.split('|') : [monster_name];

        isResizing.current = false;
        setIsCollapsed(false);
        setCurrentDay(day);
        setSelectedDay(day >= 10 ? 'Day 10+' : `Day ${day}`);
        setIdentifiedNames(names);
        setExpandedMonsters((prev) => {
          const next = new Set(prev);
          names.forEach((n: string) => next.add(n));
          return next;
        });

        setTimeout(() => {
          const element = document.getElementById(`monster-${names[0]}`);
          if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
          }
        }, 100);

        setActiveTab('monster');
      });

      await safeListen<{ name: string; name_zh: string }>('monster-matched', (payload) => {
        console.log('收到野怪匹配事件:', payload);
        setIdentifiedNames((prev) => {
          if (!prev.includes(payload.name)) {
            return [...prev, payload.name];
          }
          return prev;
        });
        setExpandedMonsters((prev) => {
          const next = new Set(prev);
          next.add(payload.name);
          return next;
        });
        setActiveTab('monster');
        setTimeout(() => {
          const element = document.getElementById(`monster-${payload.name}`);
          if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'center' });
          }
        }, 300);
      });

      await safeListen<number>('day-update', (d) => {
        setCurrentDay(d);
        setSelectedDay(d >= 10 ? 'Day 10+' : `Day ${d}`);
      });

      await safeListen<{ message: string; type?: IslandStatusType }>('island-status', (payload) => {
        if (!payload?.message) return;
        updateIslandStatus(payload.message, payload.type ?? 'info');
      });

      invoke<number | null>('get_detection_hotkey').then((val) => isMounted && setDetectionHotkey(val));
      invoke<number | null>('get_card_detection_hotkey').then((val) => isMounted && setCardDetectionHotkey(val));
      invoke<number | null>('get_toggle_collapse_hotkey').then((val) => isMounted && setToggleCollapseHotkey(val));
      invoke<number | null>('get_detail_display_hotkey').then((val) => isMounted && setDetailDisplayHotkey(val));
    };

    void setupListeners();

    return () => {
      isMounted = false;
      unlisteners.forEach((fn) => fn());
      unlisteners.length = 0;
    };
  }, [
    handleAutoRecognition,
    handleRecognizeCard,
    isResizing,
    processSyncPayload,
    setActiveTab,
    setCardDetectionHotkey,
    setCurrentDay,
    setDetailDisplayHotkey,
    setDetectionHotkey,
    setExpandedMonsters,
    setIdentifiedNames,
    setIsCollapsed,
    setSelectedDay,
    setToggleCollapseHotkey,
    updateIslandStatus,
  ]);
};
