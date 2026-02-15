import { Fragment, useCallback, useState } from 'react';

import type { MonsterSubItem, TierInfo } from '../../types';

export const useTierInfoRenderer = (itemSizes: Record<string, string>) => {
  const [progressionMode, setProgressionMode] = useState<Set<string>>(new Set());

  const renderTierInfo = useCallback((item: MonsterSubItem) => {
    if (!item) return null;
    const isProgressionActive = progressionMode.has(item.name + (item.current_tier || ''));

    const toggleProgression = (e: React.MouseEvent) => {
      e.stopPropagation();
      const key = item.name + (item.current_tier || '');
      const newModes = new Set(progressionMode);
      if (newModes.has(key)) newModes.delete(key);
      else newModes.add(key);
      setProgressionMode(newModes);
    };

    const formatDescription = (text: string) => {
      const parts = text.split(/(\[Locked\]|Quest:)/g);
      return parts.map((part, i) => {
        if (part === '[Locked]') return <span key={i} className="icon-locked" title="Locked">🔒</span>;
        if (part === 'Quest:') return <span key={i} className="icon-quest" title="Quest">📜</span>;
        return part;
      });
    };

    let currentTier = 'bronze';
    const tiers: Record<string, TierInfo | null> = (item as any).tiers || {};

    if (item.current_tier) {
      currentTier = item.current_tier.toLowerCase();
    } else {
      const availableTiers = Object.keys(tiers);
      if (availableTiers.length > 0) {
        if (availableTiers.includes('bronze')) currentTier = 'bronze';
        else if (availableTiers.includes('silver')) currentTier = 'silver';
        else if (availableTiers.includes('gold')) currentTier = 'gold';
        else currentTier = availableTiers[0];
      }
    }

    const tierData = tiers[currentTier];
    const finalData = tierData || Object.values(tiers).find((t) => t !== null);

    const getProgressionText = (line: string, lineIdx: number, field: 'description' | 'extra_description' = 'description') => {
      const tierSequence = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
      const activeTiers = tierSequence
        .map((t) => ({ tier: t, data: tiers[t] }))
        .filter((t) => t.data !== null && t.data !== undefined);

      const numRegex = /(\d+(\.\d+)?%?)/g;
      const matches = [...line.matchAll(numRegex)];

      if (matches.length > 0 && activeTiers.length > 1) {
        let lastIndex = 0;
        const parts: any[] = [];
        matches.forEach((match, mIdx) => {
          const tierValues = activeTiers.map((at) => {
            const fieldData = (at.data as any)[field] || [];
            const atMatches = [...(fieldData[lineIdx] || '').matchAll(numRegex)];
            return atMatches[mIdx] ? atMatches[mIdx][0] : match[0];
          });

          const isConstant = tierValues.every((v) => v === tierValues[0]);
          parts.push(line.substring(lastIndex, match.index));
          if (isConstant) {
            parts.push(match[0]);
          } else {
            parts.push(
              <span key={mIdx} className="progression-inline-values">
                {tierValues.map((val, i) => (
                  <span key={activeTiers[i].tier}>
                    <span className={`val-${activeTiers[i].tier}`}>{val}</span>
                    {i < activeTiers.length - 1 && <span className="upgrade-arrow">»</span>}
                  </span>
                ))}
              </span>
            );
          }
          lastIndex = match.index! + match[0].length;
        });
        parts.push(line.substring(lastIndex));
        return parts;
      }
      return formatDescription(line);
    };

    if (!finalData) {
      const hasSkills = (item as any).skills && (item as any).skills.length > 0;
      let currentTierFallback = 'bronze';
      if (item.current_tier) {
        currentTierFallback = item.current_tier.toLowerCase();
      } else if (item.tier) {
        currentTierFallback = item.tier.toLowerCase().replace(/[+\s]/g, '');
      }

      const borderColorMap: Record<string, string> = {
        bronze: '#CD7F32',
        silver: '#C0C0C0',
        gold: '#FFD700',
        diamond: '#B9F2FF',
        legendary: '#FF4500',
      };
      const borderColor = borderColorMap[currentTierFallback] || borderColorMap.bronze;

      const dbSize = (item.id && itemSizes[item.id]) ? itemSizes[item.id] : item.size;
      const sizeClassFallback = (dbSize || 'Medium').split(' / ')[0].toLowerCase();
      return (
        <div
          className={`sub-item-card tier-${currentTierFallback}`}
          style={{ borderLeft: `4px solid ${borderColor}` }}
        >
          <div className="sub-item-header">
            <div className={`sub-item-img-wrap size-${sizeClassFallback}`} style={{ outline: `2px solid ${borderColor}` }}>
              <img src={item.displayImg} className="sub-item-img" />
            </div>
            <span className="sub-item-name">{item.name}{hasSkills ? '' : ' (无描述)'}</span>
          </div>
          {hasSkills && (
            <div className="sub-item-desc">
              {(item as any).skills.map((skill: any, i: number) => (
                <div key={i} className="desc-line">
                  {formatDescription(skill.cn || skill.en || skill)}
                </div>
              ))}
            </div>
          )}
        </div>
      );
    }

    const borderColorMap: Record<string, string> = {
      bronze: '#CD7F32',
      silver: '#C0C0C0',
      gold: '#FFD700',
      diamond: '#B9F2FF',
      legendary: '#FF4500',
    };
    const borderColor = borderColorMap[currentTier] || borderColorMap.bronze;
    const dbSize = (item.id && itemSizes[item.id]) ? itemSizes[item.id] : item.size;
    const sizeClass = (dbSize || 'Medium').split(' / ')[0].toLowerCase();

    return (
      <div
        className={`sub-item-card tier-${currentTier} ${isProgressionActive ? 'progression-active' : ''}`}
        style={{ borderLeft: `4px solid ${borderColor}` }}
        onClick={toggleProgression}
      >
        <div className="sub-item-header">
          <div className={`sub-item-img-wrap size-${sizeClass}`} style={{ outline: `2px solid ${borderColor}` }}>
            <img src={item.displayImg} className="sub-item-img" />
          </div>
          <div className="sub-item-title-row">
            <span className="sub-item-name">{item.name}</span>
            {(() => {
              const tierSequence = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];
              const activeTiers = tierSequence
                .map((t) => ({ tier: t, data: (item.tiers as any)?.[t] }))
                .filter((t) => t.data !== null && t.data !== undefined);

              if (isProgressionActive && activeTiers.length > 1) {
                const cdValues = activeTiers.map((at) => at.data!.cd || '');
                const isConstant = cdValues.every((v) => v === cdValues[0]);
                if (!cdValues.some((v) => v)) return null;
                if (isConstant) return <div className="sub-item-cd">⏳ {cdValues[0]}</div>;
                return (
                  <div className="sub-item-cd-progression">
                    {cdValues.map((v, i) => (
                      <Fragment key={activeTiers[i].tier}>
                        <div className={`cd-step val-${activeTiers[i].tier}`}>
                          {v.replace('s', '')}
                        </div>
                        {i < activeTiers.length - 1 && <div className="cd-arrow">»</div>}
                      </Fragment>
                    ))}
                    <div className="cd-unit">秒</div>
                  </div>
                );
              } else {
                return finalData.cd && <div className="sub-item-cd">⏳ {finalData.cd}</div>;
              }
            })()}
          </div>
        </div>
        <div className="sub-item-desc">
          {(item as any).skills && (item as any).skills.length > 0 ? (
            (item as any).skills.map((skill: any, i: number) => (
              <div key={i} className="desc-line">
                {isProgressionActive ? formatDescription(skill.cn || skill.en || skill) : formatDescription(skill.cn || skill.en || skill)}
              </div>
            ))
          ) : (
            <>
              {finalData.description.map((d, i) => (
                <div key={i} className="desc-line">
                  {isProgressionActive ? getProgressionText(d, i, 'description') : formatDescription(d)}
                </div>
              ))}
              {finalData.extra_description?.map((d, i) => (
                <div key={`extra-${i}`} className="desc-line extra-desc">
                  {isProgressionActive ? getProgressionText(d, i, 'extra_description') : formatDescription(d)}
                </div>
              ))}
            </>
          )}
        </div>
      </div>
    );
  }, [itemSizes, progressionMode]);

  return { renderTierInfo };
};
