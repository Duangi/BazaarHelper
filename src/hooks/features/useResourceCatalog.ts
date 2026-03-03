import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

import { resolveResourceUrl } from '../../utils/helpers';

interface ResourceCatalogState {
  hiddenTagIcons: Record<string, string>;
  sponsorIcons: { vx: string; zfb: string };
  skillsArtMap: Record<string, string>;
  itemSizes: Record<string, string>;
}

interface ResourceCatalogMaps {
  skills_art_map: Record<string, string>;
  item_sizes: Record<string, string>;
}

export const useResourceCatalog = (enabled = true): ResourceCatalogState => {
  const [hiddenTagIcons, setHiddenTagIcons] = useState<Record<string, string>>({});
  const [sponsorIcons, setSponsorIcons] = useState<{ vx: string; zfb: string }>({ vx: '', zfb: '' });
  const [skillsArtMap, setSkillsArtMap] = useState<Record<string, string>>({});
  const [itemSizes, setItemSizes] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!enabled) return;

    (async () => {
      const iconNames = [
        'Ammo', 'Burn', 'Charge', 'Cooldown', 'CritChance', 'Damage', 'Income',
        'Flying', 'Freeze', 'Haste', 'Health', 'MaxHPHeart', 'Lifesteal', 'Poison',
        'Regen', 'Shield', 'Slowness',
      ];

      const icons: Record<string, string> = {};
      for (const name of iconNames) {
        try {
          const url = await resolveResourceUrl(`assets/gui/${name}.webp`);
          icons[name] = url;
        } catch (error) {
          console.error(`Failed to load icon ${name}:`, error);
        }
      }
      setHiddenTagIcons(icons);

      try {
        setSponsorIcons({
          vx: await resolveResourceUrl('assets/sponsor/vx.png'),
          zfb: await resolveResourceUrl('assets/sponsor/zfb.png'),
        });
      } catch (error) {
        console.error('Failed to load sponsor icons', error);
      }
    })();
  }, [enabled]);

  useEffect(() => {
    if (!enabled) return;

    (async () => {
      try {
        const maps = await invoke<ResourceCatalogMaps>('get_resource_catalog_maps');
        setSkillsArtMap(maps.skills_art_map || {});
        setItemSizes(maps.item_sizes || {});
      } catch (error) {
        console.warn('Failed to load resource catalog maps', error);
      }
    })();
  }, [enabled]);

  return {
    hiddenTagIcons,
    sponsorIcons,
    skillsArtMap,
    itemSizes,
  };
};
