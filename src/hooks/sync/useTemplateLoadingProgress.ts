import { useEffect, type Dispatch, type SetStateAction } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface TemplateLoadingState {
  loaded: number;
  total: number;
  is_complete: boolean;
  current_name: string;
}

export const useTemplateLoadingProgress = (
  setTemplateLoading: Dispatch<SetStateAction<TemplateLoadingState>>
) => {
  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | null = null;

    const checkProgress = async () => {
      try {
        const progress = (await invoke('get_template_loading_progress')) as TemplateLoadingState;
        setTemplateLoading(progress);

        if (progress.is_complete && timer) {
          clearInterval(timer);
          timer = null;
        }
      } catch (e) {
        console.error('获取加载进度失败:', e);
      }
    };

    void checkProgress();
    timer = setInterval(checkProgress, 500);

    return () => {
      if (timer) {
        clearInterval(timer);
      }
    };
  }, [setTemplateLoading]);
};
