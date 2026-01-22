import { useState, useEffect, useRef } from 'react';

export function useDraggable(initialPos = { x: 100, y: 100 }, storageKey?: string) {
  // 从 localStorage 读取保存的位置
  const getSavedPosition = () => {
    if (!storageKey) return initialPos;
    const saved = localStorage.getItem(storageKey);
    if (saved) {
      try {
        return JSON.parse(saved);
      } catch {
        return initialPos;
      }
    }
    return initialPos;
  };

  const [position, setPosition] = useState(getSavedPosition);
  const isDragging = useRef(false);
  const offset = useRef({ x: 0, y: 0 });

  const handleMouseDown = (e: React.MouseEvent) => {
    // 只有左键可以拖拽
    if (e.button !== 0) return;
    
    // Check if the target or any of its parents is a button or has specific class to prevent drag
    const target = e.target as HTMLElement;
    if (target.tagName === 'BUTTON' || target.closest('button') || target.classList.contains('no-drag')) {
        return;
    }

    isDragging.current = true;
    offset.current = {
      x: e.clientX - position.x,
      y: e.clientY - position.y
    };
    
    // 防止选中文本
    e.preventDefault();
  };

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current) return;
      const newPos = {
        x: e.clientX - offset.current.x,
        y: e.clientY - offset.current.y
      };
      setPosition(newPos);
    };

    const handleMouseUp = (e: MouseEvent) => {
      if (isDragging.current) {
        // 计算最终位置并立即保存
        const finalPos = {
          x: e.clientX - offset.current.x,
          y: e.clientY - offset.current.y
        };
        setPosition(finalPos);
        if (storageKey) {
          localStorage.setItem(storageKey, JSON.stringify(finalPos));
        }
        isDragging.current = false;
      }
    };

    // 绑定到 document 以防止鼠标移出 div 时拖拽失效
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [storageKey]); // 移除position依赖，避免每次position变化都重新绑定事件

  return { position, handleMouseDown, setPosition };
}