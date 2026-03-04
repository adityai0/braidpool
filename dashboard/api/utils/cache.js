const cache = {};

export function getCached(key) {
  const entry = cache[key];
  if (!entry) return null;
  if (Date.now() > entry.expiry) {
    delete cache[key];
    return null;
  }
  return entry.value;
}

export function setCached(key, value, ttlMs) {
  cache[key] = {
    value,
    expiry: Date.now() + ttlMs,
  };
}
