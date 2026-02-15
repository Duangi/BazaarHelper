import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

import type { MonsterData } from '../../types';
import { getImg } from '../../utils/helpers';

interface UseMonsterTabLogicOptions {
  activeTab: string;
  selectedDay: string;
  setSelectedDay: (day: string) => void;
  allMonsters: Record<string, MonsterData>;
  setCurrentDay: (day: number) => void;
  skillsArtMap: Record<string, string>;
  skillsDbFull: Map<string, any>;
  itemsDbFull: Map<string, any>;
  updateDayTabSelection: (day: number) => void;
}

export const useMonsterTabLogic = ({
  activeTab,
  selectedDay,
  setSelectedDay,
  allMonsters,
  setCurrentDay,
  skillsArtMap,
  skillsDbFull,
  itemsDbFull,
  updateDayTabSelection,
}: UseMonsterTabLogicOptions) => {
  const [manualMonsters, setManualMonsters] = useState<MonsterData[]>([]);
  const [identifiedNames, setIdentifiedNames] = useState<string[]>([]);
  const [expandedMonsters, setExpandedMonsters] = useState<Set<string>>(new Set());
  const [isRecognizing, setIsRecognizing] = useState(false);

  const normalizeDay = useCallback((value: string | undefined | null): string => {
    if (!value) return 'Day 1';
    const match = value.match(/Day\s*(\d+)\+?/i);
    if (!match) return 'Day 1';
    const num = Number.parseInt(match[1], 10);
    if (!Number.isFinite(num) || num <= 1) return 'Day 1';
    return num >= 10 ? 'Day 10+' : `Day ${num}`;
  }, []);

  const toggleMonsterExpand = useCallback((nameZh: string) => {
    setExpandedMonsters((prev) => {
      const next = new Set(prev);
      if (next.has(nameZh)) next.delete(nameZh);
      else next.add(nameZh);
      return next;
    });
  }, []);

  const processMonsterImages = useCallback(async (m: MonsterData) => {
    let filename = m.image ? m.image.split('/').pop() || `${m.name_zh}.webp` : `${m.name_zh}.webp`;

    let displayImg = await getImg(`assets/monsters/characters/${filename}`);

    if (!displayImg && m.name_zh.includes(' ')) {
      const spacePos = m.name_zh.lastIndexOf(' ');
      const baseName = m.name_zh.substring(spacePos + 1);
      const fallbackFilename = `${baseName}.webp`;
      const fallbackImg = await getImg(`assets/monsters/characters/${fallbackFilename}`);
      if (fallbackImg) {
        displayImg = fallbackImg;
        filename = fallbackFilename;
      }
    }

    let bgFilename = filename;
    if (m.name_zh === '绿洲守护神') {
      bgFilename = '绿洲守护神_Day9.webp';
    }
    const displayImgBg = await getImg(`assets/monsters/backgrounds/${bgFilename}`);

    return {
      ...m,
      displayImg,
      displayImgBg,
      skills: m.skills
        ? await Promise.all(
            m.skills.map(async (s) => {
              const id = s.id || s.name;
              const fullSkillInfo = id ? skillsDbFull.get(id) : null;

              let mergedSkill = s;
              if (fullSkillInfo) {
                mergedSkill = {
                  ...s,
                  skills: fullSkillInfo.descriptions || s.skills || [],
                  name: fullSkillInfo.name_cn || s.name,
                  name_cn: fullSkillInfo.name_cn,
                  name_en: fullSkillInfo.name_en,
                };
              }

              let imgPath = '';
              try {
                const art = id ? skillsArtMap[id] : undefined;
                if (art) {
                  const base = art.split('/').pop() || art;
                  const nameNoExt = base.replace(/\.[^/.]+$/, '');
                  imgPath = `images/skill/${nameNoExt}.webp`;
                } else {
                  imgPath = `images/${id || s.name}.webp`;
                }
              } catch {
                imgPath = `images/${id || s.name}.webp`;
              }

              return { ...mergedSkill, displayImg: await getImg(imgPath) };
            })
          )
        : [],
      items: m.items
        ? await Promise.all(
            m.items.map(async (i) => {
              const id = i.id || i.name;
              const fullItemInfo = id ? itemsDbFull.get(id) : null;

              const merged = fullItemInfo
                ? {
                    ...i,
                    ...fullItemInfo,
                    current_tier: i.current_tier || fullItemInfo.current_tier,
                    tier: i.tier || fullItemInfo.tier,
                    skills: [...(fullItemInfo.skills || []), ...(fullItemInfo.skills_passive || [])],
                  }
                : i;

              return {
                ...merged,
                displayImg: await getImg(`images/${merged.id || merged.name}.webp`),
              };
            })
          )
        : [],
    };
  }, [itemsDbFull, skillsArtMap, skillsDbFull]);

  const updateFilteredMonsters = useCallback(async (day: string) => {
    let targetDay = normalizeDay(day);
    if (!targetDay && Object.keys(allMonsters).length > 0) {
      targetDay = 'Day 1';
    }

    const monstersOnDay = Object.values(allMonsters).filter(
      (m) =>
        m &&
        typeof m.name_zh === 'string' &&
        m.name_zh.length > 0 &&
        normalizeDay(m.available) === targetDay
    );

    const sorted = [...monstersOnDay].sort((a, b) => {
      const indexA = identifiedNames.indexOf(a.name_zh);
      const indexB = identifiedNames.indexOf(b.name_zh);
      const posA = indexA === -1 ? 999 : indexA;
      const posB = indexB === -1 ? 999 : indexB;
      return posA - posB;
    });

    const processed = await Promise.all(sorted.map(processMonsterImages));
    setManualMonsters(processed as MonsterData[]);
  }, [allMonsters, identifiedNames, normalizeDay, processMonsterImages]);

  useEffect(() => {
    if (activeTab === 'monster') {
      void updateFilteredMonsters(selectedDay);
    }
  }, [activeTab, selectedDay, allMonsters, identifiedNames, updateFilteredMonsters]);

  const handleDayChange = useCallback(async (newDay: number) => {
    if (newDay < 1) return;
    setCurrentDay(newDay);
    updateDayTabSelection(newDay);
    try {
      await invoke('update_day', { day: newDay });
    } catch (e) {
      console.error('更新天数失败:', e);
    }
  }, [setCurrentDay, updateDayTabSelection]);

  const handleAutoRecognition = useCallback(async (day: number | null) => {
    if (isRecognizing) return;
    setIsRecognizing(true);
    try {
      const results = (await invoke('recognize_monsters_from_screenshot', { day })) as any[];
      if (results && results.length > 0) {
        const names = new Array(3).fill('');
        results.forEach((r) => {
          if (r.position >= 1 && r.position <= 3) names[r.position - 1] = r.name;
        });

        const validNames = names.filter((n) => n !== '');
        setIdentifiedNames(validNames);

        setExpandedMonsters((prev) => {
          const next = new Set(prev);
          validNames.forEach((name) => {
            if (allMonsters[name]) next.add(name);
          });
          return next;
        });

        if (validNames.length > 0) {
          const firstMonsterName = validNames[0];
          const monster = allMonsters[firstMonsterName];
          if (monster && monster.available && selectedDay !== monster.available) {
            const normalizedAvailable = normalizeDay(monster.available);
            setSelectedDay(normalizedAvailable);
            const match = normalizedAvailable.match(/Day\s+(\d+)/);
            if (match && match[1]) {
              setCurrentDay(parseInt(match[1]));
            }
          }
        }
      }
    } catch (e) {
      console.error('自动识别失败:', e);
    } finally {
      setIsRecognizing(false);
    }
  }, [allMonsters, isRecognizing, normalizeDay, selectedDay, setCurrentDay, setSelectedDay]);

  return {
    manualMonsters,
    identifiedNames,
    expandedMonsters,
    isRecognizing,
    setIsRecognizing,
    setManualMonsters,
    setIdentifiedNames,
    setExpandedMonsters,
    toggleMonsterExpand,
    handleDayChange,
    handleAutoRecognition,
  };
};
