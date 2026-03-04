import { useState } from 'react';

export const getCurrencySymbol = (curr: string) => {
  switch (curr) {
    case 'EUR':
      return '€';
    case 'GBP':
      return '£';
    case 'JPY':
      return '¥';
    default:
      return '$';
  }
};

export const formatPrice = (value: number): string => {
  if (!value) return '--';
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
};

export const formatLargeNumber = (value: number): string => {
  if (!value) return '--';
  if (value >= 1e12) return `${(value / 1e12).toFixed(2)}T`;
  if (value >= 1e9) return `${(value / 1e9).toFixed(2)}B`;
  if (value >= 1e6) return `${(value / 1e6).toFixed(2)}M`;
  return new Intl.NumberFormat('en-US').format(value);
};

export const shortenAddress = (value: string): string => {
  if (!value) return 'N/A';
  else if (value.length < 15) return value;
  return value.slice(0, 7) + '....' + value.slice(-7);
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
