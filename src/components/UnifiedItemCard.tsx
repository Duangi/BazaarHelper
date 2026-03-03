import React from 'react';
import type { ItemData } from '../types';
import { ENCHANT_COLORS, HERO_COLORS } from '../constants/colors';
import { renderText, renderEnchantText } from '../utils/renderText';

interface UnifiedItemCardProps {
  item: ItemData;
  imageSrcOverride?: string;
  allTags: string[];
  isExpanded: boolean;
  isPinned?: boolean;
  isRecognized?: boolean;
  isTopMatch?: boolean;
  showPin?: boolean;
  showExpandChevron?: boolean;
  onToggleExpand?: () => void;
  onPin?: (e: React.MouseEvent) => void;
}

const HERO_AVATAR_MAP: Record<string, string> = {
  Pygmalien: '/images/heroes/pygmalien.webp',
  Jules: '/images/heroes/jules.webp',
  Vanessa: '/images/heroes/vanessa.webp',
  Mak: '/images/heroes/mak.webp',
  Dooley: '/images/heroes/dooley.webp',
  Stelle: '/images/heroes/stelle.webp',
  P: '/images/heroes/pygmalien.webp',
  J: '/images/heroes/jules.webp',
  V: '/images/heroes/vanessa.webp',
  M: '/images/heroes/mak.webp',
  D: '/images/heroes/dooley.webp',
  S: '/images/heroes/stelle.webp'
};

const getMulticastLabel = (raw?: string) => {
  if (!raw || typeof raw !== 'string') return null;
  const vals = raw
    .split('/')
    .map((v) => Number.parseFloat(v.trim()))
    .filter((v) => Number.isFinite(v) && v > 1);
  if (vals.length === 0) return null;
  const min = Math.floor(Math.min(...vals));
  const max = Math.floor(Math.max(...vals));
  if (min === max) return `多重触发：${max}`;
  return `多重触发：${min}-${max}`;
};

