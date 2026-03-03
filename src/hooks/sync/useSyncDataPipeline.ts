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
  const processedItemCacheRef = useRef<Map<string, ItemData>>(new Map());
  const itemDigestCacheRef = useRef<Map<string, string>>(new Map());

  const getItemKey = useCallback((item: ItemData) => item.instance_id || item.uuid, []);

  const getItemDigest = useCallback((item: ItemData) => {
    const heroes = Array.isArray(item.heroes) ? item.heroes.join('|') : String(item.heroes || '');
    const enchantments = Array.isArray(item.enchantments) ? item.enchantments.join('|') : '';
    const skillCount = Array.isArray(item.skills) ? item.skills.length : 0;

    return [
      item.uuid,
      item.instance_id || '',
      item.name,
      item.name_cn,
      item.tier,
      item.available_tiers,
      item.size || '',
      item.cooldown_tiers,
      item.damage_tiers,
      item.heal_tiers,
      item.shield_tiers,
      item.ammo_tiers,
      item.crit_tiers,
      item.multicast_tiers,
      item.burn_tiers,
      item.poison_tiers,
      item.regen_tiers,
      item.lifesteal_tiers,
      heroes,
      enchantments,
      `${skillCount}`,
      item.description || '',
    ].join('::');
  }, []);

  const processItems = useCallback(async (items: ItemData[]) => {
    const nextKeys = new Set<string>();
    const processed = await Promise.all(
      items.map(async (item) => {
        const key = getItemKey(item);
        const digest = getItemDigest(item);
        nextKeys.add(key);

        const cachedDigest = itemDigestCacheRef.current.get(key);
        const cachedItem = processedItemCacheRef.current.get(key);

        if (cachedItem && cachedDigest === digest) {
          return cachedItem;
        }

        const normalized: ItemData = {
          ...item,
          displayImg: await getImg(`images/${item.uuid || item.name}.webp`),
        };

        processedItemCacheRef.current.set(key, normalized);
        itemDigestCacheRef.current.set(key, digest);
        return normalized;
      }),
    );

    if (processedItemCacheRef.current.size > 512) {
      for (const key of [...processedItemCacheRef.current.keys()]) {
        if (!nextKeys.has(key)) {
          processedItemCacheRef.current.delete(key);
          itemDigestCacheRef.current.delete(key);
        }
        if (processedItemCacheRef.current.size <= 400) break;
      }
    }

    return processed;
  }, [getItemDigest, getItemKey]);

  const areListsEquivalent = useCallback((left: ItemData[], right: ItemData[]) => {
    if (left.length !== right.length) return false;
    for (let i = 0; i < left.length; i += 1) {
      const l = left[i];
      const r = right[i];
      if (l === r) continue;
      if ((l.instance_id || l.uuid) !== (r.instance_id || r.uuid)) return false;
      if (l.displayImg !== r.displayImg) return false;
      if (getItemDigest(l) !== getItemDigest(r)) return false;
    }
    return true;
  }, [getItemDigest]);

  const areTagsEquivalent = useCallback((left: string[], right: string[]) => {
    if (left.length !== right.length) return false;
    for (let i = 0; i < left.length; i += 1) {
      if (left[i] !== right[i]) return false;
    }
    return true;
  }, []);

  const processSyncPayload = useCallback(
    async (payload: SyncPayload) => {
      const signature = JSON.stringify({
        hand: (payload.hand_items || [])
          .map((i) => `${getItemKey(i)}:${getItemDigest(i)}`)
          .sort(),
        stash: (payload.stash_items || [])
          .map((i) => `${getItemKey(i)}:${getItemDigest(i)}`)
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

      setSyncData((prev) => {
        const nextHand = areListsEquivalent(prev.hand_items || [], hand) ? (prev.hand_items || []) : hand;
        const nextStash = areListsEquivalent(prev.stash_items || [], stash) ? (prev.stash_items || []) : stash;
        const nextTags = areTagsEquivalent(prev.all_tags || [], payload.all_tags || [])
          ? prev.all_tags || []
          : payload.all_tags || [];

        if (nextHand === prev.hand_items && nextStash === prev.stash_items && nextTags === prev.all_tags) {
          return prev;
        }

        return {
          ...prev,
          hand_items: nextHand,
          stash_items: nextStash,
          all_tags: nextTags,
        };
      });
    },
    [areListsEquivalent, areTagsEquivalent, getItemDigest, getItemKey, processItems, setSyncData],
  );

  return {
    processItems,
    processSyncPayload,
  };
};
