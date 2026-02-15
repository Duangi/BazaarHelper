interface ErrorToastProps {
  errorMessage: string | null;
  onClose: () => void;
}

export function ErrorToast({ errorMessage, onClose }: ErrorToastProps) {
  if (!errorMessage) return null;

  return (
    <div
      className="error-toast"
      style={{
        position: 'fixed',
        top: '80px',
        left: '50%',
        transform: 'translateX(-50%)',
        backgroundColor: 'rgba(40, 35, 30, 0.95)',
        color: '#ff6b6b',
        border: '1px solid #ff4d4f',
        padding: '12px 24px',
        borderRadius: '8px',
        boxShadow: '0 8px 24px rgba(0,0,0,0.6)',
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        fontSize: '14px',
        fontWeight: 600,
        backdropFilter: 'blur(5px)',
        animation: 'slideDown 0.3s ease-out',
      }}
    >
      <span style={{ fontSize: '18px' }}>⚠️</span>
      <span>{errorMessage}</span>
      <button
        onClick={onClose}
        style={{
          background: 'transparent',
          border: 'none',
          color: '#888',
          cursor: 'pointer',
          fontSize: '18px',
          marginLeft: '8px',
          lineHeight: 1,
        }}
      >
        ×
      </button>
    </div>
  );
}
