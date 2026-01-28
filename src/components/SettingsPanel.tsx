// 可折叠设置分组组件
import React from 'react';

interface SettingGroupProps {
  title: string;
  expanded: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

export const SettingGroup: React.FC<SettingGroupProps> = ({ title, expanded, onToggle, children }) => {
  return (
    <div style={{ marginBottom: '12px' }}>
      <div 
        onClick={onToggle}
        style={{
          padding: '12px 16px',
          background: 'linear-gradient(135deg, rgba(255, 205, 25, 0.15), rgba(255, 180, 25, 0.1))',
          border: '1px solid rgba(255, 205, 25, 0.3)',
          borderRadius: '8px',
          cursor: 'pointer',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          transition: 'all 0.2s ease',
          userSelect: 'none'
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 205, 25, 0.2), rgba(255, 180, 25, 0.15))';
          e.currentTarget.style.borderColor = 'rgba(255, 205, 25, 0.5)';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255, 205, 25, 0.15), rgba(255, 180, 25, 0.1))';
          e.currentTarget.style.borderColor = 'rgba(255, 205, 25, 0.3)';
        }}
      >
        <span style={{ 
          fontSize: '15px', 
          fontWeight: 'bold', 
          color: '#ffcd19',
          textShadow: '0 2px 4px rgba(0, 0, 0, 0.3)'
        }}>
          {title}
        </span>
        <span style={{ 
          fontSize: '18px', 
          color: '#ffcd19',
          transform: expanded ? 'rotate(180deg)' : 'rotate(0deg)',
          transition: 'transform 0.3s ease'
        }}>
          ▼
        </span>
      </div>
      {expanded && (
        <div style={{
          padding: '16px',
          background: 'rgba(0, 0, 0, 0.2)',
          border: '1px solid rgba(255, 205, 25, 0.15)',
          borderTop: 'none',
          borderRadius: '0 0 8px 8px',
          display: 'flex',
          flexDirection: 'column',
          gap: '15px',
          animation: 'slideDown 0.3s ease'
        }}>
          {children}
        </div>
      )}
    </div>
  );
};
