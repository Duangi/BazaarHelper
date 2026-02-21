import { useCallback, useRef } from 'react';

import type { Dispatch, SetStateAction } from 'react';
import type { ItemData, SyncPayload } from '../../types';
import { getImg } from '../../utils/helpers';

interface UseSyncDataPipelineOptions {
  setSyncData: Dispatch<SetStateAction<SyncPayload & { monster: any[] }>>;
}

export const useSyncDataPipeline = ({ setSyncData }: UseSyncDataPipelineOptions) => {
  const latestSyncSeqRef = useRef(0);
  const lastPayloadSignatureRef = useRef<string>('');

  const processItems = useCallback(async (items: ItemData[]) => {
    return Promise.all(
      items.map(async (i) => ({
        ...i,
        displayImg: await getImg(`images/${i.uuid || i.name}.webp`),
      })),
    );
  }, []);

  const processSyncPayload = useCallback(
    async (payload: SyncPayload) => {
      const signature = JSON.stringify({
        hand: (payload.hand_items || [])
          .map((i) => i.instance_id || i.uuid)
          .sort(),
        stash: (payload.stash_items || [])
          .map((i) => i.instance_id || i.uuid)
          .sort(),
      });

      if (signature === lastPayloadSignatureRef.current) {
        return;
      }
      lastPayloadSignatureRef.current = signature;

      const seq = ++latestSyncSeqRef.current;
      const [hand, stash] = await Promise.all([
        processItems(payload.hand_items || []),
        processItems(payload.stash_items || []),
      ]);

      // Drop stale async updates to avoid visual jitter from out-of-order events.
      if (seq !== latestSyncSeqRef.current) {
        return;
      }

      hand.sort((a, b) => (a.instance_id || a.uuid).localeCompare(b.instance_id || b.uuid));
      stash.sort((a, b) => (a.instance_id || a.uuid).localeCompare(b.instance_id || b.uuid));

      setSyncData((prev) => ({
        ...prev,
        hand_items: hand,
        stash_items: stash,
        all_tags: payload.all_tags || [],
      }));
    },
    [processItems, setSyncData],
  );

  return {
    processItems,
    processSyncPayload,
  };
};
