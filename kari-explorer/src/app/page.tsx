'use client'

import { useState, useEffect } from 'react';
import axios from 'axios';

interface Block {
  chain_id: string;
  index: number;
  timestamp: number;
  data: string;
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
  const [isLoading, setIsLoading] = useState(true);
  const [newBlockHash, setNewBlockHash] = useState('');
  const [minerReward, setMinerReward] = useState('');
  const [totalBlocks, setTotalBlocks] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [error, setError] = useState(null); // State to store errors

  useEffect(() => {
    const fetchBlocks = async () => {
      setIsLoading(true); // Set loading to true before fetching
      try {
        const response = await axios.get('http://127.0.0.1:3030/get_all_blocks');
        setBlocks(response.data);
        setTotalBlocks(response.data.length);
        setTotalTokens(response.data.reduce((sum, block) => sum + block.tokens, 0));
      } catch (error: any) {
        console.error('Error fetching blocks:', error);
        setError(error.message); // Store the error message
      } finally {
        setIsLoading(false); // Set loading to false after fetching, regardless of success or error
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
      await fetchBlocks();
    } catch (error: any) {
      console.error('Error creating block:', error);
      setError(error.message); // Store the error message
    }
  };

  return (
    <div>
      <h1>Kanari Blockchain Explorer</h1>
      <button onClick={createBlock}>Create Block</button>
      {isLoading && <p>Loading blocks...</p>}
      {error && <p>Error: {error}</p>} {/* Display error message if there's an error */}
      {!isLoading && !error && ( 
        <div>
          {newBlockHash && <p>New block hash: {newBlockHash}</p>}
          {minerReward && <p>Miner reward: {minerReward} tokens</p>}
          <p>Blocks: {totalBlocks}, Total tokens: {totalTokens}</p>
          <ul>
            {blocks.length > 0 ? (
              blocks.map((block) => (
                <li key={block.hash}>
                  <h2>Block {block.index}</h2>
                  <p>Hash: {block.hash}</p>
                  <p>Timestamp: {block.timestamp}</p>
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
