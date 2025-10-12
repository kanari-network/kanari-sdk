"use client";

import React, { useState, useEffect } from 'react';
import Navbar from '../components/Navbar';
import { useTheme } from '../contexts/ThemeContext';
import { API_URL, getAllBlocks, getBlockchainStatus, Block, BlockchainStatus } from '../lib/api';

// Add utility function to format Kanari amounts
const formatKariAmount = (kaAmount: number, showDecimals: boolean = true): string => {
  const KA_PER_KARI: number = 1_000_000_000;
  
  // Calculate whole and fractional parts
  const wholeKari = Math.floor(kaAmount / KA_PER_KARI);
  const fractionalKa = kaAmount % KA_PER_KARI;
  
  // Format whole part with thousands separators
  const wholeFormatted = wholeKari.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  
  // Format with or without decimal places based on showDecimals parameter
  return showDecimals ? 
    `${wholeFormatted}.${fractionalKa.toString().padStart(9, '0')}` : 
    wholeFormatted;
};

// Using Block and BlockchainStatus from src/lib/api

const KanariBlockchainExplorer = () => {
  const { isDarkMode } = useTheme();
  const [error, setError] = useState('');
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [totalBlocks, setTotalBlocks] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [searchTx, setSearchTx] = useState('');
  const [blockchainStatus, setBlockchainStatus] = useState<BlockchainStatus | null>(null);
  const [chainId, setChainId] = useState<string>('');
  const [genesisDate, setGenesisDate] = useState<number>(0);
  const [latestBlock, setLatestBlock] = useState<any>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [blocksPerPage] = useState(10);
  const [totalPages, setTotalPages] = useState(1);
  const [mintedTokens, setMintedTokens] = useState(0);

  // use centralized API_URL from src/lib/api.ts

  const fetchBlocks = async () => {
    try {
      const result = await getAllBlocks();
      const blocksData: Block[] = result.blocks || [];
      setBlocks([...blocksData].reverse());
      setTotalBlocks(result.block_count || 0);
      setTotalPages(Math.ceil((result.block_count || 0) / blocksPerPage));

      const totalMinted = blocksData.reduce((sum: number, block: Block) => sum + (block.tokens_minted || 0), 0);
      setMintedTokens(totalMinted);
      setError('');
    } catch (error) {
      console.error('Error fetching blocks:', error);
      setError('An error occurred while fetching blocks. Please try again later.');
    }
  };

  const fetchBlockchainStatus = async () => {
    try {
      const status: BlockchainStatus = await getBlockchainStatus();
      setBlockchainStatus(status);
      setChainId(status.chain_id || '');
      setTotalBlocks(status.block_count || 0);
      setLatestBlock(status.latest_block || null);
      setGenesisDate(status.genesis_timestamp || 0);
      setTotalTokens(status.totalSupply || 0);
    } catch (error) {
      console.error('Error fetching blockchain status:', error);
    }
  };

  useEffect(() => {
    fetchBlocks();
    fetchBlockchainStatus();
    const intervalId = setInterval(() => {
      fetchBlocks();
      fetchBlockchainStatus();
    }, 5000);
    return () => clearInterval(intervalId);
  }, []);

  // Add pagination handler
  const handlePageChange = (pageNumber: number) => {
    if (pageNumber > 0 && pageNumber <= totalPages) {
      setCurrentPage(pageNumber);
    }
  };

  // Get current blocks for pagination
  const indexOfLastBlock = currentPage * blocksPerPage;
  const indexOfFirstBlock = indexOfLastBlock - blocksPerPage;
  const currentBlocks = blocks.slice(indexOfFirstBlock, indexOfLastBlock);

  const filteredBlocks = blocks?.filter((block) =>
    block.transactions.some((tx) =>
      tx.sender.includes(searchTx) ||
      tx.receiver.includes(searchTx)
    )
  );

  // Filter blocks if search is active, otherwise use pagination
  const displayBlocks = searchTx ? filteredBlocks : currentBlocks;
  
  // Function to check if a block is the latest
  const isLatestBlock = (block: Block) => {
    return blockchainStatus?.latest_block?.index === block.index;
  };

  return (
    <main className={`min-h-screen ${isDarkMode ? 'bg-gray-900 text-white' : 'bg-gradient-to-r from-orange-50 to-yellow-50 text-gray-800'}`}>
      {/* Add Navbar component */}
      <Navbar 
        searchTx={searchTx}
        setSearchTx={setSearchTx}
        API_URL={API_URL}
        formatKariAmount={formatKariAmount}
      />
      
      <div className="container mx-auto max-w-7xl px-4 py-6">
        {/* Header - simplified since we have a navbar now */}
        <div className="mb-6">
          <h1 className={`text-4xl md:text-5xl font-bold ${isDarkMode ? 'text-white' : 'text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-yellow-600'}`}>
            Kanari Blockchain Explorer
          </h1>
          {chainId && (
            <p className={`mt-2 ${isDarkMode ? 'text-gray-400' : 'text-orange-700'}`}>
              Network: <span className="font-medium">{chainId}</span>
            </p>
          )}
        </div>

        {/* Latest Block Card */}
        {latestBlock && (
          <div className={`mb-12 p-6 rounded-xl shadow-lg ${isDarkMode 
            ? 'bg-gradient-to-r from-orange-900/30 to-amber-900/30 border border-orange-800/50' 
            : 'bg-gradient-to-r from-orange-100 to-amber-100 border border-orange-200'}`}>
            <div className="flex flex-col md:flex-row justify-between items-start md:items-center mb-4">
              <h2 className={`text-2xl font-bold ${isDarkMode ? 'text-orange-400' : 'text-orange-700'}`}>
                Latest Block
              </h2>
              <div className={`mt-2 md:mt-0 px-3 py-1 rounded-full ${isDarkMode ? 'bg-gray-800' : 'bg-white'} shadow-sm`}>
                <p className={`text-sm ${isDarkMode ? 'text-gray-300' : 'text-gray-700'}`}>
                  Last update: {new Date().toLocaleTimeString()}
                </p>
              </div>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className={`p-4 rounded-lg ${isDarkMode ? 'bg-gray-800/50' : 'bg-white/90'} shadow-sm`}>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Block Number</p>
                <p className="text-2xl font-bold text-orange-500">{latestBlock.index}</p>
                <p className={`text-sm mt-2 ${isDarkMode ? 'text-gray-500' : 'text-gray-600'}`}>
                  {new Date(latestBlock.timestamp * 1000).toLocaleString()}
                </p>
              </div>

              <div className={`p-4 rounded-lg ${isDarkMode ? 'bg-gray-800/50' : 'bg-white/90'} shadow-sm`}>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Hash</p>
                <p className="text-sm break-all font-mono mt-1 overflow-hidden text-ellipsis">{latestBlock.hash}</p>
              </div>

              <div className={`p-4 rounded-lg ${isDarkMode ? 'bg-gray-800/50' : 'bg-white/90'} shadow-sm`}>
                <div className="flex justify-between items-start">
                  <div>
                    <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Miner</p>
                    <p className="text-sm break-all font-mono mt-1">
                      {latestBlock.miner?.substring(0, 12)}...
                    </p>
                  </div>
                  <div className="text-right">
                    <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Transactions</p>
                    <p className="text-xl font-bold text-orange-500">{latestBlock.transactions}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Statistics */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-12">
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-500 text-lg mb-1">Total Blocks</p>
            <p className={`text-3xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>{totalBlocks}</p>
          </div>
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-600 text-lg mb-1">Total Transactions</p>
            <p className={`text-3xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
              {blockchainStatus?.total_transactions || filteredBlocks.reduce((sum, block) => sum + block.transaction_count, 0)}
            </p>
          </div>
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-700 text-lg mb-1">Total Tokens</p>
            <p className={`text-3xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
              {mintedTokens || totalTokens ? formatKariAmount(mintedTokens || totalTokens, false) : '0'} KARI
            </p>
          </div>
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-700 text-lg mb-1">Genesis Date</p>
            <p className={`text-lg font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
              {genesisDate ? new Date(genesisDate * 1000).toLocaleString() : 'N/A'}
            </p>
          </div>
        </div>

        {/* Error Message */}
        {error && (
          <div className="max-w-3xl mx-auto mb-8">
            <p className={`text-center p-4 rounded-lg border ${isDarkMode 
              ? 'bg-red-900/50 text-red-200 border-red-800' 
              : 'bg-red-50 text-red-600 border-red-100'}`}>
              {error}
            </p>
          </div>
        )}

        {/* Blocks List - Now shows latest blocks first */}
        <div className="mb-6">
          </div>
        

        {/* Blocks List - Now shows latest blocks first */}
        <div className="mb-6">
          <h2 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
            Latest Blocks
          </h2>
          <div className={`rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            {displayBlocks.length > 0 ? (
              displayBlocks.map((block) => (
                <div 
                  key={block.hash} 
                  className={`p-6 border-b last:border-b-0 ${
                    isLatestBlock(block) 
                      ? isDarkMode 
                        ? 'border-orange-800 bg-gradient-to-r from-orange-900/20 to-amber-900/20' 
                        : 'border-orange-200 bg-gradient-to-r from-orange-50 to-amber-50'
                      : 'border-orange-100'
                  }`}
                >
                  <div className="flex flex-col md:flex-row md:items-center justify-between mb-4">
                    <div className="flex items-center">
                      <h2 className="text-xl font-medium text-orange-500">Block #{block.index}</h2>
                      {isLatestBlock(block) && (
                        <span className={`ml-3 px-2 py-1 text-xs rounded-full ${
                          isDarkMode ? 'bg-orange-800/70 text-orange-200' : 'bg-orange-100 text-orange-600'
                        }`}>
                          Latest
                        </span>
                      )}
                    </div>
                    <span className={`text-sm ${isDarkMode ? 'text-gray-500' : 'text-gray-600'}`}>
                      {block.datetime || new Date(block.timestamp * 1000).toLocaleString()}
                    </span>
                  </div>
                  
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                    <div>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Hash:</p>
                      <p className="text-sm break-all font-mono">{block.hash}</p>
                    </div>
                    
                    <div>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Previous Hash:</p>
                      <p className="text-sm break-all font-mono">{block.prev_hash}</p>
                    </div>
                  </div>
                  
                  <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
                    <div>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Miner:</p>
                      <p className="text-sm break-all font-mono">{block.miner?.substring(0, 20)}...</p>
                    </div>
                    
                    <div>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Transactions:</p>
                      <p className="text-sm">{block.transaction_count}</p>
                    </div>
                    
                    <div>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Tokens Minted:</p>
                      <p className="text-sm">{block.tokens_minted ? formatKariAmount(block.tokens_minted) : '0'} KARI</p>
                    </div>
                  </div>
                  
                  {block.transactions && block.transactions.length > 0 && (
                    <div className="mt-4">
                      <h3 className={`text-md font-medium mb-2 ${isDarkMode ? 'text-gray-300' : 'text-gray-700'}`}>
                        Transactions ({block.transactions.length})
                      </h3>
                      <div className="space-y-2">
                        {block.transactions.map((tx, idx) => (
                          <div key={idx} className={`p-3 rounded-lg ${isDarkMode ? 'bg-gray-700' : 'bg-orange-50'}`}>
                            <p className="text-sm break-all">
                              <span className="font-medium text-orange-500">{tx.sender}</span>
                              <span className="mx-2">→</span>
                              <span className="font-medium text-orange-500">{tx.receiver}</span>
                              <span className={`ml-2 ${isDarkMode ? 'text-gray-500' : 'text-gray-600'}`}>
                                ({formatKariAmount(tx.amount)} KARI)
                              </span>
                            </p>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              ))
            ) : (
              <div className={`p-8 text-center ${isDarkMode ? 'text-gray-500' : 'text-gray-600'}`}>
                No blocks found matching your search.
              </div>
            )}
          </div>
        </div>

        {/* Pagination */}
        {!searchTx && totalPages > 1 && (
          <div className="flex justify-center mt-8 space-x-2">
            <button 
              onClick={() => handlePageChange(currentPage - 1)} 
              disabled={currentPage === 1}
              className={`px-4 py-2 rounded-lg ${isDarkMode 
                ? 'bg-gray-800 hover:bg-gray-700 text-white disabled:bg-gray-900 disabled:text-gray-600' 
                : 'bg-white hover:bg-orange-50 text-orange-600 border border-orange-200 disabled:bg-gray-100 disabled:text-gray-400'}`}
            >
              Previous
            </button>
            
            <span className={`px-4 py-2 ${isDarkMode ? 'text-white' : 'text-gray-700'}`}>
              Page {currentPage} of {totalPages}
            </span>
            
            <button 
              onClick={() => handlePageChange(currentPage + 1)}
              disabled={currentPage === totalPages}
              className={`px-4 py-2 rounded-lg ${isDarkMode 
                ? 'bg-gray-800 hover:bg-gray-700 text-white disabled:bg-gray-900 disabled:text-gray-600' 
                : 'bg-white hover:bg-orange-50 text-orange-600 border border-orange-200 disabled:bg-gray-100 disabled:text-gray-400'}`}
            >
              Next
            </button>
          </div>
        )}
      </div>
    </main>
  );
};

export default KanariBlockchainExplorer;