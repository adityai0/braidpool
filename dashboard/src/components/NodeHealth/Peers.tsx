import { useState, useMemo } from 'react';
import { PeerInfo, BraidpoolPeerInfo, BraidpoolPeer  ,PeersProps} from './Types';
import { formatBytes, paginate, calculateTotalPages } from './Utils';
import { ITEMS_PER_PAGE } from './Constants';



export default function Peers({ bitcoinPeers, braidpoolPeerInfo }: PeersProps) {
  const [activeSection, setActiveSection] = useState<'bitcoin' | 'braidpool'>(
    'bitcoin'
  );
  const [bitcoinPage, setBitcoinPage] = useState(1);
  const [braidpoolPage, setBraidpoolPage] = useState(1);

  const braidpoolPeers = braidpoolPeerInfo?.peers ?? [];
  const bitcoinTotalPages = useMemo(
    () => calculateTotalPages(bitcoinPeers.length, ITEMS_PER_PAGE),
    [bitcoinPeers.length]
  );
  const braidpoolTotalPages = useMemo(
    () => calculateTotalPages(braidpoolPeers.length, ITEMS_PER_PAGE),
    [braidpoolPeers.length]
  );

  const paginatedBitcoinPeers = paginate(
    bitcoinPeers,
    bitcoinPage,
    ITEMS_PER_PAGE
  );
  const paginatedBraidpoolPeers = paginate(
    braidpoolPeers,
    braidpoolPage,
    ITEMS_PER_PAGE
  );

  const handleBitcoinPrev = () => {
    setBitcoinPage((prev) => Math.max(prev - 1, 1));
  };
  const handleBitcoinNext = () => {
    setBitcoinPage((prev) => Math.min(prev + 1, bitcoinTotalPages));
  };
  const handleBraidpoolPrev = () => {
    setBraidpoolPage((prev) => Math.max(prev - 1, 1));
  };
  const handleBraidpoolNext = () => {
    setBraidpoolPage((prev) => Math.min(prev + 1, braidpoolTotalPages));
  };

  const renderBitcoinPeer = (peer: PeerInfo) => (
    <div
      key={peer.id}
      className="flex max-sm:flex-col md:flex-row md:items-start md:justify-between gap-4 p-4 border border-gray-700 rounded-lg bg-gray-900/30 hover:bg-gray-900/50 transition-colors overflow-x-hidden"
    >
      <div className="flex-1 space-y-1 min-w-0">
        <p className="text-white font-medium">{peer.addr}</p>
        <div
          className={`text-sm w-fit px-2 py-0.5 rounded-full font-medium ${
            peer.inbound
              ? 'text-white bg-blue-600'
              : 'text-gray-200 bg-gray-600'
          }`}
        >
          {peer.inbound ? 'Inbound' : 'Outbound'}
        </div>
        <div className="flex max-sm:flex-col md:flex-row lg:gap-1">
          <p className="text-sm text-gray-400">Version:</p>
          <p className="text-sm text-white break-words">{peer.subver}</p>
        </div>
      </div>
      <div className="flex flex-col max-sm:w-full max-sm:pt-2 md:text-right gap-1">
        <p className="text-sm text-gray-400">
          Ping: <span className="text-white font-mono">{peer.pingtime}ms</span>
        </p>
        <p className="text-sm text-gray-400">
          ↑ {formatBytes(peer.bytessent)} ↓ {formatBytes(peer.bytesrecv)}
        </p>
      </div>
    </div>
  );

  const renderBraidpoolPeer = (peer: BraidpoolPeer) => (
    <div
      key={peer.peer_id}
      className="flex max-sm:flex-col md:flex-row md:items-start md:justify-between gap-4 p-4 border border-gray-700 rounded-lg transition-colors overflow-x-hidden"
    >
      <div className="flex-1 space-y-1 min-w-0">
        <p className="text-white font-medium ">
          {peer.peer_id}
        </p>
       <div className='flex  items-center gap-6'>
        <div
          className={`text-sm w-fit px-2 py-0.5 rounded-full font-medium ${
            peer.inbound
              ? 'text-white bg-blue-600'
              : 'text-gray-200 bg-gray-800'
          }`}
        >
          {peer.inbound ? 'Inbound' : 'Outbound'}
          
        </div>
        {peer.geo_group && (
          <p className="text-sm text-gray-400">
            Region: <span className="text-white">{peer.geo_group}</span>
          </p>
        )}
        </div> 
      </div>
      <div className="flex flex-col max-sm:w-full max-sm:pt-2 md:text-right gap-1">
        <p className="text-sm text-gray-400">
          Latency:{' '}
          <span className="text-white font-mono">
            {peer.latency_ms !== null ? `${peer.latency_ms.toFixed(1)}ms` : 'N/A'}
          </span>
        </p>
       
        <p className="text-sm text-gray-400">
          Last seen:{' '}
          <span className="text-white">{peer.last_seen_secs}s ago</span>
        </p>
         <p className="text-sm text-gray-400">
          Score:{' '}
          <span
            className={`font-mono ${peer.score >= 50 ? 'text-green-400' : peer.score >= 25 ? 'text-yellow-400' : 'text-red-400'}`}
          >
            {peer.score.toFixed(1)}
          </span>
        </p>
      </div>
    </div>
  );

  const renderPagination = (
    currentPage: number,
    totalPages: number,
    handlePrev: () => void,
    handleNext: () => void
  ) => (
    <div className="px-6 py-4 flex justify-between items-center border-t border-gray-700 text-sm text-gray-300">
      <button
        onClick={handlePrev}
        disabled={currentPage === 1}
        className={`px-3 py-1 rounded ${
          currentPage === 1
            ? 'opacity-50 cursor-not-allowed'
            : 'hover:bg-gray-800'
        }`}
      >
        Previous
      </button>
      <span>
        Page {currentPage} of {totalPages || 1}
      </span>
      <button
        onClick={handleNext}
        disabled={currentPage === totalPages || totalPages === 0}
        className={`px-3 py-1 rounded ${
          currentPage === totalPages || totalPages === 0
            ? 'opacity-50 cursor-not-allowed'
            : 'hover:bg-gray-800'
        }`}
      >
        Next
      </button>
    </div>
  );

  return (
    <div className="border border-gray-700 rounded-xl shadow-md">
      {/* Section Tabs */}
      <div className="">
  <div className="px-6 py-4 border-b border-gray-700 flex items-center justify-between">
    <div>
      <h2 className="text-white text-xl font-semibold ">
        {activeSection === 'bitcoin'
          ? 'Bitcoin Connected Peers'
          : 'Braidpool Connected Peers'}
      </h2>

      <p className="text-gray-300 text-sm ">
        {activeSection === 'bitcoin'
          ? `${bitcoinPeers.length} total peers connected`
          : braidpoolPeerInfo
            ? `${braidpoolPeerInfo.connected} total peers connected`
            : 'Braidpool node not connected'}
      </p>
    </div>

    
   <div className="flex gap-4 p-2 rounded-lg">
  <button
    className={`px-4 py-2 rounded-lg text-sm font-medium cursor-pointer transition 
      ${activeSection === 'bitcoin'
        ? 'bg-gray-900 text-white'
        : 'bg-gray-600 text-white hover:bg-gray-900'}`}
    onClick={() => setActiveSection('bitcoin')}
  >
    Bitcoin Peers
  </button>

  <button
    className={`px-4 py-2 rounded-lg text-sm font-medium cursor-pointer transition 
      ${activeSection === 'braidpool'
        ? 'bg-gray-900 text-white'
        : 'bg-gray-600 text-white hover:bg-gray-900'}`}
    onClick={() => setActiveSection('braidpool')}
  >
    Braidpool Peers
  </button>
</div>
    

  </div>
</div>

      {/* Bitcoin Peers Section */}
      {activeSection === 'bitcoin' && (
        <>
          <div className="px-6 py-4 space-y-4 max-sm:h-[1116px] md:h-[648px]">
            {paginatedBitcoinPeers.map(renderBitcoinPeer)}
            {[...Array(ITEMS_PER_PAGE - paginatedBitcoinPeers.length)].map(
              (_, idx) => (
                <div
                  key={`empty-${idx}`}
                  className="grid md:grid-cols-2 p-4 border border-transparent rounded-lg h-[96px]"
                />
              )
            )}
          </div>
          {renderPagination(
            bitcoinPage,
            bitcoinTotalPages,
            handleBitcoinPrev,
            handleBitcoinNext
          )}
        </>
      )}

      {/* Braidpool Peers Section */}
      {activeSection === 'braidpool' && (
        <>
          <div className="px-6 py-4 space-y-4 max-sm:h-[1116px] md:h-[648px]">
            {braidpoolPeerInfo === null ? (
              <div className="flex items-center justify-center h-full">
                <p className="text-gray-400">
                  Braidpool node not connected. 
                </p>
              </div>
            ) : braidpoolPeers.length === 0 ? (
              <div className="flex items-center justify-center h-full">
                <p className="text-gray-400">No braidpool peers connected.</p>
              </div>
            ) : (
              <>
                {paginatedBraidpoolPeers.map(renderBraidpoolPeer)}
                {[
                  ...Array(ITEMS_PER_PAGE - paginatedBraidpoolPeers.length),
                ].map((_, idx) => (
                  <div
                    key={`empty-${idx}`}
                    className="grid md:grid-cols-2 p-4 border border-transparent rounded-lg h-[96px]"
                  />
                ))}
              </>
            )}
          </div>
          {renderPagination(
            braidpoolPage,
            braidpoolTotalPages,
            handleBraidpoolPrev,
            handleBraidpoolNext
          )}
        </>
      )}
    </div>
  );
}
