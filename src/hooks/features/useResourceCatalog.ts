import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { resolveResource } from '@tauri-apps/api/path';

import { resolveResourceUrl } from '../../utils/helpers';

interface ResourceCatalogState {
  hiddenTagIcons: Record<string, string>;
  sponsorIcons: { vx: string; zfb: string };
  skillsArtMap: Record<string, string>;
  itemSizes: Record<string, string>;
  itemsDbFull: Map<string, any>;
  skillsDbFull: Map<string, any>;
}

export const useResourceCatalog = (): ResourceCatalogState => {
  const [hiddenTagIcons, setHiddenTagIcons] = useState<Record<string, string>>({});
  const [sponsorIcons, setSponsorIcons] = useState<{ vx: string; zfb: string }>({ vx: '', zfb: '' });
  const [skillsArtMap, setSkillsArtMap] = useState<Record<string, string>>({});
  const [itemSizes, setItemSizes] = useState<Record<string, string>>({});
  const [itemsDbFull, setItemsDbFull] = useState<Map<string, any>>(new Map());
  const [skillsDbFull, setSkillsDbFull] = useState<Map<string, any>>(new Map());

  useEffect(() => {
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
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const resPath = await resolveResource('resources/skills_db.json');
        const url = convertFileSrc(resPath);
        const resp = await fetch(url);
        const data = await resp.json();
        const map: Record<string, string> = {};

        for (const entry of data) {
          if (entry.id && entry.art_key) {
            const basename = entry.art_key.split('/').pop();
            map[entry.id] = basename;
          }
        }

        setSkillsArtMap(map);
      } catch (error) {
        console.warn('Failed to load skills_db.json for art map', error);
      }
    })();
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const resPath = await resolveResource('resources/items_db.json');
        const url = convertFileSrc(resPath);
        const resp = await fetch(url);
        const data = await resp.json();

        const sizeMap: Record<string, string> = {};
        const fullMap = new Map<string, any>();
        for (const entry of data) {
          if (entry.id) {
            if (entry.size) sizeMap[entry.id] = entry.size;
            fullMap.set(entry.id, entry);
          }
          if (entry.name_cn) fullMap.set(entry.name_cn, entry);
          if (entry.name_en) fullMap.set(entry.name_en, entry);
        }

        setItemSizes(sizeMap);
        setItemsDbFull(fullMap);
      } catch (error) {
        console.warn('Failed to load items_db', error);
      }
    })();
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const resPath = await resolveResource('resources/skills_db.json');
        const url = convertFileSrc(resPath);
        const resp = await fetch(url);
        const data = await resp.json();

        const fullMap = new Map<string, any>();
        for (const entry of data) {
          if (entry.id) fullMap.set(entry.id, entry);
          if (entry.name_cn) fullMap.set(entry.name_cn, entry);
          if (entry.name_en) fullMap.set(entry.name_en, entry);
        }

        setSkillsDbFull(fullMap);
      } catch (error) {
        console.warn('Failed to load skills_db', error);
      }
    })();
  }, []);

  return {
    hiddenTagIcons,
    sponsorIcons,
    skillsArtMap,
    itemSizes,
    itemsDbFull,
    skillsDbFull,
  };
};
