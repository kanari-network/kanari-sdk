"use client";

import axios from 'axios';
import React, { useState, useEffect } from 'react';

interface Transaction {
  sender: string;
  receiver: string;
  amount: number;
}

interface Block {
  index: number;
  hash: string;
  prev_hash: string;
  timestamp: number;
  datetime: string;
  miner: string;
  transactions: Transaction[];
  transaction_count: number;
  tokens_minted: number;
}

interface Account {
  address: string;
  balance: number;
  balance_formatted: string;
  transaction_count: number;
  is_contract: boolean;
}

interface BlockchainStatus {
  chain_id: string;
  block_height: number;
  block_count: number;
  latest_block: {
    index: number;
    hash: string;
    timestamp: number;
    transactions: number;
    miner: string;
  };
  total_transactions: number;
  genesis_timestamp: number;
}

const KanariBlockchainExplorer = () => {
  const [error, setError] = useState('');
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [totalBlocks, setTotalBlocks] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [searchTx, setSearchTx] = useState('');
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [searchAccount, setSearchAccount] = useState('');
  const [accountBalance, setAccountBalance] = useState<number | null>(null);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [showAllAccounts, setShowAllAccounts] = useState(false);
  const [blockchainStatus, setBlockchainStatus] = useState<BlockchainStatus | null>(null);
  const [chainId, setChainId] = useState<string>('');
  const [genesisDate, setGenesisDate] = useState<number>(0);
  const [latestBlock, setLatestBlock] = useState<any>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [blocksPerPage] = useState(10);
  const [totalPages, setTotalPages] = useState(1);
  const [mintedTokens, setMintedTokens] = useState(0);

  const API_URL = 'http://127.0.0.1:30031';

  const fetchBlocks = async () => {
    try {
      const response = await axios.post(API_URL, {
        jsonrpc: "2.0",
        method: "get_all_blocks",
        params: [],
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      console.log("Full Response:", response.data);

      if (response.data.result) {
        // Store blocks in reverse order (newest first)
        const blocksData = response.data.result.blocks || [];
        setBlocks([...blocksData].reverse());
        
        setTotalBlocks(response.data.result.block_count || 0);
        setTotalPages(Math.ceil((response.data.result.block_count || 0) / blocksPerPage));
        
        // Calculate total minted tokens from blocks
        const totalMinted = blocksData.reduce(
          (sum: number, block: Block) => sum + (block.tokens_minted || 0), 0
        );
        setMintedTokens(totalMinted);
        
        setError('');
      }
    } catch (error) {
      console.error('Error fetching blocks:', error);
      setError('An error occurred while fetching blocks. Please try again later.');
    }
  };

  const fetchBlockchainStatus = async () => {
    try {
      const response = await axios.post(API_URL, {
        jsonrpc: "2.0",
        method: "blockchain_status",
        params: [],
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (response.data.result) {
        const status = response.data.result;
        setBlockchainStatus(status);
        setChainId(status.chain_id || '');
        setTotalBlocks(status.block_count || 0);
        setLatestBlock(status.latest_block || null);
        setGenesisDate(status.genesis_timestamp || 0);
        setTotalTokens(status.totalSupply || 0);
        
        // Use total_transactions from blockchain status if available
        if (status.total_transactions !== undefined) {
          // This will be updated when we have actual transaction data
        }
      }
    } catch (error) {
      console.error('Error fetching blockchain status:', error);
    }
  };

  const fetchAccountBalance = async () => {
    // Keep this function for possible future use
    if (!searchAccount.trim()) {
      setError('Please enter an account address');
      setAccountBalance(null);
      return;
    }

    try {
      const response = await axios.post(API_URL, {
        jsonrpc: "2.0",
        method: "get_balance",
        params: [searchAccount],
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (response.data.result !== undefined) {
        setAccountBalance(response.data.result);
        setError('');
      } else if (response.data.error) {
        setError(response.data.error.message || 'Error fetching balance');
        setAccountBalance(null);
      }
    } catch (error) {
      console.error('Error fetching account balance:', error);
      setError('Failed to fetch account balance. Please try again.');
      setAccountBalance(null);
    }
  };

  const fetchAllAccounts = async () => {
    try {
      const response = await axios.post(API_URL, {
        jsonrpc: "2.0",
        method: "list_accounts",
        params: [],
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (response.data.result && response.data.result.accounts) {
        setAccounts(response.data.result.accounts);
        setError('');
      } else {
        setAccounts([]);
        setError('No accounts data available');
      }
    } catch (error) {
      console.error('Error fetching accounts:', error);
      setError('Failed to fetch accounts. Please try again.');
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

  const handleSearchTxChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTx(event.target.value);
  };

  const handleSearchAccountChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchAccount(event.target.value);
    setAccountBalance(null);
  };

  const toggleAccounts = () => {
    if (!showAllAccounts) {
      fetchAllAccounts();
    }
    setShowAllAccounts(!showAllAccounts);
  };

  const toggleTheme = () => {
    setIsDarkMode(!isDarkMode);
  };

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
      <div className="container mx-auto max-w-7xl px-4 py-6">
        {/* Header */}
        <div className="flex justify-between items-center mb-6">
          <div>
            <h1 className={`text-4xl md:text-5xl font-bold ${isDarkMode ? 'text-white' : 'text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-yellow-600'}`}>
              Kanari Blockchain Explorer
            </h1>
            {chainId && (
              <p className={`mt-2 ${isDarkMode ? 'text-gray-400' : 'text-orange-700'}`}>
                Network: <span className="font-medium">{chainId}</span>
              </p>
            )}
          </div>
          <button
            onClick={toggleTheme}
            className={`px-4 py-2 rounded-lg transition-colors ${isDarkMode
                ? 'bg-gray-800 hover:bg-gray-700 border-gray-700'
                : 'bg-white hover:bg-orange-50 border-orange-200'
              } border shadow-sm`}
          >
            {isDarkMode ? '🌞' : '🌙'}
          </button>
        </div>

        {/* Latest Block Card - Moved to the top and made more prominent */}
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
            <p className={`text-3xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>{mintedTokens || totalTokens}</p>
          </div>
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-700 text-lg mb-1">Genesis Date</p>
            <p className={`text-lg font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
              {genesisDate ? new Date(genesisDate * 1000).toLocaleString() : 'N/A'}
            </p>
          </div>
        </div>

        {/* Account Balance section removed */}

        {/* Show All Accounts Toggle */}
        <div className="max-w-3xl mx-auto mb-8">
          <button
            onClick={toggleAccounts}
            className={`w-full px-6 py-4 rounded-xl text-lg font-medium ${isDarkMode
              ? 'bg-gray-800 hover:bg-gray-700 text-white'
              : 'bg-white hover:bg-orange-50 text-orange-600 border border-orange-200'
            }`}
          >
            {showAllAccounts ? 'Hide All Accounts' : 'Show All Accounts'}
          </button>
        </div>

        {/* All Accounts List */}
        {showAllAccounts && (
          <div className={`max-w-3xl mx-auto mb-12 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <h2 className={`text-2xl font-bold p-6 border-b ${isDarkMode ? 'border-gray-700 text-white' : 'border-orange-100 text-orange-700'}`}>
              All Accounts
            </h2>
            <div className="max-h-96 overflow-y-auto">
              {accounts.map((account, index) => (
                <div key={index} className={`p-4 border-b ${isDarkMode ? 'border-gray-700' : 'border-orange-100'} last:border-b-0`}>
                  <div className="flex flex-col space-y-1">
                    <p className="text-sm break-all">
                      <span className="font-medium text-orange-500">{account.address}</span>
                    </p>
                    <div className="flex flex-wrap gap-x-4">
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-700'}`}>
                        <span className="font-medium">Balance:</span> {account.balance_formatted}
                      </p>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-700'}`}>
                        <span className="font-medium">Transactions:</span> {account.transaction_count}
                      </p>
                      {account.is_contract && (
                        <p className={`text-sm ${isDarkMode ? 'text-amber-400' : 'text-amber-600'}`}>
                          Contract Account
                        </p>
                      )}
                    </div>
                  </div>
                </div>
              ))}
              {accounts.length === 0 && (
                <div className="p-6 text-center text-gray-500">
                  No accounts found.
                </div>
              )}
            </div>
          </div>
        )}

        {/* Transaction Search */}
        <div className="max-w-3xl mx-auto mb-12">
          <h2 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
            Search Transactions
          </h2>
          <div className="relative">
            <input
              type="text"
              placeholder="Search transactions by sender or receiver address..."
              value={searchTx}
              onChange={handleSearchTxChange}
              className={`w-full px-6 py-4 rounded-xl text-lg shadow-sm border ${isDarkMode
                ? 'bg-gray-800 border-gray-700 focus:border-orange-500 text-white'
                : 'bg-white border-orange-200 focus:border-orange-500 text-gray-900'
              } focus:outline-none focus:ring-2 focus:ring-orange-300`}
            />
            <span className="absolute right-6 top-1/2 -translate-y-1/2 text-gray-400">🔍</span>
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
                      <p className="text-sm">{block.tokens_minted}</p>
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
                                ({tx.amount} tokens)
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