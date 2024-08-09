"use client";

import axios from 'axios';
import React, { useState, useEffect } from 'react';

const KanariBlockchainExplorer = () => {
  const [error, setError] = useState('');
  const [blocks, setBlocks] = useState([]);
  const [totalBlocks, setTotalBlocks] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [searchTx, setSearchTx] = useState('');

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

  const filteredBlocks = blocks?.filter((block) => 
    block.transactions.some((tx) =>
      tx.sender.includes(searchTx) ||
      tx.receiver.includes(searchTx)
    )
  );

  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-4 sm:p-8 md:p-16 lg:p-24 bg-gradient-to-r from-pink-100 to-purple-100">
      <div className="container mx-auto max-w-6xl">
        <h1 className="text-3xl sm:text-4xl md:text-5xl font-bold text-center text-transparent bg-clip-text bg-gradient-to-r from-pink-500 to-purple-500 mb-8">
          Kanari Blockchain Explorer
        </h1>

        {error && <p className="text-red-500 text-center mb-4 p-2 bg-red-100 rounded">{error}</p>}

        <div className="mb-8 w-full max-w-md mx-auto">
          <input
            type="text"
            placeholder="Search by Transaction Sender or Receiver"
            value={searchTx}
            onChange={handleSearchTxChange}
            className="w-full px-4 py-2 border rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-pink-300"
          />
        </div>

        <ul className="bg-white rounded-lg shadow-md p-4 sm:p-6 w-full divide-y divide-gray-200">
          <li className="py-4 text-center">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
              <div>
                <p className="text-lg font-semibold text-pink-500">Total Blocks</p>
                <p className="text-2xl font-bold">{totalBlocks}</p>
              </div>
              <div>
                <p className="text-lg font-semibold text-purple-500">Total Transactions</p>
                <p className="text-2xl font-bold">{filteredBlocks.reduce((sum, block) => sum + block.transactions.length, 0)}</p>
              </div>
              <div>
                <p className="text-lg font-semibold text-indigo-500">Total Tokens</p>
                <p className="text-2xl font-bold">{totalTokens}</p>
              </div>
            </div>
          </li>
          {filteredBlocks.length > 0 ? (
            filteredBlocks.map((block) => (
              <li key={block.hash} className="py-4">
                <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between">
                  <h2 className="text-xl font-medium text-pink-500 mb-2 sm:mb-0">Block {block.index}</h2>
                  <span className="text-gray-500 text-sm">{new Date(block.timestamp).toLocaleString()}</span>
                </div>
                <p className="text-gray-700 mt-1 break-all">Hash: {block.hash}</p>
                <h3 className="text-lg font-semibold mt-4">Transactions: {block.transactions.length}</h3>
                <ul className="list-disc list-inside ml-2 sm:ml-6 mt-2">
                  {block.transactions.map((tx, index) => (
                    <li key={index} className="text-gray-800 break-all">
                      <span className="font-medium">{tx.sender}</span> sent {tx.amount} to <span className="font-medium">{tx.receiver}</span> (Gas: {tx.gas_cost})
                    </li>
                  ))}
                </ul>
              </li>
            ))
          ) : (
            <li className="py-4">
              <p className="text-center text-gray-500">No blocks found.</p>
            </li>
          )}
        </ul>
      </div>
    </main>
  );
};

export default KanariBlockchainExplorer;