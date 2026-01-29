import React from 'react';
import { MonsterData } from '../types';
import { renderText } from '../utils/renderText';

interface MonsterViewProps {
  manualMonsters: MonsterData[];
  selectedDay: string;
  identifiedNames: string[];
  expandedMonsters: Set<string>;
  allTags: string[];
  onToggleExpand: (name_zh: string) => void;
}

export const MonsterView: React.FC<MonsterViewProps> = ({
  manualMonsters,
  selectedDay,
  identifiedNames,
  expandedMonsters,
  allTags,
  onToggleExpand
}) => {
  // 按 selectedDay 过滤
  const filteredMonsters = manualMonsters.filter(m => {
    if (!selectedDay) return true;
    const avail = m.available || "";
    if (selectedDay === "Day 10+") {
      return avail.includes("10") || avail.includes("11") || avail.includes("12");
    }
    return avail.includes(selectedDay);
  });

  // 按 identifiedNames 排序
  const sortedMonsters = [...filteredMonsters].sort((a, b) => {
    const aIdx = identifiedNames.indexOf(a.name_zh);
    const bIdx = identifiedNames.indexOf(b.name_zh);
    if (aIdx === -1 && bIdx === -1) return 0;
    if (aIdx === -1) return 1;
    if (bIdx === -1) return -1;
    return bIdx - aIdx;
  });

  return (
    <div className="monster-list">
      {sortedMonsters.map((monster) => {
        const isExpanded = expandedMonsters.has(monster.name_zh);
        const wasIdentified = identifiedNames.includes(monster.name_zh);

        return (
          <div
            key={monster.name_zh}
            className={`monster-card ${wasIdentified ? 'identified' : ''}`}
            onClick={() => onToggleExpand(monster.name_zh)}
          >
            <div className="monster-header">
              {monster.displayImgBg && (
                <img src={monster.displayImgBg} alt="" className="monster-bg" />
              )}
              {monster.displayImg && (
                <img src={monster.displayImg} alt="" className="monster-char" />
              )}
              <div className="monster-info">
                <div className="monster-name-row">
                  <span className="monster-name-zh">{monster.name_zh}</span>
                  <span className="monster-name-en">{monster.name}</span>
                </div>
                {monster.available && (
                  <div className="monster-available">可用: {monster.available}</div>
                )}
              </div>
            </div>

            {isExpanded && (
              <div className="monster-details">
                {monster.health && (
                  <div className="monster-health">
                    <strong>生命值:</strong> {renderText(monster.health, allTags)}
                  </div>
                )}

                {monster.skills && monster.skills.length > 0 && (
                  <div className="monster-skills">
                    <strong>技能:</strong>
                    {monster.skills.map((skill, idx) => (
                      <div key={idx} className="skill-item">
                        {skill.displayImg && <img src={skill.displayImg} alt="" />}
                        <div>
                          <div className="skill-name">{skill.name}</div>
                          {Object.entries(skill.tiers).map(([tier, info]) => {
                            if (!info) return null;
                            return (
                              <div key={tier} className="skill-tier">
                                <span className="tier-label">{tier}:</span>
                                {info.description.map((desc, i) => (
                                  <div key={i}>{renderText(desc, allTags)}</div>
                                ))}
                              </div>
                            );
                          })}
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {monster.items && monster.items.length > 0 && (
                  <div className="monster-items">
                    <strong>物品:</strong>
                    {monster.items.map((item, idx) => (
                      <div key={idx} className="item-entry">
                        {item.displayImg && <img src={item.displayImg} alt="" />}
                        <span>{item.name}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};
