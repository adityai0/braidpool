import { fetchBraidpoolPeers } from '../fetchBraidpoolPeers.js';
import axios from 'axios';

jest.mock('axios');

describe('fetchBraidpoolPeers', () => {
  const originalEnv = process.env;

  beforeEach(() => {
    jest.resetModules();
    process.env = { ...originalEnv };
  });

  afterEach(() => {
    process.env = originalEnv;
    jest.clearAllMocks();
  });

  it('returns null when BRAIDPOOL_NODE_URL is not set', async () => {
    delete process.env.BRAIDPOOL_NODE_URL;

    const result = await fetchBraidpoolPeers();

    expect(result).toBeNull();
    expect(axios.post).not.toHaveBeenCalled();
  });

  it('calls RPC with JSON-RPC 2.0 and returns peer info when URL is set', async () => {
    process.env.BRAIDPOOL_NODE_URL = 'http://localhost:6682';

    const mockPeerInfo = {
      total_peers: 5,
      connected: 3,
      inbound: 1,
      outbound: 2,
      network_groups: 2,
      avg_latency_ms: 42.5,
      peers: [
        {
          peer_id: '12D3KooWTest1',
          ip: '192.168.1.1',
          inbound: false,
          latency_ms: 30,
          score: 80,
          last_seen_secs: 5,
          geo_group: 'US',
        },
      ],
    };

    axios.post.mockResolvedValue({ data: { result: mockPeerInfo } });

    const result = await fetchBraidpoolPeers();

    expect(axios.post).toHaveBeenCalledWith(
      'http://localhost:6682',
      {
        jsonrpc: '2.0',
        id: 1,
        method: 'getpeerinfo',
        params: [],
      },
      expect.objectContaining({
        headers: { 'Content-Type': 'application/json' },
      })
    );
    expect(result).toEqual(mockPeerInfo);
  });

  it('returns null when RPC call fails', async () => {
    process.env.BRAIDPOOL_NODE_URL = 'http://localhost:6682';

    axios.post.mockRejectedValue(new Error('Connection refused'));

    const result = await fetchBraidpoolPeers();

    expect(result).toBeNull();
  });

  it('returns null when RPC returns an error response', async () => {
    process.env.BRAIDPOOL_NODE_URL = 'http://localhost:6682';

    axios.post.mockResolvedValue({
      data: { error: { code: -32600, message: 'Invalid request' } },
    });

    const result = await fetchBraidpoolPeers();

    expect(result).toBeNull();
  });
});
