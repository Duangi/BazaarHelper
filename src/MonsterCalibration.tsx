import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

const MonsterCalibration: React.FC = () => {
  const [windowSize, setWindowSize] = useState({ width: 800, height: 600 });
  const [regions, setRegions] = useState<Region[]>([]);
  const [dragging, setDragging] = useState<{ index: number; offsetX: number; offsetY: number } | null>(null);
  const [resizing, setResizing] = useState<{ index: number; edge: string; startX: number; startY: number } | null>(null);

  // 获取当前窗口大小并初始化区域
  useEffect(() => {
    const getWindowSize = async () => {
      const window = getCurrentWindow();
      const size = await window.innerSize();
      const w = size.width;
      const h = size.height;
      
      console.log('[Monster Calibration] Window size:', w, h);
      setWindowSize({ width: w, height: h });
      
      // 根据窗口大小初始化三个区域（均匀分布）
      const regionWidth = Math.floor(w * 0.2);  // 每个区域占20%宽度
      const regionHeight = Math.floor(h * 0.4); // 每个区域占40%高度
      const spacing = (w - regionWidth * 3) / 4; // 计算间距
      const topMargin = Math.floor(h * 0.3);    // 距离顶部30%
      
      setRegions([
        { x: spacing, y: topMargin, width: regionWidth, height: regionHeight },
        { x: spacing * 2 + regionWidth, y: topMargin, width: regionWidth, height: regionHeight },
        { x: spacing * 3 + regionWidth * 2, y: topMargin, width: regionWidth, height: regionHeight },
      ]);
    };
    getWindowSize();
  }, []);

  // 关闭窗口
  const handleClose = async () => {
    try {
      await invoke('close_calibration_window');
    } catch (err) {
      console.error('[Monster Calibration] Failed to close window:', err);
    }
  };

  // ESC键退出
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // 处理鼠标按下
  const handleMouseDown = (e: React.MouseEvent, index: number, edge?: string) => {
    e.preventDefault();
    e.stopPropagation();
    
    if (edge) {
      // 开始调整大小
      setResizing({
        index,
        edge,
        startX: e.clientX,
        startY: e.clientY,
      });
    } else {
      // 开始拖动
      const region = regions[index];
      setDragging({
        index,
        offsetX: e.clientX - region.x,
        offsetY: e.clientY - region.y,
      });
    }
  };

  // 处理鼠标移动
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (dragging !== null) {
        const newX = Math.max(0, Math.min(windowSize.width - regions[dragging.index].width, e.clientX - dragging.offsetX));
        const newY = Math.max(0, Math.min(windowSize.height - regions[dragging.index].height, e.clientY - dragging.offsetY));
        
        setRegions(prev => {
          const newRegions = [...prev];
          newRegions[dragging.index] = { ...newRegions[dragging.index], x: newX, y: newY };
          return newRegions;
        });
      }
      
      if (resizing !== null) {
        const { index, edge, startX, startY } = resizing;
        const deltaX = e.clientX - startX;
        const deltaY = e.clientY - startY;
        
        setRegions(prev => {
          const newRegions = [...prev];
          const region = { ...newRegions[index] };
          
          if (edge.includes('right')) {
            region.width = Math.max(50, region.width + deltaX);
          }
          if (edge.includes('bottom')) {
            region.height = Math.max(50, region.height + deltaY);
          }
          if (edge.includes('left')) {
            const newWidth = Math.max(50, region.width - deltaX);
            if (newWidth > 50) {
              region.x = region.x + deltaX;
              region.width = newWidth;
            }
          }
          if (edge.includes('top')) {
            const newHeight = Math.max(50, region.height - deltaY);
            if (newHeight > 50) {
              region.y = region.y + deltaY;
              region.height = newHeight;
            }
          }
          
          newRegions[index] = region;
          return newRegions;
        });
        
        setResizing({ ...resizing, startX: e.clientX, startY: e.clientY });
      }
    };

    const handleMouseUp = () => {
      setDragging(null);
      setResizing(null);
    };

    if (dragging !== null || resizing !== null) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
      return () => {
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [dragging, resizing, windowSize, regions]);

  // 保存校准数据
  const handleSave = async () => {
    try {
      // 获取游戏窗口信息用于保存
      const gameWindowInfo = await invoke<any>('get_game_window_info');
      
      const calibration = {
        regions,
        game_window_width: gameWindowInfo.width,
        game_window_height: gameWindowInfo.height,
        screen_width: window.screen.width,
        screen_height: window.screen.height,
      };
      
      await invoke('save_monster_calibration', { calibration });
      console.log('[Monster Calibration] Saved:', calibration);
      
      // 保存成功后关闭窗口
      await handleClose();
    } catch (err) {
      console.error('[Monster Calibration] Failed to save:', err);
      alert('保存失败: ' + err);
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        left: 0,
        top: 0,
        width: `${windowSize.width}px`,
        height: `${windowSize.height}px`,
        background: 'rgba(0, 0, 0, 0.5)',
        cursor: 'default',
        overflow: 'hidden',
      }}
    >
      {/* 说明文字 */}
      <div style={{
        position: 'absolute',
        top: 20,
        left: '50%',
        transform: 'translateX(-50%)',
        background: 'rgba(20, 18, 15, 0.95)',
        border: '2px solid rgba(255, 205, 25, 0.6)',
        borderRadius: '12px',
        padding: '20px 30px',
        color: '#fff',
        textAlign: 'center',
        boxShadow: '0 8px 24px rgba(0, 0, 0, 0.8)',
      }}>
        <div style={{ fontSize: '20px', fontWeight: 'bold', color: '#ffcd19', marginBottom: '12px' }}>
          野怪识别校准
        </div>
        <div style={{ fontSize: '14px', color: '#ddd', marginBottom: '8px' }}>
          拖动并调整三个方框的大小，使其覆盖三个野怪区域
        </div>
        <div style={{ fontSize: '12px', color: '#aaa' }}>
          提示：按 ESC 键取消，点击确认按钮保存
        </div>
      </div>

      {/* 三个可调整的方框 */}
      {regions.length === 0 && (
        <div style={{
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          color: '#fff',
          fontSize: '18px',
          background: 'rgba(0, 0, 0, 0.8)',
          padding: '20px 40px',
          borderRadius: '8px',
        }}>
          正在初始化校准区域...
        </div>
      )}
      {regions.map((region, index) => (
        <div
          key={index}
          style={{
            position: 'absolute',
            left: region.x,
            top: region.y,
            width: region.width,
            height: region.height,
            border: '3px solid #ffcd19',
            background: 'rgba(255, 205, 25, 0.1)',
            boxShadow: '0 0 20px rgba(255, 205, 25, 0.5)',
            cursor: 'move',
          }}
          onMouseDown={(e) => handleMouseDown(e, index)}
        >
          {/* 标签 */}
          <div style={{
            position: 'absolute',
            top: -30,
            left: 0,
            background: '#ffcd19',
            color: '#000',
            padding: '4px 12px',
            borderRadius: '4px',
            fontSize: '14px',
            fontWeight: 'bold',
            pointerEvents: 'none',
          }}>
            区域 {index + 1}
          </div>

          {/* 调整大小的手柄 */}
          {['top', 'bottom', 'left', 'right', 'top-left', 'top-right', 'bottom-left', 'bottom-right'].map(edge => (
            <div
              key={edge}
              style={{
                position: 'absolute',
                ...(edge === 'top' && { top: -5, left: '50%', transform: 'translateX(-50%)', width: 40, height: 10, cursor: 'ns-resize' }),
                ...(edge === 'bottom' && { bottom: -5, left: '50%', transform: 'translateX(-50%)', width: 40, height: 10, cursor: 'ns-resize' }),
                ...(edge === 'left' && { left: -5, top: '50%', transform: 'translateY(-50%)', width: 10, height: 40, cursor: 'ew-resize' }),
                ...(edge === 'right' && { right: -5, top: '50%', transform: 'translateY(-50%)', width: 10, height: 40, cursor: 'ew-resize' }),
                ...(edge === 'top-left' && { top: -5, left: -5, width: 15, height: 15, cursor: 'nwse-resize' }),
                ...(edge === 'top-right' && { top: -5, right: -5, width: 15, height: 15, cursor: 'nesw-resize' }),
                ...(edge === 'bottom-left' && { bottom: -5, left: -5, width: 15, height: 15, cursor: 'nesw-resize' }),
                ...(edge === 'bottom-right' && { bottom: -5, right: -5, width: 15, height: 15, cursor: 'nwse-resize' }),
                background: '#ffcd19',
                borderRadius: '50%',
              }}
              onMouseDown={(e) => handleMouseDown(e, index, edge)}
            />
          ))}
        </div>
      ))}

      {/* 底部按钮 */}
      <div style={{
        position: 'absolute',
        bottom: 30,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        gap: '16px',
      }}>
        <button
          onClick={handleClose}
          style={{
            padding: '12px 32px',
            background: 'rgba(255, 255, 255, 0.1)',
            border: '2px solid rgba(255, 255, 255, 0.3)',
            borderRadius: '8px',
            color: '#fff',
            fontSize: '16px',
            fontWeight: 'bold',
            cursor: 'pointer',
            transition: 'all 0.2s',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = 'rgba(255, 255, 255, 0.2)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)';
          }}
        >
          取消 (ESC)
        </button>
        <button
          onClick={handleSave}
          style={{
            padding: '12px 32px',
            background: 'linear-gradient(135deg, #ffcd19, #d4af37)',
            border: '2px solid #ffcd19',
            borderRadius: '8px',
            color: '#000',
            fontSize: '16px',
            fontWeight: 'bold',
            cursor: 'pointer',
            transition: 'all 0.2s',
            boxShadow: '0 4px 12px rgba(255, 205, 25, 0.4)',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = 'linear-gradient(135deg, #ffd700, #ffcd19)';
            e.currentTarget.style.boxShadow = '0 6px 16px rgba(255, 205, 25, 0.6)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'linear-gradient(135deg, #ffcd19, #d4af37)';
            e.currentTarget.style.boxShadow = '0 4px 12px rgba(255, 205, 25, 0.4)';
          }}
        >
          确认保存
        </button>
      </div>
    </div>
  );
};

export default MonsterCalibration;
