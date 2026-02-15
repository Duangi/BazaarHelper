import { useCallback, useEffect, useState, type Dispatch, type SetStateAction } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

import type { ItemData, TabType } from '../../types';
import { getImg } from '../../utils/helpers';

interface UseCardRecognitionOptions {
  setActiveTab: Dispatch<SetStateAction<TabType>>;
  setExpandedItems: Dispatch<SetStateAction<Set<string>>>;
  setErrorMessage: Dispatch<SetStateAction<string | null>>;
  showToast: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
}

export const useCardRecognition = ({
  setActiveTab,
  setExpandedItems,
  setErrorMessage,
  showToast,
}: UseCardRecognitionOptions) => {
  const [recognizedCards, setRecognizedCards] = useState<ItemData[]>([]);
  const [isRecognizingCard, setIsRecognizingCard] = useState(false);

  useEffect(() => {
    const unlisten = listen<string>('scan-error', (event) => {
      console.error('[Backend Error]', event.payload);
      setErrorMessage(`识别错误: ${event.payload}`);
      setTimeout(() => setErrorMessage(null), 5000);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [setErrorMessage]);

  const handleRecognizeCard = useCallback(async (switchTab = false) => {
    if (isRecognizingCard) {
      console.log('[Card Recognition] Already recognizing, skipping...');
      return;
    }

    console.log('[Card Recognition] Starting recognition...');
    if (switchTab) {
      setActiveTab('card');
    }
    setIsRecognizingCard(true);
    setErrorMessage(null);

    try {
      const rawResults = await invoke<any>('recognize_card_at_mouse');
      console.log('[Card Recognition] Raw backend result:', rawResults, typeof rawResults);

      let results: any[] = [];
      if (rawResults) {
        if (Array.isArray(rawResults)) {
          results = rawResults;
        } else if (typeof rawResults === 'string') {
          try {
            const parsed = JSON.parse(rawResults);
            if (Array.isArray(parsed)) results = parsed;
          } catch (e) {
            console.error('[Card Recognition] Failed to parse JSON:', e);
            results = [];
          }
        }
      }

      console.log('[Card Recognition] Parsed results:', results);

      if (results && results.length > 0) {
        const fullInfos: ItemData[] = [];
        for (let i = 0; i < results.length; i++) {
          const res = results[i];
          console.log(`[Card Recognition] Processing result ${i}:`, res);

          if (!res || !res.id) {
            console.warn(`[Card Recognition] Skipping invalid result at index ${i}`);
            continue;
          }

          try {
            const itemInfo = await invoke<ItemData | null>('get_item_info', { id: res.id });
            if (itemInfo) {
              const imgUrl = await getImg(`images/${itemInfo.uuid || itemInfo.name}.webp`);
              const matchLabel = i === 0 ? '✓ Match' : '? Maybe';
              fullInfos.push({
                ...itemInfo,
                displayImg: imgUrl,
                matchLabel,
                matchConfidence: res.confidence,
                matchCount: res.match_count,
              });
              console.log(`[Card Recognition] Added item ${i}:`, itemInfo.name_cn || itemInfo.name);
            } else {
              console.warn(`[Card Recognition] No item info found for id: ${res.id}`);
            }
          } catch (err) {
            console.error(`[Card Recognition] Error fetching item ${res.id}:`, err);
          }
        }

        console.log(`[Card Recognition] Total items loaded: ${fullInfos.length}`);

        if (fullInfos.length > 0) {
          setRecognizedCards(fullInfos);
          setExpandedItems((prev) => {
            const next = new Set(prev);
            fullInfos.forEach((info) => next.add(info.uuid));
            return next;
          });
          showToast(`识别成功: 找到 ${fullInfos.length} 个匹配项 (第1个为最佳匹配)`, 'success');
        } else {
          console.warn('[Card Recognition] No valid items found in database');
          setErrorMessage('识别到了卡牌，但没能在数据库中找到对应信息');
        }
      } else {
        setErrorMessage('未能识别到鼠标下的卡牌。请确保鼠标指向卡牌中心。');
      }
    } catch (e: any) {
      console.error(e);
      setErrorMessage(`卡牌识别执行出错: ${e}`);
    } finally {
      setIsRecognizingCard(false);
      setTimeout(() => setErrorMessage(null), 3000);
    }
  }, [isRecognizingCard, setActiveTab, setErrorMessage, setExpandedItems, showToast]);

  return {
    recognizedCards,
    isRecognizingCard,
    handleRecognizeCard,
  };
};
