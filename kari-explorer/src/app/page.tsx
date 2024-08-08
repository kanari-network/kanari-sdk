"use client";

import axios from 'axios';
import React, { useState, useEffect } from 'react';

const KanariBlockchainExplorer = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [newBlockHash, setNewBlockHash] = useState('');
  const [minerReward, setMinerReward] = useState('');
  const [blocks, setBlocks] = useState([]);
  const [totalBlocks, setTotalBlocks] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [searchTx, setSearchTx] = useState('');

  const fetchBlocks = async () => {
    try {
      setIsLoading(true);
      const response = await axios.post('http://127.0.0.1:3030', {
        jsonrpc: "2.0",
        method: "get_latest_block", // Verify this method name
        params: [],
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      console.log("Full Response:", response.data); // Log the entire response

      // Assuming response.data.result has this structure:
      // {
      //   "blocks": [], 
      //   "totalBlocks": 123, 
      //   "totalTokens": 456 
      // }
      setBlocks(response.data.result.blocks);
      setTotalBlocks(response.data.result.totalBlocks);
      setTotalTokens(response.data.result.totalTokens);

    } catch (error: any) {
      console.error('Error fetching blocks:', error);
      setError(error.message || 'An error occurred while fetching blocks.');
    } finally {
      setIsLoading(false);
    }
  };

  const createBlock = async () => {
    try {
      setIsLoading(true);
      const response = await axios.post('http://127.0.0.1:3030', {
        jsonrpc: "2.0",
        method: "create_block", // Verify this method name
        params: [],
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      console.log("Full Response:", response.data); // Log the entire response

      setNewBlockHash(response.data.result.hash);

      // Check if response.data.result.tokens exists and is not undefined
      if (response.data.result.tokens !== undefined) {
        setMinerReward(response.data.result.tokens.toString());
      } else {
        console.warn("Miner reward (tokens) is undefined in the response.");
        setMinerReward('N/A'); // Or any other default value
      }

      // Update state with the new block data
      setBlocks([response.data.result, ...blocks]);
      setTotalBlocks(totalBlocks + 1);

    } catch (error: any) {
      console.error('Error creating block:', error);
      setError(error.message || 'An error occurred while creating a block.');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchBlocks();
    const intervalId = setInterval(fetchBlocks, 5000);
    return () => clearInterval(intervalId);
  }, []);

  const handleSearchTxChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTx(event.target.value);
  };

  const filteredBlocks = blocks?.filter((block) => 
    block.transactions.some((tx) =>
      tx.sender.includes(searchTx) ||
      tx.receiver.includes(searchTx)
    )
  );

  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-24 bg-gradient-to-r from-pink-100 to-purple-100">
      <h1 className="text-5xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-pink-500 to-purple-500">
        Kanari Blockchain Explorer
      </h1>

      {/* Loading and Error Messages */}
      {isLoading && <p className="text-gray-600">Loading...</p>}
      {error && <p className="text-red-500">Error: {error}</p>}

      {/* Create Block Button */}
      <button
        onClick={createBlock}
        disabled={isLoading}
        className="bg-gradient-to-r from-pink-500 to-purple-500 hover:from-pink-600 hover:to-purple-600 text-white font-bold py-3 px-6 rounded-full shadow-md"
      >
        Create Block
      </button>

      {/* New Block Information */}
      {newBlockHash && (
        <p className="text-green-500 mt-4">
          New block hash: <span className="font-bold">{newBlockHash}</span>
        </p>
      )}
      {minerReward && <p className="mt-2">Miner reward: {minerReward} tokens</p>}
      <p className="mt-2">Blocks: {totalBlocks}, Total tokens: {totalTokens}</p>

      {/* Search Bar */}
      <div className="mt-6 w-full max-w-md">
        <input
          type="text"
          placeholder="Search by Transaction Sender or Receiver"
          value={searchTx}
          onChange={handleSearchTxChange}
          className="w-full px-4 py-2 border rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-pink-300"
        />
      </div>

      {/* Block List */}
      <ul className="bg-white rounded-lg shadow-md p-6 mt-8 w-full max-w-xl divide-y divide-gray-200">
        {filteredBlocks.length > 0 ? (
          filteredBlocks.map((block) => (
            <li key={block.hash} className="py-4">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-medium text-pink-500">Block {block.index}</h2>
                <span className="text-gray-500 text-sm">{new Date(block.timestamp).toLocaleString()}</span>
              </div>
              <p className="text-gray-700 mt-1">Hash: {block.hash}</p>
              <h3 className="text-lg font-semibold mt-4">Transactions:</h3>
              <ul className="list-disc list-inside ml-6 mt-2">
                {block.transactions.map((tx) => (
                  <li key={tx.sender + tx.receiver + tx.amount} className="text-gray-800">
                    {tx.sender} sent {tx.amount} to {tx.receiver} (Gas: {tx.gas_cost})
                  </li>
                ))}
              </ul>
      
            </li>
            
          ))
        ) : (
          <p className="text-center text-gray-500 py-4">No blocks found.</p>
        )}
      </ul>

            {/* Block List */}
      <ul className="bg-white rounded-lg shadow-md p-6 mt-8 w-full max-w-xl divide-y divide-gray-200">
        {blocks
          .filter((block) =>
            block.transactions.some((tx) =>
              tx.sender.includes(searchTx) ||
              tx.receiver.includes(searchTx)
            )
          )
          .map((block) => (
            <li key={block.hash} className="py-4">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-medium text-pink-500">
                  Block {block.index}
                </h2>
                <span className="text-gray-500 text-sm">
                  {new Date(block.timestamp).toLocaleString()}
                </span>
              </div>
              <p className="text-gray-700 mt-1">Hash: {block.hash}</p>
              <h3 className="text-lg font-semibold mt-4">Transactions:</h3>
              <ul className="list-disc list-inside ml-6 mt-2">
                {block.transactions.map((tx) => (
                  <li key={tx.sender + tx.receiver + tx.amount} className="text-gray-800">
                    {tx.sender} sent {tx.amount} to {tx.receiver} (Gas: {tx.gas_cost})
                  </li>
                ))}
              </ul>
            </li>
          ))
        .length > 0 ? (
          blocks
          .filter((block) =>
            block.transactions.some((tx) =>
              tx.sender.includes(searchTx) ||
              tx.receiver.includes(searchTx)
            )
          )
          .map((block) => (
            <li key={block.hash} className="py-4">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-medium text-pink-500">
                  Block {block.index}
                </h2>
                <span className="text-gray-500 text-sm">
                  {new Date(block.timestamp).toLocaleString()}
                </span>
              </div>
              <p className="text-gray-700 mt-1">Hash: {block.hash}</p>
              <h3 className="text-lg font-semibold mt-4">Transactions:</h3>
              <ul className="list-disc list-inside ml-6 mt-2">
                {block.transactions.map((tx) => (
                  <li key={tx.sender + tx.receiver + tx.amount} className="text-gray-800">
                    {tx.sender} sent {tx.amount} to {tx.receiver} (Gas: {tx.gas_cost})
                  </li>
                ))}
              </ul>
            </li>
          ))
        ) : (
          <p className="text-center text-gray-500 py-4">No blocks found.</p>
        )}
      </ul>
    </main>
  );
};

export default KanariBlockchainExplorer;
