"use client";

import React, { useState, useEffect } from 'react';
import axios from 'axios';
import Navbar from '../../components/Navbar';
import Link from 'next/link';
import { useTheme } from '../../contexts/ThemeContext';

interface StakingStats {
  total_staked_amount: number;
  total_staked_amount_formatted: string;
  total_validators: number;
  total_nodes: number;
  average_reward_rate: number;
  latest_rewards_distributed: number;
  latest_rewards_distributed_formatted: string;
}

interface StakingInfo {
  address: string;
  is_staking: boolean;
  minimum_staking_amount: number;
  minimum_staking_formatted: string;
  minimum_validator_amount: number;
  minimum_validator_formatted: string;
  staked_amount?: number;
  staked_amount_formatted?: string;
  is_validator?: boolean;
  rewards_earned?: number;
  rewards_earned_formatted?: string;
  stake_date?: number;
  unlock_date?: number;
  status?: string;
}

const formatKariAmount = (kaAmount: number, showDecimals: boolean = true): string => {
  const KA_PER_KARI: number = 1_000_000_000;

  const wholeKari = Math.floor(kaAmount / KA_PER_KARI);
  const fractionalKa = kaAmount % KA_PER_KARI;

  const wholeFormatted = wholeKari.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");

  return showDecimals
    ? `${wholeFormatted}.${fractionalKa.toString().padStart(9, '0')}`
    : wholeFormatted;
};

