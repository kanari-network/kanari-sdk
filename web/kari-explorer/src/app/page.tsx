"use client";

import axios from 'axios';
import React, { useState, useEffect } from 'react';

const KanariBlockchainExplorer = () => {
  const [error, setError] = useState('');
  const [blocks, setBlocks] = useState([]);
  const [totalBlocks, setTotalBlocks] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [searchTx, setSearchTx] = useState('');
  const [isDarkMode, setIsDarkMode] = useState(false);

  const fetchBlocks = async () => {
    try {
      const response = await axios.post('http://127.0.0.1:3030', {
        jsonrpc: "2.0",
        method: "get_latest_block",
        params: [],
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      console.log("Full Response:", response.data);

      setBlocks(response.data.result.blocks);
      setTotalBlocks(response.data.result.totalBlocks);
      setTotalTokens(response.data.result.totalTokens);
      setError('');
    } catch (error) {
      console.error('Error fetching blocks:', error);
      setError('An error occurred while fetching blocks. Please try again later.');
    }
  };

  useEffect(() => {
    fetchBlocks();
    const intervalId = setInterval(fetchBlocks, 5000);
    return () => clearInterval(intervalId);
  }, []);

  const handleSearchTxChange = (event) => {
    setSearchTx(event.target.value);
  };

  const toggleTheme = () => {
    setIsDarkMode(!isDarkMode);
  };

  const filteredBlocks = blocks?.filter((block) => 
    block.transactions.some((tx) =>
      tx.sender.includes(searchTx) ||
      tx.receiver.includes(searchTx)
    )
  );

  return (
    <main className={`min-h-screen ${isDarkMode ? 'bg-gray-900 text-white' : 'bg-gradient-to-r from-orange-50 to-yellow-50'}`}>
      {/* Header */}
      <div className="container mx-auto max-w-7xl px-4 py-6">
        <div className="flex justify-between items-center mb-12">
          <h1 className={`text-4xl md:text-5xl font-bold ${isDarkMode ? 'text-white' : 'text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-yellow-600'}`}>
            Kanari Blockchain Explorer
          </h1>
          <button 
            onClick={toggleTheme} 
            className={`px-4 py-2 rounded-lg transition-colors ${
              isDarkMode 
                ? 'bg-gray-800 hover:bg-gray-700 border-gray-700' 
                : 'bg-white hover:bg-orange-50 border-orange-200'
            } border shadow-sm`}
          >
            {isDarkMode ? '🌞' : '🌙'}
          </button>
        </div>
  
        {/* Search */}
        <div className="max-w-3xl mx-auto mb-12">
          <div className="relative">
            <input
              type="text"
              placeholder="Search transactions by sender or receiver address..."
              value={searchTx}
              onChange={handleSearchTxChange}
              className={`w-full px-6 py-4 rounded-xl text-lg shadow-sm border ${
                isDarkMode 
                  ? 'bg-gray-800 border-gray-700 focus:border-orange-500' 
                  : 'bg-white border-orange-200 focus:border-orange-500'
              } focus:outline-none focus:ring-2 focus:ring-orange-300`}
            />
            <span className="absolute right-6 top-1/2 -translate-y-1/2 text-gray-400">🔍</span>
          </div>
        </div>
  
        {/* Error Message */}
        {error && (
          <div className="max-w-3xl mx-auto mb-8">
            <p className="text-red-500 text-center p-4 bg-red-50 rounded-lg border border-red-100">
              {error}
            </p>
          </div>
        )}
  
        {/* Statistics */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-500 text-lg mb-1">Total Blocks</p>
            <p className="text-3xl font-bold">{totalBlocks}</p>
          </div>
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-600 text-lg mb-1">Total Transactions</p>
            <p className="text-3xl font-bold">{filteredBlocks.reduce((sum, block) => sum + block.transactions.length, 0)}</p>
          </div>
          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <p className="text-orange-700 text-lg mb-1">Total Tokens</p>
            <p className="text-3xl font-bold">{totalTokens}</p>
          </div>
        </div>
  
        {/* Blocks List */}
        <div className={`rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
          {filteredBlocks.length > 0 ? (
            filteredBlocks.map((block) => (
              <div key={block.hash} className="p-6 border-b last:border-b-0 border-orange-100">
                <div className="flex flex-col md:flex-row md:items-center justify-between mb-4">
                  <h2 className="text-xl font-medium text-orange-500">Block #{block.index}</h2>
                  <span className="text-gray-500 text-sm">{new Date(block.timestamp).toLocaleString()}</span>
                </div>
                <p className="text-sm text-gray-600 break-all mb-4">Hash: {block.hash}</p>
                <div className="space-y-2">
                  {block.transactions.map((tx, idx) => (
                    <div key={idx} className={`p-3 rounded-lg ${isDarkMode ? 'bg-gray-700' : 'bg-orange-50'}`}>
                      <p className="text-sm break-all">
                        <span className="font-medium text-orange-500">{tx.sender}</span>
                        <span className="mx-2">→</span>
                        <span className="font-medium text-orange-500">{tx.receiver}</span>
                        <span className="ml-2 text-gray-500">({tx.amount} tokens)</span>
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            ))
          ) : (
            <div className="p-8 text-center text-gray-500">
              No blocks found matching your search.
            </div>
          )}
        </div>
      </div>
    </main>
  );
};

export default KanariBlockchainExplorer;