// import React from 'react';
import { KEYWORD_COLORS, TIER_COLORS } from '../constants/colors';

export const renderText = (text: any, allTags: string[]) => {
  if (!text) return null;
  
  let content = "";
  if (typeof text === 'string') {
    content = text;
  } else if (text.cn) {
    content = text.cn;
  } else if (text.en) {
    content = text.en;
  } else {
    return null;
  }
  
  const parts = content.split(/(\d+(?:\/\d+)+)/g);
  
  return parts.map((part, i) => {
    if (part.includes('/')) {
      const nums = part.split('/');
      return (
        <span key={i} className="progression-nums">
          {nums.map((n, idx) => {
            let colorIdx = idx;
            if (nums.length === 2) colorIdx = idx + 2;
            else if (nums.length === 3) colorIdx = idx + 1;
            
            return (
              <span key={idx}>
                <span style={{ color: TIER_COLORS[colorIdx] || '#fff', fontWeight: 'bold' }}>{n}</span>
                {idx < nums.length - 1 && <span style={{ color: '#fff' }}>/</span>}
              </span>
            );
          })}
        </span>
      );
    }

    const keywords = Object.keys(KEYWORD_COLORS);
    const allMatches = [...new Set([...keywords, ...allTags])].filter(k => k.length > 0);
    
    if (allMatches.length === 0) return part;
    
    const regex = new RegExp(`(${allMatches.join('|')})`, 'g');
    const subParts = part.split(regex);
    
    return subParts.map((sub, j) => {
      if (KEYWORD_COLORS[sub]) {
        return <span key={j} style={{ color: KEYWORD_COLORS[sub], fontWeight: 'bold' }}>{sub}</span>;
      }
      if (allTags.includes(sub)) {
        return <span key={j} style={{ color: '#8eba31', fontWeight: 'bold' }}>{sub}</span>;
      }
      return sub;
    });
  });
};

export const renderEnchantText = (content: string, allTags: string[]) => {
  if (!content) return null;
  
  const parts = content.split(/(\d+(?:\/\d+)+)/g);
  
  return parts.map((part, i) => {
    if (part.includes('/')) {
      const nums = part.split('/');
      return (
        <span key={i} className="progression-nums">
          {nums.map((n, idx) => {
            let colorIdx = idx;
            if (nums.length === 2) colorIdx = idx + 2;
            else if (nums.length === 3) colorIdx = idx + 1;
            
            const val = parseFloat(n);
            const displayVal = (!isNaN(val) && val > 100) ? (val / 1000).toFixed(1) : n;
            
            return (
              <span key={idx}>
                <span style={{ color: TIER_COLORS[colorIdx] || '#fff', fontWeight: 'bold' }}>{displayVal}</span>
                {idx < nums.length - 1 && <span style={{ color: '#fff' }}>/</span>}
              </span>
            );
          })}
        </span>
      );
    }

    let processedPart = part;
    processedPart = processedPart.replace(/\b(\d{3,})\b/g, (match) => {
      const val = parseInt(match, 10);
      return val > 100 ? (val / 1000).toFixed(1) : match;
    });

    const keywords = Object.keys(KEYWORD_COLORS);
    const allMatches = [...new Set([...keywords, ...allTags])].filter(k => k.length > 0);
    
    if (allMatches.length === 0) return processedPart;
    
    const regex = new RegExp(`(${allMatches.join('|')})`, 'g');
    const subParts = processedPart.split(regex);
    
    return subParts.map((sub, j) => {
      if (KEYWORD_COLORS[sub]) {
        return <span key={j} style={{ color: KEYWORD_COLORS[sub], fontWeight: 'bold' }}>{sub}</span>;
      }
      if (allTags.includes(sub)) {
        return <span key={j} style={{ color: '#8eba31', fontWeight: 'bold' }}>{sub}</span>;
      }
      return sub;
    });
  });
};
