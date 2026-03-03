import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface UseMonsterHotkeyListenerOptions {
  enabled?: boolean;
  currentDay: number | null;
  onSwitchToMonsterTab: () => void;
  onAutoRecognition: (day: number | null) => Promise<void>;
}

export const useMonsterHotkeyListener = ({
  enabled = true,
  currentDay,
  onSwitchToMonsterTab,
  onAutoRecognition,
}: UseMonsterHotkeyListenerOptions) => {
  useEffect(() => {
    if (!enabled) return;

    const unlistenMonster = listen('hotkey-monster', async () => {
      console.log('[App] Received hotkey-monster');
      onSwitchToMonsterTab();
      await onAutoRecognition(currentDay);
    });

    return () => {
      unlistenMonster.then((f) => f());
    };
  }, [currentDay, enabled, onAutoRecognition, onSwitchToMonsterTab]);
};
