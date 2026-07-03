import type { HistoryItem } from "./types";

export const historyStorageKey = "icon-tracer-web-history-v1";

export function loadHistory(): HistoryItem[] {
  try {
    const raw = localStorage.getItem(historyStorageKey);
    if (!raw) {
      return [];
    }
    return JSON.parse(raw) as HistoryItem[];
  } catch {
    return [];
  }
}

export function fileToDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

export async function dataUrlToBytes(dataUrl: string): Promise<Uint8Array> {
  const response = await fetch(dataUrl);
  return new Uint8Array(await response.arrayBuffer());
}

export function stripExtension(fileName: string): string {
  return fileName.replace(/\.[^/.]+$/, "");
}
