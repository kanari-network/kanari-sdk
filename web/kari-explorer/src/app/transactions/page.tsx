"use client";

import React, { useState, useEffect } from 'react';
import { getTransactionById as rpcGetTransactionById, searchTransactions as rpcSearchTransactions, getGasFeeInfo as rpcGetGasFeeInfo, API_URL, Transaction } from '../../lib/api';
import Navbar from '../../components/Navbar';
import Link from 'next/link';
import { useTheme } from '../../contexts/ThemeContext';

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

export default function TransactionsPage() {
  const { isDarkMode } = useTheme();
  const [searchTx, setSearchTx] = useState('');
  const [transactionResults, setTransactionResults] = useState<Transaction[]>([]);
  const [singleTransaction, setSingleTransaction] = useState<Transaction | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [limit] = useState(10);
  const [offset, setOffset] = useState(0);
  const [totalTransactions, setTotalTransactions] = useState(0);
  const [gasInfo, setGasInfo] = useState<any>(null);

  const searchTransactions = async () => {
    // Reset single transaction view
    setSingleTransaction(null);
    
    setLoading(true);
    setError('');
    
    try {
      // Check if search is for a transaction ID (starts with 0x followed by 64 hex characters)
      if (searchTx.match(/^0x[a-fA-F0-9]{64}$/)) {
        // Search for specific transaction by ID
        try {
          const result = await rpcGetTransactionById(searchTx);
          setSingleTransaction(result);
          setTransactionResults([]);
        } catch (err: any) {
          setError(err?.message || 'Transaction not found');
        }
      } else if (searchTx) {
        // Search transactions by address
        try {
          const result = await rpcSearchTransactions(searchTx, limit, offset);
          setTransactionResults(result.transactions || []);
          setTotalTransactions(result.total_count || 0);
        } catch (err: any) {
          setError(err?.message || 'Failed to search transactions');
        }
      } else {
        setError('Please enter a transaction ID or address to search');
      }
    } catch (error) {
      console.error('Error searching transactions:', error);
      setError('An error occurred while searching. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const fetchGasFeeInfo = async () => {
    try {
  const result = await rpcGetGasFeeInfo();
  if (result) setGasInfo(result);
    } catch (error) {
      console.error('Error fetching gas fee info:', error);
    }
  };

  useEffect(() => {
    fetchGasFeeInfo();
  }, []);

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    searchTransactions();
  };

  const formatTimestamp = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const handleLoadMore = () => {
    setOffset(offset + limit);
    searchTransactions();
  };

  return (
    <main className={`min-h-screen ${isDarkMode ? 'bg-gray-900 text-white' : 'bg-gradient-to-r from-orange-50 to-yellow-50 text-gray-800'}`}>
      <Navbar 
        searchTx={searchTx}
        setSearchTx={setSearchTx}
        API_URL={API_URL}
        formatKariAmount={formatKariAmount}
        currentPage="transactions"
      />
      
      <div className="container mx-auto max-w-7xl px-4 py-6">
        <h1 className={`text-4xl font-bold mb-6 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
          Transactions
        </h1>

        {/* Search Form */}
        <div className="mb-8">
          <form onSubmit={handleSearchSubmit} className="flex flex-col md:flex-row gap-4">
            <div className="flex-grow">
              <input
                type="text"
                placeholder="Search by transaction ID (0x...) or address"
                value={searchTx}
                onChange={(e) => setSearchTx(e.target.value)}
                className={`w-full px-4 py-3 rounded-lg shadow-sm border ${isDarkMode
                  ? 'bg-gray-800 border-gray-700 focus:border-orange-500 text-white'
                  : 'bg-white border-orange-200 focus:border-orange-500 text-gray-900'
                } focus:outline-none focus:ring-1 focus:ring-orange-300`}
              />
            </div>
            <button
              type="submit"
              className={`px-6 py-3 rounded-lg font-medium ${isDarkMode
                ? 'bg-orange-600 hover:bg-orange-700 text-white'
                : 'bg-orange-500 hover:bg-orange-600 text-white'
              }`}
            >
              Search
            </button>
          </form>
        </div>

        {/* Gas Fee Information */}
        {gasInfo && (
          <div className={`mb-8 p-6 rounded-xl shadow-lg ${isDarkMode 
            ? 'bg-gray-800 border border-gray-700' 
            : 'bg-white border border-orange-100'}`}>
            <h2 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
              Gas Fee Information
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div>
                <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Current Gas Price:</p>
                <p className={`text-xl font-bold ${isDarkMode ? 'text-orange-400' : 'text-orange-600'}`}>
                  {gasInfo.current_gas_price_formatted || "0.000000002"} KARI
                </p>
              </div>
              <div>
                <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Network Congestion:</p>
                <p className={`text-xl font-bold ${isDarkMode ? 'text-orange-400' : 'text-orange-600'}`}>
                  {gasInfo.network_congestion || "Low"}
                </p>
              </div>
              <div>
                <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Gas Collection Address:</p>
                <p className={`text-sm break-all font-mono ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>
                  {gasInfo.gas_collection_address || "0x47621776628ba3a5b9baaab38e61f4c98e893e124204bc4dad52e702e2b24ea1"}
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Loading indicator */}
        {loading && (
          <div className="flex justify-center my-8">
            <div className={`animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 ${isDarkMode ? 'border-orange-500' : 'border-orange-600'}`}></div>
          </div>
        )}

        {/* Error message */}
        {error && (
          <div className="mb-8">
            <p className={`text-center p-4 rounded-lg border ${isDarkMode 
              ? 'bg-red-900/50 text-red-200 border-red-800' 
              : 'bg-red-50 text-red-600 border-red-100'}`}>
              {error}
            </p>
          </div>
        )}

        {/* Single Transaction Details */}
        {singleTransaction && (
          <div className={`mb-8 p-6 rounded-xl shadow-lg ${isDarkMode 
            ? 'bg-gray-800 border border-gray-700' 
            : 'bg-white border border-orange-100'}`}>
            <h2 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
              Transaction Details
            </h2>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
              <div>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Transaction ID</p>
                <p className="text-sm break-all font-mono mt-1">{singleTransaction.id}</p>
              </div>
              
              <div>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Timestamp</p>
                <p className="mt-1">{singleTransaction.timestamp ? formatTimestamp(singleTransaction.timestamp) : 'N/A'}</p>
              </div>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
              <div>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>From</p>
                <Link href={`/accounts/${singleTransaction.sender}`}>
                  <p className="text-sm break-all font-mono mt-1 text-orange-500 hover:text-orange-400">
                    {singleTransaction.sender}
                  </p>
                </Link>
              </div>
              
              <div>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>To</p>
                <Link href={`/accounts/${singleTransaction.receiver}`}>
                  <p className="text-sm break-all font-mono mt-1 text-orange-500 hover:text-orange-400">
                    {singleTransaction.receiver}
                  </p>
                </Link>
              </div>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
              <div>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Amount</p>
                <p className="text-xl font-bold text-orange-500 mt-1">
                  {singleTransaction.amount_formatted || formatKariAmount(singleTransaction.amount)} KARI
                </p>
              </div>
              
              <div>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Gas Fee</p>
                <p className={`mt-1 ${isDarkMode ? 'text-gray-300' : 'text-gray-700'}`}>
                  {singleTransaction.gas_fee_formatted || (singleTransaction.gas_fee !== undefined ? formatKariAmount(singleTransaction.gas_fee) : 'N/A')} KARI
                </p>
              </div>
            </div>
            
            {singleTransaction.block_index !== undefined && (
              <div>
                <p className={`text-sm font-medium ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Block</p>
                <p className={`mt-1 ${isDarkMode ? 'text-gray-300' : 'text-gray-700'}`}>
                  {singleTransaction.block_index}
                </p>
              </div>
            )}
          </div>
        )}

        {/* Transaction Results List */}
        {!singleTransaction && transactionResults.length > 0 && (
          <div className="mb-8">
            <h2 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
              Transaction Results
              <span className="text-sm font-normal ml-2">
                ({transactionResults.length} of {totalTransactions})
              </span>
            </h2>
            
            <div className={`rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className={`${isDarkMode ? 'border-b border-gray-700' : 'border-b border-orange-100'}`}>
                      <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Transaction ID</th>
                      <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>From</th>
                      <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>To</th>
                      <th className={`px-4 py-3 text-right ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Amount</th>
                      <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Timestamp</th>
                    </tr>
                  </thead>
                  <tbody>
                    {transactionResults.map((tx, idx) => (
                      <tr key={idx} className={`${isDarkMode ? 'border-b border-gray-700' : 'border-b border-orange-100'} last:border-b-0 hover:${isDarkMode ? 'bg-gray-700' : 'bg-orange-50'}`}>
                        <td className={`px-4 py-3 font-mono text-sm ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>
                          <Link href={`?txid=${tx.id}`} onClick={(e) => {
                            e.preventDefault();
                            if (tx.id) setSearchTx(tx.id);
                            searchTransactions();
                          }}>
                            <span className="text-orange-500 hover:text-orange-400">
                              {tx.id ? `${tx.id.substring(0, 10)}...${tx.id.substring(tx.id.length - 10)}` : 'N/A'}
                            </span>
                          </Link>
                        </td>
                        <td className={`px-4 py-3 font-mono text-sm ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>
                          <Link href={`/accounts/${tx.sender}`}>
                            <span className="text-orange-500 hover:text-orange-400">
                              {tx.sender.substring(0, 8)}...{tx.sender.substring(tx.sender.length - 8)}
                            </span>
                          </Link>
                        </td>
                        <td className={`px-4 py-3 font-mono text-sm ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>
                          <Link href={`/accounts/${tx.receiver}`}>
                            <span className="text-orange-500 hover:text-orange-400">
                              {tx.receiver.substring(0, 8)}...{tx.receiver.substring(tx.receiver.length - 8)}
                            </span>
                          </Link>
                        </td>
                        <td className={`px-4 py-3 text-right ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>
                          {tx.amount_formatted || formatKariAmount(tx.amount)} KARI
                        </td>
                        <td className={`px-4 py-3 ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>
                          {tx.timestamp ? formatTimestamp(tx.timestamp) : 'N/A'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
            
            {/* Load More Button */}
            {transactionResults.length < totalTransactions && (
              <div className="flex justify-center mt-6">
                <button
                  onClick={handleLoadMore}
                  className={`px-6 py-3 rounded-lg font-medium ${isDarkMode
                    ? 'bg-gray-800 hover:bg-gray-700 text-white'
                    : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200'
                  }`}
                >
                  Load More
                </button>
              </div>
            )}
          </div>
        )}

        {/* No Results */}
        {!loading && !error && !singleTransaction && transactionResults.length === 0 && searchTx && (
          <div className="text-center py-10">
            <p className={`text-lg ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
              No transactions found matching your search.
            </p>
          </div>
        )}
        
        {/* Initial State */}
        {!loading && !error && !singleTransaction && transactionResults.length === 0 && !searchTx && (
          <div className="text-center py-10">
            <p className={`text-lg ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
              Enter a transaction ID or address to search for transactions.
            </p>
          </div>
        )}
      </div>
    </main>
  );
}
