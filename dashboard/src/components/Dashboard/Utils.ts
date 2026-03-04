import { useState } from 'react';


export function formatUnixTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toTimeString().slice(0, 8); // "HH:MM:SS"
}

export const formatTimestamp = (ts: number) => {
  const d = new Date(ts * 1000);
  return d.toLocaleString();
};

export const useCopyToClipboard = (
  timeout: number = 1500
): [boolean, (text: string) => void] => {
  const [copied, setCopied] = useState(false);

  const copy = (text: string) => {
    if (!navigator?.clipboard) return;
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), timeout);
  };

  return [copied, copy];
};
