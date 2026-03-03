import { useEffect, type Dispatch, type SetStateAction } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface TemplateLoadingState {
  loaded: number;
  total: number;
  is_complete: boolean;
  current_name: string;
}

export const useTemplateLoadingProgress = (
  setTemplateLoading: Dispatch<SetStateAction<TemplateLoadingState>>,
  enabled = true,
) => {
  useEffect(() => {
    if (!enabled) return;

    let timer: ReturnType<typeof setInterval> | null = null;
    let pollCount = 0;
    const MAX_POLLS = 180; // 90s upper bound to avoid long-lived idle polling leaks

    const checkProgress = async () => {
      try {
        pollCount += 1;
        const progress = (await invoke('get_template_loading_progress')) as TemplateLoadingState;
        setTemplateLoading((prev) => {
          if (
            prev.loaded === progress.loaded
            && prev.total === progress.total
            && prev.is_complete === progress.is_complete
            && prev.current_name === progress.current_name
          ) {
            return prev;
          }
          return progress;
        });

        const effectivelyComplete =
          progress.is_complete
          || (progress.total > 0 && progress.loaded >= progress.total)
          || pollCount >= MAX_POLLS;

        if (effectivelyComplete && timer) {
          clearInterval(timer);
          timer = null;
        }
      } catch (e) {
        console.error('获取加载进度失败:', e);
        if (timer) {
          clearInterval(timer);
          timer = null;
        }
      }
    };

    void checkProgress();
    timer = setInterval(checkProgress, 500);

    return () => {
      if (timer) {
        clearInterval(timer);
      }
    };
  }, [enabled, setTemplateLoading]);
};
