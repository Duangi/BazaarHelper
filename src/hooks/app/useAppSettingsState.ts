import { useCallback, useEffect, useState } from 'react';

import type { Toast } from '../../types';

export const useAppSettingsState = () => {
  const [fontSize, setFontSize] = useState(() => {
    const saved = localStorage.getItem('user-font-size');
    return saved ? parseInt(saved, 10) : 16;
  });
  const [showSettings, setShowSettings] = useState(false);

  const [enableYoloAuto, setEnableYoloAuto] = useState(() => {
    const saved = localStorage.getItem('enable-yolo-auto');
    return saved === 'true';
  });
  const [yoloScanInterval, setYoloScanInterval] = useState(() => {
    const saved = localStorage.getItem('yolo-scan-interval');
    return saved ? parseFloat(saved) : 1.0;
  });

  useEffect(() => {
    console.log(
      `[App Config] Loaded from cache - EnableYoloAuto: ${enableYoloAuto}, Interval: ${yoloScanInterval}s`,
    );
  }, []);

  const [useGpuAcceleration, setUseGpuAcceleration] = useState(() => {
    const saved = localStorage.getItem('use-gpu-acceleration');
    if (saved === null) {
      localStorage.setItem('use-gpu-acceleration', 'true');
      return true;
    }
    return saved === 'true';
  });
  const [showYoloMonitor, setShowYoloMonitor] = useState(() => {
    const saved = localStorage.getItem('show-yolo-monitor');
    if (saved === null) {
      localStorage.setItem('show-yolo-monitor', 'false');
      return false;
    }
    return saved === 'true';
  });
  const [yoloHotkey, setYoloHotkey] = useState<number | null>(() => {
    const saved = localStorage.getItem('yolo-hotkey');
    return saved ? parseInt(saved) : 0;
  });

  const [toasts, setToasts] = useState<Toast[]>([]);
  const showToast = useCallback((message: string, type: 'success' | 'error' | 'warning' | 'info' = 'info') => {
    const id = Date.now();
    setToasts((prev) => [...prev, { id, message, type }]);
  }, []);

  const [detectionHotkey, setDetectionHotkey] = useState<number | null>(null);
  const [cardDetectionHotkey, setCardDetectionHotkey] = useState<number | null>(null);
  const [toggleCollapseHotkey, setToggleCollapseHotkey] = useState<number | null>(null);
  const [detailDisplayHotkey, setDetailDisplayHotkey] = useState<number | null>(null);

  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [isRecordingCardHotkey, setIsRecordingCardHotkey] = useState(false);
  const [isRecordingToggleHotkey, setIsRecordingToggleHotkey] = useState(false);
  const [isRecordingYoloHotkey, setIsRecordingYoloHotkey] = useState(false);
  const [isRecordingDetailHotkey, setIsRecordingDetailHotkey] = useState(false);
  const [showResetHotkeysConfirm, setShowResetHotkeysConfirm] = useState(false);

  const [settingsExpanded, setSettingsExpanded] = useState({
    ui: false,
    yolo: false,
    hotkeys: false,
    debug: false,
  });

  return {
    fontSize,
    setFontSize,
    showSettings,
    setShowSettings,
    enableYoloAuto,
    setEnableYoloAuto,
    yoloScanInterval,
    setYoloScanInterval,
    useGpuAcceleration,
    setUseGpuAcceleration,
    showYoloMonitor,
    setShowYoloMonitor,
    yoloHotkey,
    setYoloHotkey,
    toasts,
    setToasts,
    showToast,
    detectionHotkey,
    setDetectionHotkey,
    cardDetectionHotkey,
    setCardDetectionHotkey,
    toggleCollapseHotkey,
    setToggleCollapseHotkey,
    detailDisplayHotkey,
    setDetailDisplayHotkey,
    isRecordingHotkey,
    setIsRecordingHotkey,
    isRecordingCardHotkey,
    setIsRecordingCardHotkey,
    isRecordingToggleHotkey,
    setIsRecordingToggleHotkey,
    isRecordingYoloHotkey,
    setIsRecordingYoloHotkey,
    isRecordingDetailHotkey,
    setIsRecordingDetailHotkey,
    showResetHotkeysConfirm,
    setShowResetHotkeysConfirm,
    settingsExpanded,
    setSettingsExpanded,
  };
};
