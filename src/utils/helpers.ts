import { resolveResource } from "@tauri-apps/api/path";
import { convertFileSrc } from "@tauri-apps/api/core";

const imgCache = new Map<string, string>();

export const getImg = async (path: string | null | undefined): Promise<string> => {
  if (!path) return "";
  if (imgCache.has(path)) return imgCache.get(path)!;
  try {
    const fullPath = await resolveResource(`resources/${path}`);
    const assetUrl = convertFileSrc(fullPath);
    imgCache.set(path, assetUrl);
    return assetUrl;
  } catch {
    return "";
  }
};

export const getHotkeyLabel = (code: number): string => {
  if (code >= 65 && code <= 90) return `Key ${String.fromCharCode(code)}`;
  if (code >= 48 && code <= 57) return `Key ${code - 48}`;
  if (code >= 112 && code <= 123) return `F${code - 111}`;
  
  switch(code) {
    case 1: return "鼠标左键";
    case 2: return "鼠标右键";
    case 4: return "鼠标中键";
    case 5: return "鼠标侧键1 (后退)";
    case 6: return "鼠标侧键2 (前进)";
    case 8: return "BackSpace";
    case 9: return "Tab";
    case 13: return "Enter";
    case 16: return "Shift";
    case 17: return "Ctrl";
    case 18: return "Alt";
    case 20: return "CapsLock";
    case 27: return "Esc";
    case 32: return "Space";
    case 33: return "PageUp";
    case 34: return "PageDown";
    case 35: return "End";
    case 36: return "Home";
    case 37: return "Left";
    case 38: return "Up";
    case 39: return "Right";
    case 40: return "Down";
    case 45: return "Insert";
    case 46: return "Delete";
    case 192: return "~";
    default: return `Unknown (${code})`;
  }
};
