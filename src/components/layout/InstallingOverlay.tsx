interface InstallingOverlayProps {
  visible: boolean;
}

export function InstallingOverlay({ visible }: InstallingOverlayProps) {
  if (!visible) return null;

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: '#292521',
        color: '#ffcd19',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 9999,
      }}
    >
      <div className="version-logo">BH</div>
      <div style={{ fontSize: '18px', marginBottom: '10px' }}>正在启动更新安装程序...</div>
      <div style={{ fontSize: '12px', opacity: 0.7 }}>程序即将自动重启以完成安装</div>
      <div className="loader" style={{ marginTop: '20px' }}></div>
    </div>
  );
}
