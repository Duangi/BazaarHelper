interface HotkeyCaptureOverlayProps {
  visible: boolean;
  description: string;
  allowMouseButtons?: number[];
  onCapture: (vk: number) => void;
  onCancel: () => void;
}

const buttonToVk: Record<number, number> = {
  0: 1,
  1: 4,
  2: 2,
  3: 5,
  4: 6,
};

export function HotkeyCaptureOverlay({
  visible,
  description,
  allowMouseButtons = [0, 1, 2, 3, 4],
  onCapture,
  onCancel,
}: HotkeyCaptureOverlayProps) {
  if (!visible) return null;

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0,0,0,0.8)',
        zIndex: 9999,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        alignItems: 'center',
        color: '#fff',
      }}
      onMouseDown={(e) => {
        e.preventDefault();
        e.stopPropagation();

        if (!allowMouseButtons.includes(e.button)) return;
        const vk = buttonToVk[e.button] || 0;
        if (vk > 0) onCapture(vk);
      }}
      onKeyDown={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (e.keyCode) onCapture(e.keyCode);
      }}
      tabIndex={0}
      ref={(el) => el?.focus()}
    >
      <div style={{ fontSize: '20px', marginBottom: '10px' }}>请按下新的热键</div>
      <div style={{ fontSize: '14px', color: '#aaa' }}>{description}</div>
      <button
        style={{ marginTop: '20px', padding: '5px 15px' }}
        onClick={(e) => {
          e.stopPropagation();
          onCancel();
        }}
      >
        取消
      </button>
    </div>
  );
}
