interface ResetHotkeysConfirmModalProps {
  visible: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export function ResetHotkeysConfirmModal({
  visible,
  onCancel,
  onConfirm,
}: ResetHotkeysConfirmModalProps) {
  if (!visible) return null;

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0, 0, 0, 0.85)',
        backdropFilter: 'blur(8px)',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        zIndex: 10000,
        animation: 'fadeIn 0.2s ease',
      }}
    >
      <div
        style={{
          background: 'linear-gradient(135deg, rgba(20, 18, 15, 0.98) 0%, rgba(30, 25, 20, 0.98) 100%)',
          border: '2px solid rgba(255, 205, 25, 0.4)',
          borderRadius: '12px',
          padding: '24px',
          maxWidth: '420px',
          width: '90%',
          boxShadow: '0 16px 48px rgba(0, 0, 0, 0.9), 0 0 32px rgba(255, 205, 25, 0.15)',
          animation: 'scaleIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1)',
        }}
      >
        <div
          style={{
            fontSize: '20px',
            fontWeight: 'bold',
            color: '#ffcd19',
            marginBottom: '16px',
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
            textShadow: '0 2px 8px rgba(255, 205, 25, 0.3)',
          }}
        >
          <span style={{ fontSize: '24px' }}>⚠️</span>
          <span>重置确认</span>
        </div>

        <div
          style={{
            fontSize: '14px',
            color: '#ddd',
            lineHeight: '1.6',
            marginBottom: '24px',
            padding: '16px',
            background: 'rgba(0, 0, 0, 0.3)',
            borderRadius: '8px',
            border: '1px solid rgba(255, 205, 25, 0.1)',
          }}
        >
          <p style={{ margin: '0 0 12px 0' }}>
            此操作将<span style={{ color: '#ff6b6b', fontWeight: 'bold' }}>重置所有快捷键设置</span>，包括：
          </p>
          <ul style={{ margin: '8px 0', paddingLeft: '24px', color: '#aaa' }}>
            <li>怪物识别热键</li>
            <li>卡牌识别热键</li>
            <li>YOLO 扫描热键</li>
            <li>详情显示热键</li>
            <li>折叠/展开热键</li>
          </ul>
          <p style={{ margin: '12px 0 0 0', color: '#888', fontSize: '13px' }}>
            重置后，所有快捷键功能将被禁用，您需要重新设置才能使用。
          </p>
        </div>

        <div
          style={{
            display: 'flex',
            gap: '12px',
            justifyContent: 'flex-end',
          }}
        >
          <button
            style={{
              padding: '10px 24px',
              background: 'rgba(255, 255, 255, 0.1)',
              border: '1px solid rgba(255, 255, 255, 0.2)',
              borderRadius: '6px',
              color: '#fff',
              fontSize: '14px',
              cursor: 'pointer',
              transition: 'all 0.2s ease',
              fontWeight: '500',
            }}
            onClick={onCancel}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'rgba(255, 255, 255, 0.15)';
              e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.3)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)';
              e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.2)';
            }}
          >
            取消
          </button>
          <button
            style={{
              padding: '10px 24px',
              background: 'linear-gradient(135deg, rgba(255, 69, 58, 0.8), rgba(255, 59, 48, 0.9))',
              border: '1px solid rgba(255, 69, 58, 0.6)',
              borderRadius: '6px',
              color: '#fff',
              fontSize: '14px',
              cursor: 'pointer',
              transition: 'all 0.2s ease',
              fontWeight: '600',
              boxShadow: '0 4px 12px rgba(255, 69, 58, 0.3)',
            }}
            onClick={onConfirm}
            onMouseEnter={(e) => {
              e.currentTarget.style.background =
                'linear-gradient(135deg, rgba(255, 79, 68, 0.9), rgba(255, 69, 58, 1))';
              e.currentTarget.style.boxShadow = '0 6px 16px rgba(255, 69, 58, 0.4)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background =
                'linear-gradient(135deg, rgba(255, 69, 58, 0.8), rgba(255, 59, 48, 0.9))';
              e.currentTarget.style.boxShadow = '0 4px 12px rgba(255, 69, 58, 0.3)';
            }}
          >
            确认重置
          </button>
        </div>
      </div>
    </div>
  );
}
