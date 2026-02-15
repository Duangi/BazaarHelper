import { useCallback, type Dispatch, type SetStateAction } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

import type { Toast } from '../../types';

interface UseUpdateActionsOptions {
  showToast: (message: string, type?: Toast['type']) => void;
  currentVersion: string;
  setAnnouncement: Dispatch<SetStateAction<string>>;
  setIsInstalling: Dispatch<SetStateAction<boolean>>;
  setShowResetHotkeysConfirm: Dispatch<SetStateAction<boolean>>;
  setDetectionHotkey: Dispatch<SetStateAction<number | null>>;
  setCardDetectionHotkey: Dispatch<SetStateAction<number | null>>;
  setToggleCollapseHotkey: Dispatch<SetStateAction<number | null>>;
  setYoloHotkey: Dispatch<SetStateAction<number | null>>;
  setDetailDisplayHotkey: Dispatch<SetStateAction<number | null>>;
  setUpdateStatus: Dispatch<SetStateAction<any>>;
  setUpdateAvailable: Dispatch<SetStateAction<any>>;
}

export const useUpdateActions = ({
  showToast,
  currentVersion,
  setAnnouncement,
  setIsInstalling,
  setShowResetHotkeysConfirm,
  setDetectionHotkey,
  setCardDetectionHotkey,
  setToggleCollapseHotkey,
  setYoloHotkey,
  setDetailDisplayHotkey,
  setUpdateStatus,
  setUpdateAvailable,
}: UseUpdateActionsOptions) => {
  const handleConfirmResetHotkeys = useCallback(async () => {
    setShowResetHotkeysConfirm(false);
    try {
      await invoke('reset_all_hotkeys');
      const detection = await invoke<number | null>('get_detection_hotkey');
      const card = await invoke<number | null>('get_card_detection_hotkey');
      const toggle = await invoke<number | null>('get_toggle_collapse_hotkey');
      const yolo = await invoke<number | null>('get_yolo_hotkey');
      const detail = await invoke<number | null>('get_detail_display_hotkey');

      setDetectionHotkey(detection || 0);
      setCardDetectionHotkey(card || 0);
      setToggleCollapseHotkey(toggle || 0);
      setYoloHotkey(yolo || 0);
      setDetailDisplayHotkey(detail || 0);

      localStorage.removeItem('detection-hotkey');
      localStorage.removeItem('card-detection-hotkey');
      localStorage.removeItem('toggle-collapse-hotkey');
      localStorage.removeItem('yolo-hotkey');
      localStorage.removeItem('detail-display-hotkey');

      showToast('所有快捷键已重置', 'success');
    } catch (e) {
      console.error('Failed to reset hotkeys:', e);
      showToast('重置失败', 'error');
    }
  }, [
    setShowResetHotkeysConfirm,
    setDetectionHotkey,
    setCardDetectionHotkey,
    setToggleCollapseHotkey,
    setYoloHotkey,
    setDetailDisplayHotkey,
    showToast,
  ]);

  const handleManualCheckUpdate = useCallback(async () => {
    const ENDPOINT = 'https://gh.llkk.cc/https://raw.githubusercontent.com/Duangi/BazaarHelper/main/update.json';
    console.log('[Update] 用户手动触发更新检查...');
    console.log(`[Update] 目标 JSON 地址: ${ENDPOINT}`);
    setUpdateStatus('checking');

    try {
      console.log('[Update] 尝试手动 Fetch 远端内容以验证访问...');
      const response = await fetch(ENDPOINT, { cache: 'no-store' });
      if (response.ok) {
        const remoteJson = await response.json();
        console.log('[Update] 远端 JSON 内容获取成功:', remoteJson);
        console.log(`[Update] 远端版本: ${remoteJson.version}, 当前本地版本: ${currentVersion}`);

        if (remoteJson.notes) {
          setAnnouncement(remoteJson.notes);
        }

        if (remoteJson.version === currentVersion) {
          console.log('[Update] 提示: 版本号完全一致，Tauri check() 必然返回 null');
        }
      } else {
        console.error(`[Update] 远端 JSON 访问失败! 状态码: ${response.status}`);
      }

      console.log('[Update] 调用 Tauri 插件 check() 进行正式比对与签名校验...');
      const u = await check();
      console.log('[Update] check() 返回对象:', u);

      if (u) {
        console.log(`[Update] 手动检查发现新版本: v${u.version}`);
        setUpdateAvailable(u);
        setUpdateStatus('available');
      } else {
        console.log('[Update] 手动检查结果: 已经是最新版本 (check 返回 null)');
        setUpdateStatus('none');
      }
    } catch (e) {
      console.error('[Update] 手动检查过程中发生异常:', e);
      setUpdateStatus('none');
    }
  }, [currentVersion, setAnnouncement, setUpdateAvailable, setUpdateStatus]);

  const handleInstallReady = useCallback(() => {
    setIsInstalling(true);
    setTimeout(() => {
      void invoke('prepare_app_exit')
        .catch(() => {})
        .finally(() => {
          void relaunch();
        });
    }, 1000);
  }, [setIsInstalling]);

  return {
    handleConfirmResetHotkeys,
    handleManualCheckUpdate,
    handleInstallReady,
  };
};
