"use client";

import React, { useState, useEffect } from 'react';
import { useParams } from 'next/navigation';
import axios from 'axios';
import Link from 'next/link';
import Navbar from '../../../components/Navbar';
import { useTheme } from '../../../contexts/ThemeContext';
import TransactionChart from '../../../components/TransactionChart';

interface Transaction {
  sender: string;
  receiver: string;
  amount: number;
  block_index?: number;
  timestamp?: number;
  hash?: string;
}

interface AccountDetails {
  address: string;
  balance: number;
  balance_formatted: string;
  account_type: string;
  is_contract: boolean;
  transaction_count: number;
  transactions: Transaction[];
  code?: string;
}

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

export default function AccountDetailsPage() {
  const params = useParams();
  const address = params.address as string;
  const { isDarkMode } = useTheme();
  
  const [searchTx, setSearchTx] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [accountDetails, setAccountDetails] = useState<AccountDetails | null>(null);

  const API_URL = 'http://192.168.1.103:30030';

  const fetchAccountDetails = async () => {
    try {
      setLoading(true);
      
      // Use the new get_account_details endpoint
      const accountResponse = await axios.post(API_URL, {
        jsonrpc: "2.0",
        method: "get_account_details",
        params: {
          address: address
        },
        id: 1
      }, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (accountResponse.data.error) {
        throw new Error(accountResponse.data.error.message || 'Failed to fetch account details');
      }

      setAccountDetails(accountResponse.data.result);
      setError('');
    } catch (error) {
      console.error('Error fetching account details:', error);
      setError('Failed to fetch account details. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (address) {
      fetchAccountDetails();
    }
  }, [address]);

  return (
    <main className={`min-h-screen ${isDarkMode ? 'bg-gray-900 text-white' : 'bg-gradient-to-r from-orange-50 to-yellow-50 text-gray-800'}`}>
      <Navbar 
        searchTx={searchTx}
        setSearchTx={setSearchTx}
        API_URL={API_URL}
        formatKariAmount={formatKariAmount}
        currentPage="accounts"
      />
      
      <div className="container mx-auto max-w-7xl px-4 py-6">
        {/* Back button */}
        <div className="mb-6">
          <Link href="/accounts">
            <span className={`inline-flex items-center px-4 py-2 rounded-lg font-medium ${
              isDarkMode 
                ? 'bg-gray-800 hover:bg-gray-700 text-white' 
                : 'bg-white hover:bg-orange-50 text-orange-600 border border-orange-200'
            }`}>
              ← Back to All Accounts
            </span>
          </Link>
        </div>

        <h1 className={`text-3xl font-bold mb-2 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
          Account Details
        </h1>
        
        <h2 className={`text-sm font-mono break-all mb-6 ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
          {address}
        </h2>

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

        {/* Account info */}
        {accountDetails && (
          <>
            {/* Account stats */}
            <div className={`grid grid-cols-1 md:grid-cols-3 gap-6 mb-8`}>
              <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
                <p className={`text-orange-500 text-lg mb-1`}>Balance</p>
                <p className={`text-2xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                  {accountDetails.balance_formatted} KARI
                </p>
              </div>
              
              <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
                <p className={`text-orange-600 text-lg mb-1`}>Transactions</p>
                <p className={`text-2xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                  {accountDetails.transaction_count}
                </p>
              </div>
              
              <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
                <p className={`text-orange-700 text-lg mb-1`}>Account Type</p>
                <p className={`text-xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                  {accountDetails.is_contract ? 
                    <span className={`px-3 py-1 rounded-full text-sm ${isDarkMode ? 'bg-amber-900/30 text-amber-400' : 'bg-amber-100 text-amber-600'}`}>
                      Contract Account
                    </span> : 
                    accountDetails.account_type || 'Regular Account'
                  }
                </p>
              </div>
            </div>
            
            {/* Transaction Chart */}
            {accountDetails.transactions && accountDetails.transactions.length > 0 && (
              <div className="mb-8">
                <TransactionChart 
                  transactions={accountDetails.transactions} 
                  accountAddress={address} 
                  isDarkMode={isDarkMode} 
                />
              </div>
            )}

            {/* Transactions */}
            <h3 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
              Transactions
            </h3>
            
            {accountDetails.transactions && accountDetails.transactions.length > 0 ? (
              <div className={`rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className={`${isDarkMode ? 'border-b border-gray-700' : 'border-b border-orange-100'}`}>
                        <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Type</th>
                        <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>From/To</th>
                        <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Amount</th>
                        <th className={`px-4 py-3 text-left ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Block</th>
                      </tr>
                    </thead>
                    <tbody>
                      {/* Sort transactions by block_index in descending order */}
                      {[...accountDetails.transactions]
                        .sort((a, b) => {
                          // Sort by block_index in descending order (latest first)
                          const blockA = a.block_index !== undefined ? a.block_index : 0;
                          const blockB = b.block_index !== undefined ? b.block_index : 0;
                          return blockB - blockA;
                        })
                        .map((tx, idx) => {
                        const isReceived = tx.receiver === address;
                        const counterparty = isReceived ? tx.sender : tx.receiver;
                        
                        return (
                          <tr key={idx} className={`${isDarkMode ? 'border-b border-gray-700' : 'border-b border-orange-100'} last:border-b-0`}>
                            <td className={`px-4 py-3`}>
                              <span className={`px-2 py-1 rounded-md text-xs ${isReceived ? 
                                (isDarkMode ? 'bg-green-900/30 text-green-400' : 'bg-green-100 text-green-600') : 
                                (isDarkMode ? 'bg-red-900/30 text-red-400' : 'bg-red-100 text-red-600')
                              }`}>
                                {isReceived ? 'Received' : 'Sent'}
                              </span>
                            </td>
                            <td className={`px-4 py-3 font-mono text-sm ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>
                              {counterparty}
                            </td>
                            <td className={`px-4 py-3 ${isReceived ? 
                              (isDarkMode ? 'text-green-400' : 'text-green-600') : 
                              (isDarkMode ? 'text-red-400' : 'text-red-600')
                            }`}>
                              {isReceived ? '+' : '-'}{formatKariAmount(tx.amount)} KARI
                            </td>
                            <td className={`px-4 py-3 ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
                              {tx.block_index !== undefined ? tx.block_index : 'N/A'}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : (
              <div className={`p-8 text-center ${isDarkMode ? 'text-gray-500' : 'text-gray-600'} ${isDarkMode ? 'bg-gray-800' : 'bg-white'} rounded-lg`}>
                No transactions found for this account.
              </div>
            )}
            
            {/* Contract code section if applicable */}
            {accountDetails.is_contract && accountDetails.code && (
              <div className="mt-8">
                <h3 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
                  Contract Code
                </h3>
                <div className={`p-4 rounded-lg ${isDarkMode ? 'bg-gray-800' : 'bg-white'}`}>
                  <pre className={`overflow-auto p-4 rounded-md ${isDarkMode ? 'bg-gray-900 text-gray-300' : 'bg-gray-50 text-gray-800'}`}>
                    {accountDetails.code}
                  </pre>
                </div>
              </div>
            )}
          </>
        )}
        
        {/* Refresh button */}
        {!loading && (
          <div className="flex justify-center mt-8">
            <button
              onClick={fetchAccountDetails}
              className={`px-6 py-3 rounded-lg font-medium ${isDarkMode
                ? 'bg-gray-800 hover:bg-gray-700 text-white'
                : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200'
              }`}
            >
              Refresh Account Data
            </button>
          </div>
        )}
      </div>
    </main>
  );
}
