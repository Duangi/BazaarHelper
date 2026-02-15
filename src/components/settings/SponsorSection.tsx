import React from 'react';

interface SponsorSectionProps {
  sponsorIcons: { vx: string; zfb: string };
}

export const SponsorSection: React.FC<SponsorSectionProps> = ({ sponsorIcons }) => {
  return (
    <div className="setting-item" style={{ marginTop: '20px', textAlign: 'center' }}>
      <label style={{ display: 'block', marginBottom: '12px', color: '#ffcd19', fontSize: '14px', fontWeight: 'bold' }}>
        赞助与支持 (Sponsor)
      </label>
      <div style={{ display: 'flex', justifyContent: 'space-around', alignItems: 'flex-start', flexWrap: 'wrap', gap: '20px' }}>
        {sponsorIcons.vx && (
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
            <img src={sponsorIcons.vx} alt="WeChat" style={{ width: '180px', borderRadius: '8px', boxShadow: '0 4px 12px rgba(0,0,0,0.5)' }} />
            <span style={{ fontSize: '12px', color: '#888' }}>微信 (WeChat)</span>
          </div>
        )}
        {sponsorIcons.zfb && (
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
            <img src={sponsorIcons.zfb} alt="Alipay" style={{ width: '180px', borderRadius: '8px', boxShadow: '0 4px 12px rgba(0,0,0,0.5)' }} />
            <span style={{ fontSize: '12px', color: '#888' }}>支付宝 (Alipay)</span>
          </div>
        )}
      </div>
      <div style={{ fontSize: '11px', color: '#666', marginTop: '12px' }}>
        如果这个工具对你有帮助，欢迎请作者喝杯咖啡 ☕
      </div>
    </div>
  );
};
