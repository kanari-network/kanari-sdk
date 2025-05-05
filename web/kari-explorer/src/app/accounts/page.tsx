"use client";

import React, { useState, useEffect } from 'react';
import axios from 'axios';
import Navbar from '../../components/Navbar';
import Link from 'next/link';
import { useTheme } from '../../contexts/ThemeContext';

interface Account {
  address: string;
  balance: number;
  balance_formatted: string;
  transaction_count: number;
  is_contract: boolean;
  account_type?: string;
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

export default function AccountsPage() {
  const { isDarkMode } = useTheme();
  const [searchTx, setSearchTx] = useState('');
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchAccount, setSearchAccount] = useState('');
  const [filteredAccounts, setFilteredAccounts] = useState<Account[]>([]);

  const API_URL = 'http://192.168.1.103:30030';

  const fetchAllAccounts = async () => {
    try {
      setLoading(true);
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
        setFilteredAccounts(response.data.result.accounts);
        setError('');
      } else {
        setAccounts([]);
        setFilteredAccounts([]);
        setError('No accounts data available');
      }
    } catch (error) {
      console.error('Error fetching accounts:', error);
      setError('Failed to fetch accounts. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchAllAccounts();
    
    // Set up interval to refresh accounts
    const intervalId = setInterval(fetchAllAccounts, 30000); // Refresh every 30 seconds
    
    return () => clearInterval(intervalId);
  }, []);

  useEffect(() => {
    // Filter accounts when search term changes
    if (searchAccount) {
      const filtered = accounts.filter(account => 
        account.address.toLowerCase().includes(searchAccount.toLowerCase())
      );
      setFilteredAccounts(filtered);
    } else {
      setFilteredAccounts(accounts);
    }
  }, [searchAccount, accounts]);

  const handleSearchAccountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchAccount(e.target.value);
  };

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
        <h1 className={`text-4xl font-bold mb-6 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
          All Accounts
        </h1>

        {/* Search box specifically for accounts */}
        <div className="mb-6">
          <div className="relative max-w-md mx-auto">
            <input
              type="text"
              placeholder="Search accounts by address..."
              value={searchAccount}
              onChange={handleSearchAccountChange}
              className={`w-full px-4 py-3 rounded-lg shadow-sm border ${isDarkMode
                ? 'bg-gray-800 border-gray-700 focus:border-orange-500 text-white'
                : 'bg-white border-orange-200 focus:border-orange-500 text-gray-900'
              } focus:outline-none focus:ring-1 focus:ring-orange-300`}
            />
            <span className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400">🔍</span>
          </div>
        </div>

        {/* Loading indicator */}
        {loading && (
          <div className="flex justify-center my-8">
            <div className={`animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 ${isDarkMode ? 'border-orange-500' : 'border-orange-600'}`}></div>
          </div>
        )}

        {/* Error message */}
        {error && (
          <div className="max-w-3xl mx-auto mb-8">
            <p className={`text-center p-4 rounded-lg border ${isDarkMode 
              ? 'bg-red-900/50 text-red-200 border-red-800' 
              : 'bg-red-50 text-red-600 border-red-100'}`}>
              {error}
            </p>
          </div>
        )}

        {/* Accounts list */}
        <div className="grid grid-cols-1 gap-3">
          {filteredAccounts.map((account, index) => (
            <div key={index} className={`p-4 rounded-lg shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white'}`}>
              <div className="flex flex-col space-y-2">
                <p className="text-sm break-all">
                  <Link href={`/accounts/${account.address}`}>
                    <span className="font-medium text-orange-500 hover:text-orange-400 cursor-pointer underline">{account.address}</span>
                  </Link>
                </p>
                <div className="flex flex-wrap gap-x-6 gap-y-2">
                  <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-700'}`}>
                    <span className="font-medium">Balance:</span> {account.balance ? formatKariAmount(account.balance) : account.balance_formatted} KARI
                  </p>
                  <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-700'}`}>
                    <span className="font-medium">Transactions:</span> {account.transaction_count}
                  </p>
                  <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-700'}`}>
                    <span className="font-medium">Type:</span> {account.account_type || (account.is_contract ? 'Contract' : 'Wallet')}
                  </p>
                  {account.is_contract && (
                    <p className={`text-sm px-2 py-1 rounded-full ${isDarkMode ? 'bg-amber-900/30 text-amber-400' : 'bg-amber-100 text-amber-600'}`}>
                      Contract Account
                    </p>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Empty state */}
        {!loading && filteredAccounts.length === 0 && !error && (
          <div className="text-center py-10">
            <p className={`text-lg ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
              No accounts found matching your search.
            </p>
          </div>
        )}
        
        {/* Refresh button */}
        <div className="flex justify-center mt-8">
          <button
            onClick={fetchAllAccounts}
            className={`px-6 py-3 rounded-lg font-medium ${isDarkMode
              ? 'bg-gray-800 hover:bg-gray-700 text-white'
              : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200'
            }`}
          >
            Refresh Accounts
          </button>
        </div>
      </div>
    </main>
  );
}
