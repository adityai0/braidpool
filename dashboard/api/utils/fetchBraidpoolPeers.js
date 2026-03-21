import axios from 'axios';
 
export async function fetchBraidpoolPeers() {
  const url = process.env.BRAIDPOOL_NODE_URL;

  if (!url) {
    console.warn(
      '[fetchBraidpoolPeers] BRAIDPOOL_NODE_URL not set, skipping braidpool peer fetch'
    );
    return null;
  }

  const payload = {
    jsonrpc: '2.0',
    id: 1,
    method: 'getpeerinfo',
    params: [],
  };

  try {
    const response = await axios.post(url, payload, {
      headers: { 'Content-Type': 'application/json' },
      timeout: 5000,
    });

    if (response.data.error) {
      throw new Error(JSON.stringify(response.data.error));
    }

    return response.data.result;
  } catch (error) {
    console.error('[fetchBraidpoolPeers] Failed to fetch braidpool peers:', error.message);
    return null;
  }
}
