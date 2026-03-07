import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { IslandStatusType } from '../../types';

interface UseYoloScanOptions {
  enabled?: boolean;
  enableYoloAuto: boolean;
  isCollapsed: boolean;
  yoloScanInterval: number;
  yoloHotkey: number | null;
  setErrorMessage: Dispatch<SetStateAction<string | null>>;
  updateIslandStatus: (message: string, type?: IslandStatusType) => void;
  showToast?: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
}

export const useYoloScan = ({
  enabled = true,
  enableYoloAuto,
  yoloScanInterval,
  yoloHotkey,
  setErrorMessage,
  updateIslandStatus,
  showToast,
}: UseYoloScanOptions) => {
  const [isPageVisible, setIsPageVisible] = useState(() => document.visibilityState === 'visible');
  const lastCacheClearAtRef = useRef(0);

  const clearYoloCacheThrottled = useCallback(() => {
    const now = Date.now();
    // Avoid high-frequency IPC churn when visibility/collapse state flaps.
    if (now - lastCacheClearAtRef.current < 15_000) {
      return;
    }
    lastCacheClearAtRef.current = now;
    void invoke('clear_yolo_cache').catch(() => {});
  }, []);

  useEffect(() => {
    if (!enabled) return;

    const onVisibilityChange = () => {
      const visible = document.visibilityState === 'visible';
      setIsPageVisible(visible);
      if (!visible && !enableYoloAuto) {
        clearYoloCacheThrottled();
      }
    };
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => document.removeEventListener('visibilitychange', onVisibilityChange);
  }, [clearYoloCacheThrottled, enabled]);

  const runYoloScan = useCallback(async (source: 'manual' | 'auto' = 'manual') => {
    const useGpu = localStorage.getItem('use-gpu-acceleration');
    const useGpuBool = useGpu === 'true';
    const sourceLabel = source === 'manual' ? '手动' : '自动';
    const shouldNotifyUi = true;
    const shouldShowToast = source === 'manual';

    try {
      if (source === 'auto') {
        try {
          const gameForeground = await invoke<boolean>('is_game_window_active_cmd');
          if (!gameForeground) {
            return;
          }
        } catch {
          // ignore gating failure and continue
        }
      }

      if ((window as any).__yolo_running) {
        console.log('[YOLO Manual/Auto] Scan already running, skipping');
        if (shouldNotifyUi) {
          updateIslandStatus(`YOLO(${sourceLabel}): 正在扫描中...`, 'info');
        }
        return;
      }
      (window as any).__yolo_running = true;
      const startedAt = performance.now();
      console.log(`[YOLO Manual/Auto] Starting scan (GPU: ${useGpuBool})`);
      if (shouldNotifyUi) {
        updateIslandStatus(`YOLO(${sourceLabel}): 正在扫描...`, 'info');
      }
      const count = await invoke<number>('trigger_yolo_scan', {
        useGpu: useGpuBool,
        storeImage: true,
      });
      const elapsedMs = Math.round(performance.now() - startedAt);
      console.log(`[YOLO Manual/Auto] Scan complete, detected ${count} objects`);
      if (count > 0) {
        if (shouldNotifyUi) {
          updateIslandStatus(`YOLO(${sourceLabel}): 扫描完成，共 ${count} 个目标 (${elapsedMs}ms)`, 'success');
          if (shouldShowToast) {
            showToast?.(`YOLO扫描完成：识别到 ${count} 个目标`, 'success');
          }
        }
      } else {
        if (shouldNotifyUi) {
          updateIslandStatus(`YOLO(${sourceLabel}): 扫描完成，共 0 个目标 (${elapsedMs}ms)`, 'warning');
          if (shouldShowToast) {
            showToast?.('YOLO扫描完成：未识别到目标', 'warning');
          }
        }
      }

      try {
        const stats = await invoke('get_yolo_stats');
        await emit('yolo-stats-updated', stats);
      } catch (statsErr) {
        console.error('[YOLO Manual/Auto] Failed to get stats:', statsErr);
      }
    } catch (err) {
      console.error('[YOLO Manual/Auto] Scan failed:', err);
      if (shouldNotifyUi) {
        setErrorMessage(`YOLO扫描失败: ${err}`);
        updateIslandStatus('YOLO 扫描失败', 'error');
        if (shouldShowToast) {
          showToast?.(`YOLO扫描失败: ${err}`, 'error');
        }
      }
    } finally {
      (window as any).__yolo_running = false;
    }
  }, [setErrorMessage, showToast, updateIslandStatus]);

  useEffect(() => {
    if (!enabled) return;

    if (!enableYoloAuto || !isPageVisible) {
      if (!enableYoloAuto) console.log('[YOLO Auto] Auto scan disabled');
      if (!isPageVisible) console.log('[YOLO Auto] Auto scan paused while page is hidden');
      return;
    }

    const effectiveIntervalSec = Math.max(0.5, Number.isFinite(yoloScanInterval) ? yoloScanInterval : 1);
    const yoloTimer = setInterval(() => {
      void runYoloScan('auto');
    }, effectiveIntervalSec * 1000);
    console.log(`[YOLO Auto] Timer started with interval: ${effectiveIntervalSec}s`);

    return () => {
      clearInterval(yoloTimer);
      console.log('[YOLO Auto] Timer stopped');
    };
  }, [enableYoloAuto, enabled, isPageVisible, yoloScanInterval, runYoloScan]);

  useEffect(() => {
    if (!enabled) return;

    const unlisten = listen('yolo_hotkey_pressed', () => {
      console.log('[YOLO Hotkey] Triggered');
      void runYoloScan('manual');
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [enabled, runYoloScan]);

  useEffect(() => {
    if (!enabled) return;
    if (yoloHotkey) {
      invoke('set_yolo_hotkey', { hotkey: yoloHotkey }).catch(console.error);
    }
  }, [enabled, yoloHotkey]);

  return { runYoloScan };
};
