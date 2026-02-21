import { useCallback, useEffect, type Dispatch, type SetStateAction } from 'react';
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { IslandStatusType } from '../../types';

interface UseYoloScanOptions {
  enableYoloAuto: boolean;
  yoloScanInterval: number;
  yoloHotkey: number | null;
  setErrorMessage: Dispatch<SetStateAction<string | null>>;
  updateIslandStatus: (message: string, type?: IslandStatusType) => void;
  showToast?: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
}

export const useYoloScan = ({
  enableYoloAuto,
  yoloScanInterval,
  yoloHotkey,
  setErrorMessage,
  updateIslandStatus,
  showToast,
}: UseYoloScanOptions) => {
  const runYoloScan = useCallback(async (source: 'manual' | 'auto' = 'manual') => {
    const useGpu = localStorage.getItem('use-gpu-acceleration');
    const useGpuBool = useGpu === 'true';
    const sourceLabel = source === 'manual' ? '手动' : '自动';
    const shouldNotifyUi = source === 'manual';

    try {
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
      const count = await invoke<number>('trigger_yolo_scan', { useGpu: useGpuBool });
      const elapsedMs = Math.round(performance.now() - startedAt);
      console.log(`[YOLO Manual/Auto] Scan complete, detected ${count} objects`);
      if (count > 0) {
        if (shouldNotifyUi) {
          updateIslandStatus(`YOLO(${sourceLabel}): 识别到 ${count} 个目标 (${elapsedMs}ms)`, 'success');
          showToast?.(`YOLO扫描完成：识别到 ${count} 个目标`, 'success');
        }
      } else {
        if (shouldNotifyUi) {
          updateIslandStatus(`YOLO(${sourceLabel}): 未识别到目标 (${elapsedMs}ms)`, 'warning');
          showToast?.('YOLO扫描完成：未识别到目标', 'warning');
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
        showToast?.(`YOLO扫描失败: ${err}`, 'error');
      }
    } finally {
      (window as any).__yolo_running = false;
    }
  }, [setErrorMessage, showToast, updateIslandStatus]);

  useEffect(() => {
    if (!enableYoloAuto) {
      console.log('[YOLO Auto] Auto scan disabled');
      return;
    }

    const yoloTimer = setInterval(() => {
      void runYoloScan('auto');
    }, yoloScanInterval * 1000);
    console.log(`[YOLO Auto] Timer started with interval: ${yoloScanInterval}s`);

    return () => {
      clearInterval(yoloTimer);
      console.log('[YOLO Auto] Timer stopped');
    };
  }, [enableYoloAuto, yoloScanInterval, runYoloScan]);

  useEffect(() => {
    const unlisten = listen('yolo_hotkey_pressed', () => {
      console.log('[YOLO Hotkey] Triggered');
      void runYoloScan('manual');
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [runYoloScan]);

  useEffect(() => {
    if (yoloHotkey) {
      invoke('set_yolo_hotkey', { hotkey: yoloHotkey }).catch(console.error);
    }
  }, [yoloHotkey]);

  return { runYoloScan };
};
