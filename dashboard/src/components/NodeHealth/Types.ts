export interface BlockchainInfo {
  chain: string;
  blocks: number;
  headers: number;
  bestblockhash: string;
  difficulty: number;
  verificationprogress: number;
  chainwork: string;
  pruned: boolean;
  size_on_disk: number;
}

export interface PeerInfo {
  id: number;
  addr: string;
  version: number;
  subver: string;
  inbound: boolean;
  startingheight: number;
  synced_headers: number;
  synced_blocks: number;
  pingtime: number;
  bytessent: number;
  bytesrecv: number;
}

export interface NetworkInfo {
  version: number;
  subversion: string;
  protocolversion: number;
  localservices: string;
  localrelay: boolean;
  timeoffset: number;
  networkactive: boolean;
  connections: number;
  connections_in: number;
  connections_out: number;
  relayfee: number;
  incrementalfee: number;
  localaddresses: string[];
  warnings: string;
}
export interface NetworkPanelProps {
  network: NetworkInfo;
}
export interface MempoolInfo {
  loaded: boolean;
  size: number;
  bytes: number;
  usage: number;
  maxmempool: number;
  mempoolminfee: number;
  minrelaytxfee: number;
}

export interface NetTotals {
  totalbytesrecv: number;
  totalbytessent: number;
  timemillis: number;
  uploadtarget: {
    timeframe: number;
    target: number;
    target_reached: boolean;
    serve_historical_blocks: boolean;
    bytes_left_in_cycle: number;
    time_left_in_cycle: number;
  };
}

export interface NodeHealthMessage {
  type: 'node_health_data';
  data: {
    blockchainInfo: BlockchainInfo;
    peerInfo: PeerInfo[];
    networkInfo: NetworkInfo;
    mempoolInfo: MempoolInfo;
    netTotals: NetTotals;
    lastUpdated: string;
  };
}
export interface BandwidthHistoryPoint {
  timestamp: number;
  totalbytesrecv: number;
  totalbytessent: number;
}

export interface BandwidthPanelProps {
  bandwidthHistory: BandwidthHistoryPoint[];
}
export interface PeersProps {
  bitcoinPeers: PeerInfo[];
  braidpoolPeerInfo: BraidpoolPeerInfo | null;
}
export interface BraidpoolPeer {
  peer_id: string;
  ip: string | null;
  inbound: boolean;
  latency_ms: number | null;
  score: number;
  last_seen_secs: number;
  geo_group: string | null;
}

export interface BraidpoolPeerInfo {
  total_peers: number;
  connected: number;
  inbound: number;
  outbound: number;
  network_groups: number;
  avg_latency_ms: number;
  peers: BraidpoolPeer[];
}

export interface BraidpoolPeerMessage {
  type: 'braidpool_peer_data';
  data: BraidpoolPeerInfo;
  time: string;
}