const UnifiedItemCardImpl: React.FC<UnifiedItemCardProps> = ({
  item,
  imageSrcOverride,
  allTags,
  isExpanded,
  isPinned = false,
  isRecognized = false,
  isTopMatch = false,
  showPin = false,
  showExpandChevron = true,
  onToggleExpand,
  onPin
}) => {
  const tierClass = (item.tier || '').split(' / ')[0].toLowerCase();
  const tierNameZh = {
    bronze: '青铜+',
    silver: '白银+',
    gold: '黄金+',
    diamond: '钻石+',
    legendary: '传说'
  }[tierClass as 'bronze' | 'silver' | 'gold' | 'diamond' | 'legendary'] || tierClass;

  const sizeClass = item.size?.split(' / ')[0].toLowerCase() || 'medium';

  let rawHero = 'Common';
  if (Array.isArray(item.heroes) && item.heroes.length > 0) {
    rawHero = item.heroes[0];
  } else if (typeof item.heroes === 'string' && item.heroes) {
    rawHero = item.heroes;
  }

  const heroKey = rawHero.split(' / ')[0];
  const heroZhRaw = rawHero.split(' / ')[1] || rawHero;
  const heroZh = heroZhRaw === 'Common' ? '通用' : heroZhRaw;
  const heroColor = HERO_COLORS[heroKey] || undefined;
  const avatar = HERO_AVATAR_MAP[heroKey] || (heroKey.length === 1 ? HERO_AVATAR_MAP[heroKey.toUpperCase()] : undefined);

  const renderTextLocal = (text: any) => renderText(text, allTags || []);
  const renderEnchantTextLocal = (content: string) => renderEnchantText(content, allTags || []);
  const displayImg = imageSrcOverride || item.displayImg;
  const skills = Array.isArray(item.skills) ? item.skills : [];
  const passiveSkills = Array.isArray((item as any).skills_passive) ? (item as any).skills_passive : [];
  const multicastLabel = getMulticastLabel((item as any).multicast_tiers);
  const hasAnyDetail =
    skills.length > 0
    || passiveSkills.length > 0
    || Boolean(multicastLabel)
    || ((item.enchantments || []).length > 0)
    || Boolean(item.description);

  return (
    <div
      className={`item-card-container ${isExpanded ? 'expanded' : ''} ${isRecognized ? 'identified-glow' : ''}`}
      onClick={onToggleExpand}
    >
      <div className={`item-card tier-${tierClass}`}>
        <div className="card-left">
          <div className={`image-box size-${sizeClass}`}>
            <img src={displayImg} alt={item.name} loading="lazy" decoding="async" />
          </div>
        </div>

        <div className="card-center">
          <div className="name-line">
            <span className="name-cn">{item.name_cn}</span>
            {isRecognized && (
              <span className="id-badge" style={{ marginLeft: '4px', backgroundColor: isTopMatch ? '#238636' : '#8b949e' }}>
                {isTopMatch ? 'MATCH' : 'MAYBE'}
              </span>
            )}
            <span className={`tier-label tier-${tierClass}`}>{tierNameZh}</span>
          </div>
          <div className="tags-line">
            {(item.processed_tags || []).slice(0, 3).map((tag) => (
              <span key={tag} className="tag-badge">{tag}</span>
            ))}
          </div>
        </div>

        <div className="card-right">
          <div className="top-right-group">
            <div
              className="toggle-btn hero-btn"
              style={{ width: 32, height: 32, minWidth: 32, minHeight: 32, padding: 0, marginRight: 0, cursor: 'default', border: avatar ? 'none' : undefined }}
              title={heroZh}
            >
              {avatar ? (
                <img src={avatar} alt={heroZh} style={{ width: 28, height: 28, borderRadius: '50%' }} />
              ) : (
                <span style={{ color: heroColor }}>{heroZh}</span>
              )}
            </div>
            {showPin && onPin && (
              <div className={`pin-btn ${isPinned ? 'active' : ''}`} onClick={onPin}>
                {isPinned ? '📌' : '📍'}
              </div>
            )}
          </div>
          {showExpandChevron && <div className="expand-chevron">{isExpanded ? '▴' : '▾'}</div>}
        </div>
      </div>

      {isExpanded && (
        <>
          <div className={`item-details-v2 ${isPinned ? 'progression-active' : ''}`}>
            {(() => {
              try {
                const cdTiersRaw = (item as any).cooldown_tiers;
                const availTiersRaw = (item as any).available_tiers;
                const hasProgression = cdTiersRaw && typeof cdTiersRaw === 'string' && cdTiersRaw.includes('/');

                if (hasProgression) {
                  const cdVals = (cdTiersRaw as string).split('/').map((v: string) => {
                    const ms = parseFloat(v);
                    if (isNaN(ms)) return '0.0';
                    return (ms > 100 ? ms / 1000 : ms).toFixed(1);
                  });
                  const availTiers = (availTiersRaw || '').split('/').map((t: string) => t.toLowerCase().trim());
                  const tierSequence = ['bronze', 'silver', 'gold', 'diamond', 'legendary'];

                  return (
                    <div className="details-left">
                      <div className="sub-item-cd-progression" style={{ position: 'static', background: 'rgba(0,0,0,0.2)', border: '1px solid rgba(255,255,255,0.05)', padding: '4px', borderRadius: '4px', minWidth: '50px' }}>
                        {cdVals.map((v: string, i: number) => {
                          let tierName = 'gold';
                          if (availTiers[i]) {
                            tierName = availTiers[i];
                          } else {
                            if (cdVals.length === 2) tierName = i === 0 ? 'gold' : 'diamond';
                            else tierName = tierSequence[i] || 'gold';
                          }
                          return (
                            <React.Fragment key={i}>
                              <div className={`cd-step val-${tierName}`} style={{ fontSize: '16px' }}>{v}</div>
                              {i < cdVals.length - 1 && <div className="cd-arrow" style={{ transform: 'none', margin: '0' }}>↓</div>}
                            </React.Fragment>
                          );
                        })}
                        <div className="cd-unit">秒</div>
                      </div>
                    </div>
                  );
                }
              } catch (e) {
                console.error('Error rendering CD progression:', e);
              }

              return item.cooldown !== undefined && item.cooldown > 0 && (
                <div className="details-left">
                  <div className="cd-display">
                    <div className="cd-value">{(item.cooldown > 100 ? item.cooldown / 1000 : item.cooldown).toFixed(1)}</div>
                    <div className="cd-unit">秒</div>
                  </div>
                </div>
              );
            })()}

            <div className="details-right">
              {multicastLabel && (
                <div className="skill-item skill-multicast">🔁 {multicastLabel}</div>
              )}
              {skills.map((skill, idx) => (
                <div key={idx} className="skill-item">
                  🗡️ {renderTextLocal(skill)}
                </div>
              ))}
              {passiveSkills.length > 0 && (
                passiveSkills.map((skill: any, idx: number) => (
                  <div key={`passive-${idx}`} className="skill-item skill-passive">
                    ⚙️ {renderTextLocal(skill)}
                  </div>
                ))
              )}
              {!hasAnyDetail && (
                <div className="skill-item skill-empty">暂无详情，稍后可重试。</div>
              )}
            </div>
          </div>

          {(item as any).quests && (
            <div className="quests-section">
              {(Array.isArray((item as any).quests) ? (item as any).quests : [(item as any).quests]).map((quest: any, index: number) => (
                <div key={index} className="quest-item">
                  <div className="quest-header">📜 任务 {index + 1}:</div>
                  {quest.cn_target && <div className="quest-target">→ {renderTextLocal(quest.cn_target || quest.en_target)}</div>}
                  {quest.cn_reward && <div className="quest-reward">✨ {renderTextLocal(quest.cn_reward || quest.en_reward)}</div>}
                </div>
              ))}
            </div>
          )}

          {(item.enchantments || []).length > 0 && (
            <div className="item-enchantments-row">
              {(item.enchantments || []).map((enc, idx) => {
                const parts = enc.split('|');
                if (parts.length > 1) {
                  const name = parts[0];
                  const effect = parts[1];
                  const color = ENCHANT_COLORS[name] || '#ffcd19';
                  return (
                    <div key={idx} className="enchant-item">
                      <span className="enchant-badge" style={{ '--enc-clr': color } as React.CSSProperties}>{name}</span>
                      <span className="enchant-effect">{renderEnchantTextLocal(effect)}</span>
                    </div>
                  );
                }
                return (
                  <div key={idx} className="enchant-item">
                    {renderTextLocal(enc)}
                  </div>
                );
              })}
            </div>
          )}

          {item.description && (
            <div className="item-description-row">
              <div className="description-text">{renderTextLocal(item.description)}</div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export const UnifiedItemCard = React.memo(
  UnifiedItemCardImpl,
  (prev, next) =>
    prev.item === next.item &&
    prev.imageSrcOverride === next.imageSrcOverride &&
    prev.allTags === next.allTags &&
    prev.isExpanded === next.isExpanded &&
    prev.isPinned === next.isPinned &&
    prev.isRecognized === next.isRecognized &&
    prev.isTopMatch === next.isTopMatch &&
    prev.showPin === next.showPin &&
    prev.showExpandChevron === next.showExpandChevron,
);
