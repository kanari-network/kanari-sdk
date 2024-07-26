'use client'

import { useState, useEffect } from 'react';
import axios from 'axios';

interface Block {
    chain_id: string;
    index: number;
    timestamp: number;
    data: string; // Assuming data is a string representation
    hash: string;
    prev_hash: string;
    tokens: number;
    token_name: string;
    transactions: Transaction[];
    miner_address: string;
}

interface Transaction {
    sender: string;
    receiver: string;
    amount: number;
}

export default function Home() {
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [isLoading, setIsLoading] = useState(true); // Add loading state
  const [newBlockHash, setNewBlockHash] = useState('');
  const [minerReward, setMinerReward] = useState('');
  const [totalBlocks, setTotalBlocks] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);

  useEffect(() => {
      const fetchBlocks = async () => {
          try {
              const response = await axios.get('http://127.0.0.1:3030/get_all_blocks');
              setBlocks(response.data);
              setTotalBlocks(response.data.length);
              setTotalTokens(response.data.reduce((sum, block) => sum + block.tokens, 0));
          } catch (error) {
              console.error('Error fetching blocks:', error);
          } finally {
              setIsLoading(false); // Set loading to false after fetching
          }
      };

      fetchBlocks();
  }, []);

  const createBlock = async () => {
      try {
          const response = await axios.post('http://127.0.0.1:3030/create_block');
          console.log(response.data);
          setNewBlockHash(response.data.block.hash);
          setMinerReward(response.data.block.tokens.toString());
          // After creating a block, fetch the updated blocks
          // Call fetchBlocks again to update the UI
          await fetchBlocks(); // Wait for fetchBlocks to complete
      } catch (error) {
          console.error('Error creating block:', error);
      }
  };

  return (
      <div>
          <h1>Kanari Blockchain Explorer</h1>
          <button onClick={createBlock}>Create Block</button> {/* Add a button to trigger block creation */}
          {isLoading ? (
              <p>Loading blocks...</p>
          ) : (
              <div>
                {newBlockHash && (
                  <p>New block hash: {newBlockHash}</p>
                )}
                {minerReward && (
                  <p>Miner reward: {minerReward} tokens</p>
                )}
                <p>blocks: {totalBlocks}, Total tokens: {totalTokens}</p>
                <ul>
                  {blocks.length > 0 ? (
                      blocks.map((block) => (
                          <li key={block.hash}>
                              <h2>Block {block.index}</h2>
                              <p>Hash: {block.hash}</p>
                              <p>Timestamp: {block.timestamp}</p>
                              {/* ... other block details */}
                              <h3>Transactions:</h3>
                              <ul>
                                  {block.transactions.map((tx) => (
                                      <li key={tx.sender + tx.receiver + tx.amount}>
                                          {tx.sender} sent {tx.amount} to {tx.receiver}
                                      </li>
                                  ))}
                              </ul>
                          </li>
                      ))
                  ) : (
                      <p>No blocks found.</p>
                  )}
              </ul>
              </div>
          )}
      </div>
  );
}
