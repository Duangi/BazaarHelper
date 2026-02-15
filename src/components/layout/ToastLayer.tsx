import { ToastContainer } from '../Toast';
import type { Toast } from '../../types';

interface ToastLayerProps {
  toasts: Toast[];
  onRemove: (id: number) => void;
}

export function ToastLayer({ toasts, onRemove }: ToastLayerProps) {
  if (toasts.length === 0) return null;
  return <ToastContainer toasts={toasts} onRemove={onRemove} />;
}
