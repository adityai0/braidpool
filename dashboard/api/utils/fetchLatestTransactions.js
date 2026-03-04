import axios from 'axios';
import { getCached, setCached } from './cache.js';

export async function fetchLatestTransactions() {
  const cacheKey = 'latestTransactions';
  const cached = getCached(cacheKey);
  if (cached) return cached;
  let retries = 2;
  let lastError;
  while (retries-- > 0) {
    try {
      const response = await axios.get(
        `${process.env.MEMPOOL_URL}/api/mempool/recent`
      );
      setCached(cacheKey, response.data, 10000); // cache for 10 seconds
      return response.data;
    } catch (error) {
      lastError = error;
      if (error.response && error.response.status === 429) {
        const retryAfter = parseInt(
          error.response.headers['retry-after'] || '2',
          10
        );
        await new Promise((res) => setTimeout(res, retryAfter * 1000));
      } else {
        break;
      }
    }
  }
  console.error('Error fetching latest transactions:', lastError);
  throw lastError;
}

// Fetch latest RBF transaction
export async function fetchLatestRBFTransactions() {
  const cacheKey = 'latestRBFTransactions';
  const cached = getCached(cacheKey);
  if (cached) return cached;
  let retries = 2;
  let lastError;
  while (retries-- > 0) {
    try {
      const response = await axios.get(
        `${process.env.MEMPOOL_URL}/api/v1/replacements`
      );
      setCached(cacheKey, response.data, 10000); // cache for 10 seconds
      return response.data;
    } catch (error) {
      lastError = error;
      if (error.response && error.response.status === 429) {
        const retryAfter = parseInt(
          error.response.headers['retry-after'] || '2',
          10
        );
        await new Promise((res) => setTimeout(res, retryAfter * 1000));
      } else {
        break;
      }
    }
  }
  console.error('Error fetching latest RBF transactions:', lastError);
  throw lastError;
}

export async function fetchLatestBlocks() {
  const cacheKey = 'latestBlocks';
  const cached = getCached(cacheKey);
  if (cached) return cached;
  let retries = 2;
  let lastError;
  while (retries-- > 0) {
    try {
      const response = await axios.get(
        `${process.env.MEMPOOL_URL}/api/v1/blocks`
      );
      setCached(cacheKey, response.data, 10000); // cache for 10 seconds
      return response.data;
    } catch (error) {
      lastError = error;
      if (error.response && error.response.status === 429) {
        const retryAfter = parseInt(
          error.response.headers['retry-after'] || '2',
          10
        );
        await new Promise((res) => setTimeout(res, retryAfter * 1000));
      } else {
        break;
      }
    }
  }
  console.error('Error fetching latest blocks:', lastError);
  throw lastError;
}

export async function fetchBlockDetailsByHash(hash) {
  try {
    const response = await axios.get(
      `${process.env.MEMPOOL_URL}/api/block/${hash}`
    );
    return response.data;
  } catch (error) {
    console.error('Error fetching block details:', error);
    throw error;
  }
}

export async function fetchTxInfo(txid) {
  try {
    const response = await axios.get(
      `${process.env.MEMPOOL_URL}/api/tx/${txid}`
    );
    return response.data;
  } catch (error) {
    console.error('Error fetching transaction info:', error);
    throw error;
  }
}
