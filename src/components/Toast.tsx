import React, { useEffect, useRef } from 'react';
import { Toast as ToastType } from '../types';

interface ToastContainerProps {
  toasts: ToastType[];
  onRemove: (id: number) => void;
}

export const ToastContainer: React.FC<ToastContainerProps> = ({ toasts, onRemove }) => {
  return (
    <div className="toast-container">
      {toasts.map(toast => (
        <ToastItem key={toast.id} toast={toast} onRemove={onRemove} />
      ))}
    </div>
  );
};

interface ToastItemProps {
  toast: ToastType;
  onRemove: (id: number) => void;
}

const ToastItem: React.FC<ToastItemProps> = ({ toast, onRemove }) => {
  const onRemoveRef = useRef(onRemove);

  useEffect(() => {
    onRemoveRef.current = onRemove;
  }, [onRemove]);

  useEffect(() => {
    const timer = setTimeout(() => {
      onRemoveRef.current(toast.id);
    }, 3000);
    return () => clearTimeout(timer);
  }, [toast.id]);

  return (
    <div className={`toast toast-${toast.type}`}>
      {toast.message}
    </div>
  );
};