export default function StakingPage() {
  const { isDarkMode } = useTheme();
  const [searchTx, setSearchTx] = useState('');
  const [stakingStats, setStakingStats] = useState<StakingStats | null>(null);
  const [stakingInfo, setStakingInfo] = useState<StakingInfo | null>(null);
  const [searchAddress, setSearchAddress] = useState('');
  const [loading, setLoading] = useState(false);
  const [statsLoading, setStatsLoading] = useState(true);
  const [error, setError] = useState('');

  const API_URL = 'http://192.168.1.103:30031';

  const fetchStakingStats = async () => {
    try {
      setStatsLoading(true);
      const response = await axios.post(
        API_URL,
        {
          jsonrpc: "2.0",
          method: "get_staking_stats",
          params: [],
          id: 1,
        },
        {
          headers: {
            'Content-Type': 'application/json',
          },
        }
      );

      if (response.data.result) {
        console.log("Staking stats response:", response.data.result);
        const stats = {
          total_staked_amount: 0,
          total_staked_amount_formatted: "0.000000000",
          total_validators: 0,
          total_nodes: 0,
          average_reward_rate: 0.01,
          latest_rewards_distributed: 0,
          latest_rewards_distributed_formatted: "0.000000000",
          ...response.data.result
        };
        setStakingStats(stats);
      }
    } catch (error) {
      console.error('Error fetching staking stats:', error);
    } finally {
      setStatsLoading(false);
    }
  };

  const fetchStakingInfo = async () => {
    if (!searchAddress) {
      setError('Please enter an address to check staking info');
      return;
    }

    try {
      setLoading(true);
      setError('');

      const response = await axios.post(
        API_URL,
        {
          jsonrpc: "2.0",
          method: "get_staking_info",
          params: [searchAddress],
          id: 1,
        },
        {
          headers: {
            'Content-Type': 'application/json',
          },
        }
      );

      if (response.data.result) {
        setStakingInfo(response.data.result);
      } else if (response.data.error) {
        setError(response.data.error.message || 'No staking information found for this address');
        setStakingInfo(null);
      }
    } catch (error) {
      console.error('Error fetching staking info:', error);
      setError('An error occurred while fetching staking information');
      setStakingInfo(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStakingStats();

    const intervalId = setInterval(fetchStakingStats, 30000);

    return () => clearInterval(intervalId);
  }, []);

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    fetchStakingInfo();
  };

  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const calculateTimeRemaining = (unlockDate: number): string => {
    const now = Math.floor(Date.now() / 1000);
    const remainingSecs = unlockDate - now;

    if (remainingSecs <= 0) {
      return 'Unlocked';
    }

    const days = Math.floor(remainingSecs / 86400);
    const hours = Math.floor((remainingSecs % 86400) / 3600);
    const minutes = Math.floor((remainingSecs % 3600) / 60);

    return `${days}d ${hours}h ${minutes}m`;
  };

  const safeNumberDisplay = (value: number | undefined | null, defaultValue: number = 0): string => {
    if (value === undefined || value === null || isNaN(value)) {
      return defaultValue.toString();
    }
    return value.toString();
  };

  const safeKariAmount = (amount: number | undefined | null, formattedAmount: string | undefined | null): string => {
    if (formattedAmount && formattedAmount !== "NaN.000000NaN") {
      return formattedAmount;
    }
    
    if (amount === undefined || amount === null || isNaN(amount)) {
      return "0.000000000";
    }
    
    return formatKariAmount(amount);
  };

  return (
    <main className={`min-h-screen ${isDarkMode ? 'bg-gray-900 text-white' : 'bg-gradient-to-r from-orange-50 to-yellow-50 text-gray-800'}`}>
      <Navbar
        searchTx={searchTx}
        setSearchTx={setSearchTx}
        API_URL={API_URL}
        formatKariAmount={formatKariAmount}
        currentPage="staking"
      />

      <div className="container mx-auto max-w-7xl px-4 py-6">
        <h1 className={`text-4xl font-bold mb-6 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
          Staking
        </h1>

        <div className={`mb-12 p-6 rounded-xl shadow-lg ${isDarkMode
          ? 'bg-gradient-to-r from-orange-900/30 to-amber-900/30 border border-orange-800/50'
          : 'bg-gradient-to-r from-orange-100 to-amber-100 border border-orange-200'}`}>
          <h2 className={`text-2xl font-bold mb-6 ${isDarkMode ? 'text-orange-400' : 'text-orange-700'}`}>
            Network Staking Statistics
          </h2>

          {statsLoading ? (
            <div className="flex justify-center py-8">
              <div className={`animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 ${isDarkMode ? 'border-orange-500' : 'border-orange-600'}`}></div>
            </div>
          ) : stakingStats ? (
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
                <p className={`text-orange-500 text-lg mb-1`}>Total Staked</p>
                <p className={`text-2xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                  {safeKariAmount(stakingStats.total_staked_amount, stakingStats.total_staked_amount_formatted)} KARI
                </p>
              </div>

              <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
                <p className={`text-orange-600 text-lg mb-1`}>Validators / Nodes</p>
                <p className={`text-2xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                  {safeNumberDisplay(stakingStats.total_validators)} / {safeNumberDisplay(stakingStats.total_nodes)}
                </p>
              </div>

              <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
                <p className={`text-orange-700 text-lg mb-1`}>Average Reward Rate</p>
                <p className={`text-2xl font-bold ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                  {safeNumberDisplay(stakingStats.average_reward_rate, 0.01)}%
                </p>
              </div>
            </div>
          ) : (
            <div className="text-center py-6">
              <p className={`text-lg ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
                Staking statistics unavailable
              </p>
            </div>
          )}
        </div>

        <div className="mb-8">
          <h2 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
            Check Your Staking Status
          </h2>

          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <form onSubmit={handleSearchSubmit} className="flex flex-col md:flex-row gap-4">
              <div className="flex-grow">
                <input
                  type="text"
                  placeholder="Enter your address to check staking status"
                  value={searchAddress}
                  onChange={(e) => setSearchAddress(e.target.value)}
                  className={`w-full px-4 py-3 rounded-lg shadow-sm border ${isDarkMode
                    ? 'bg-gray-700 border-gray-600 focus:border-orange-500 text-white'
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
                Check Status
              </button>
            </form>

            {loading && (
              <div className="flex justify-center my-8">
                <div className={`animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 ${isDarkMode ? 'border-orange-500' : 'border-orange-600'}`}></div>
              </div>
            )}

            {error && (
              <div className="mt-6">
                <p className={`text-center p-4 rounded-lg border ${isDarkMode
                  ? 'bg-red-900/50 text-red-200 border-red-800'
                  : 'bg-red-50 text-red-600 border-red-100'}`}>
                  {error}
                </p>
              </div>
            )}

            {stakingInfo && !loading && (
              <div className="mt-8">
                <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-700' : 'bg-orange-50 border border-orange-100'}`}>
                  <h3 className={`text-xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
                    Staking Information
                  </h3>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-4">
                    <div>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Address:</p>
                      <p className="text-sm break-all font-mono">{stakingInfo.address}</p>
                    </div>

                    <div>
                      <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Status:</p>
                      <p className={`inline-block px-3 py-1 rounded-full text-sm ${
                        stakingInfo.is_staking
                          ? (isDarkMode ? 'bg-green-900/30 text-green-400' : 'bg-green-100 text-green-600')
                          : (isDarkMode ? 'bg-yellow-900/30 text-yellow-400' : 'bg-yellow-100 text-yellow-600')
                      }`}>
                        {stakingInfo.is_staking ? 'Staking' : 'Not Staking'}
                      </p>
                    </div>
                  </div>

                  {stakingInfo.is_staking ? (
                    <>
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-4">
                        <div>
                          <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Staked Amount:</p>
                          <p className="text-xl font-bold text-orange-500">
                            {stakingInfo.staked_amount_formatted || formatKariAmount(stakingInfo.staked_amount || 0)} KARI
                          </p>
                        </div>

                        <div>
                          <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Validator:</p>
                          <p className={`text-lg ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                            {stakingInfo.is_validator ? 'Yes' : 'No'}
                          </p>
                        </div>
                      </div>

                      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-4">
                        <div>
                          <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Rewards Earned:</p>
                          <p className={`text-lg font-medium ${isDarkMode ? 'text-green-400' : 'text-green-600'}`}>
                            {stakingInfo.rewards_earned_formatted || formatKariAmount(stakingInfo.rewards_earned || 0)} KARI
                          </p>
                        </div>

                        <div>
                          <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Stake Date:</p>
                          <p className={`text-sm ${isDarkMode ? 'text-white' : 'text-gray-800'}`}>
                            {stakingInfo.stake_date ? formatDate(stakingInfo.stake_date) : 'N/A'}
                          </p>
                        </div>
                      </div>

                      <div>
                        <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>Unlock Date:</p>
                        <div className="flex items-center">
                          <p className={`text-sm ${isDarkMode ? 'text-white' : 'text-gray-800'} mr-3`}>
                            {stakingInfo.unlock_date ? formatDate(stakingInfo.unlock_date) : 'N/A'}
                          </p>
                          {stakingInfo.unlock_date && (
                            <span className={`px-2 py-1 text-xs rounded-full ${
                              calculateTimeRemaining(stakingInfo.unlock_date) === 'Unlocked'
                                ? (isDarkMode ? 'bg-green-900/30 text-green-400' : 'bg-green-100 text-green-600')
                                : (isDarkMode ? 'bg-orange-900/30 text-orange-400' : 'bg-orange-100 text-orange-600')
                            }`}>
                              {calculateTimeRemaining(stakingInfo.unlock_date)}
                            </span>
                          )}
                        </div>
                      </div>
                    </>
                  ) : (
                    <div className="mt-4">
                      <div className={`p-4 rounded-lg border ${isDarkMode
                        ? 'bg-blue-900/30 text-blue-200 border-blue-800'
                        : 'bg-blue-50 text-blue-600 border-blue-100'}`}>
                        <p className="mb-2">This address is not currently staking.</p>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
                          <div>
                            <p className="font-medium">Required to run a node:</p>
                            <p className="text-lg font-bold mt-1">
                              {stakingInfo.minimum_staking_formatted} KARI
                            </p>
                          </div>
                          <div>
                            <p className="font-medium">Required to be a validator:</p>
                            <p className="text-lg font-bold mt-1">
                              {stakingInfo.minimum_validator_formatted} KARI
                            </p>
                          </div>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="mb-8">
          <h2 className={`text-2xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
            Staking Requirements
          </h2>

          <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
              <div>
                <h3 className={`text-xl font-medium mb-4 ${isDarkMode ? 'text-orange-400' : 'text-orange-600'}`}>
                  Node Operator
                </h3>
                <ul className="space-y-2">
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>Minimum stake: 200 KARI</span>
                  </li>
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>24-hour lock period for all staked tokens</span>
                  </li>
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>Early unstaking incurs a 10% penalty</span>
                  </li>
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>No rewards - helps secure the network</span>
                  </li>
                </ul>
              </div>

              <div>
                <h3 className={`text-xl font-medium mb-4 ${isDarkMode ? 'text-orange-400' : 'text-orange-600'}`}>
                  Validator
                </h3>
                <ul className="space-y-2">
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>Minimum stake: 32 KARI</span>
                  </li>
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>24-hour lock period for all staked tokens</span>
                  </li>
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>Earn 0.01% annual rewards on staked amount</span>
                  </li>
                  <li className="flex items-start">
                    <span className={`inline-block w-5 h-5 rounded-full flex items-center justify-center mt-1 mr-2 ${
                      isDarkMode ? 'bg-orange-700 text-white' : 'bg-orange-100 text-orange-700'
                    }`}>•</span>
                    <span>Participate in consensus and earn additional gas rewards</span>
                  </li>
                </ul>
              </div>
            </div>

            <div className="mt-8 p-4 rounded-lg bg-opacity-50 text-sm border-l-4 border-orange-500 bg-orange-500/10">
              <p className="mb-2"><strong>Note:</strong> To stake tokens, you need to run a command using the Kanari SDK:</p>
              <pre className={`p-3 rounded-md overflow-x-auto ${isDarkMode ? 'bg-gray-900 text-gray-300' : 'bg-gray-50 text-gray-800'}`}>
                {`curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "stake_tokens",
  "params": {
    "address": "YOUR_ACCOUNT_ADDRESS",
    "amount": 200,
    "password": "YOUR_PASSWORD",
    "validator": true
  },
  "id": 1
}' http://127.0.0.1:30031`}
              </pre>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
