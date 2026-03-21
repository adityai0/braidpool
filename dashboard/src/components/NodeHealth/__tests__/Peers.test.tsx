import { render, screen, fireEvent } from '@testing-library/react';
import Peers from '../Peers';
import { PeerInfo, BraidpoolPeerInfo } from '../Types';

const mockBitcoinPeers: PeerInfo[] = Array.from({ length: 12 }).map((_, i) => ({
  id: i,
  addr: `192.168.0.${i}`,
  version: 70015,
  subver: '/Satoshi:0.21.0/',
  inbound: i % 2 === 0,
  startingheight: 100,
  synced_headers: 100,
  synced_blocks: 100,
  pingtime: 30 + i,
  bytessent: 1048576 * (i + 1),
  bytesrecv: 524288 * (i + 1),
}));

const mockBraidpoolPeerInfo: BraidpoolPeerInfo = {
  total_peers: 8,
  connected: 6,
  inbound: 2,
  outbound: 4,
  network_groups: 3,
  avg_latency_ms: 45.5,
  peers: Array.from({ length: 6 }).map((_, i) => ({
    peer_id: `12D3KooW${i}abcdefghij`,
    ip: `10.0.0.${i}`,
    inbound: i % 3 === 0,
    latency_ms: 20 + i * 10,
    score: 50 + i * 5,
    last_seen_secs: i * 5,
    geo_group: i % 2 === 0 ? 'US' : 'EU',
  })),
};

describe('Peers Component', () => {
  test('renders summary stats cards', () => {
    render(
      <Peers
        bitcoinPeers={mockBitcoinPeers}
        braidpoolPeerInfo={mockBraidpoolPeerInfo}
      />
    );

    // Summary cards should show peer counts
    expect(screen.getByText('Bitcoin Peers')).toBeInTheDocument();
    expect(screen.getByText('Braidpool Peers')).toBeInTheDocument();
    expect(screen.getByText('12')).toBeInTheDocument(); // Bitcoin peer count
    expect(screen.getByText('6')).toBeInTheDocument(); // Braidpool connected count
  });

  test('renders both Bitcoin and Braidpool panels side by side', () => {
    render(
      <Peers
        bitcoinPeers={mockBitcoinPeers}
        braidpoolPeerInfo={mockBraidpoolPeerInfo}
      />
    );

    // Both network panels should be visible
    expect(screen.getByText('Bitcoin Network')).toBeInTheDocument();
    expect(screen.getByText('Braidpool Network')).toBeInTheDocument();
  });

  test('shows bitcoin peer addresses', () => {
    render(
      <Peers
        bitcoinPeers={mockBitcoinPeers}
        braidpoolPeerInfo={mockBraidpoolPeerInfo}
      />
    );

    // Check if bitcoin peers are shown (5 per page)
    const peerAddresses = screen.getAllByText(/192.168.0./i);
    expect(peerAddresses).toHaveLength(5);
  });

  test('shows braidpool peer IDs', () => {
    render(
      <Peers
        bitcoinPeers={mockBitcoinPeers}
        braidpoolPeerInfo={mockBraidpoolPeerInfo}
      />
    );

    // Check braidpool peers are displayed
    const braidpoolPeers = screen.getAllByText(/12D3KooW/i);
    expect(braidpoolPeers).toHaveLength(5); // 5 per page
  });

  test('shows inbound/outbound labels for peers', () => {
    render(
      <Peers
        bitcoinPeers={mockBitcoinPeers}
        braidpoolPeerInfo={mockBraidpoolPeerInfo}
      />
    );

    // IN and OUT labels should be present
    expect(screen.getAllByText(/↓ IN/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/↑ OUT/i).length).toBeGreaterThan(0);
  });

  test('shows network groups and avg latency stats', () => {
    render(
      <Peers
        bitcoinPeers={mockBitcoinPeers}
        braidpoolPeerInfo={mockBraidpoolPeerInfo}
      />
    );

    expect(screen.getByText('Network Groups')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument(); // network groups count
    expect(screen.getByText('Avg Latency')).toBeInTheDocument();
    expect(screen.getByText('46ms')).toBeInTheDocument(); // avg latency rounded
  });

  test('shows not connected message when braidpool is null', () => {
    render(<Peers bitcoinPeers={mockBitcoinPeers} braidpoolPeerInfo={null} />);

    expect(screen.getByText('Node Not Connected')).toBeInTheDocument();
    expect(screen.getByText(/Configure BRAIDPOOL_NODE_URL/i)).toBeInTheDocument();
  });

  test('shows dashes for stats when braidpool not connected', () => {
    render(<Peers bitcoinPeers={mockBitcoinPeers} braidpoolPeerInfo={null} />);

    // Stats should show dashes when not connected
    const dashes = screen.getAllByText('—');
    expect(dashes.length).toBeGreaterThan(0);
  });
});
