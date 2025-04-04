"use client";

import React, { useState } from 'react';
import Link from 'next/link';
import { useTheme } from '../contexts/ThemeContext';

interface NavbarProps {
  searchTx: string;
  setSearchTx: (value: string) => void;
  API_URL: string;
  formatKariAmount: (kaAmount: number, showDecimals?: boolean) => string;
  currentPage?: string;
}

const Navbar: React.FC<NavbarProps> = ({
  searchTx,
  setSearchTx,
  API_URL,
  formatKariAmount,
  currentPage = 'home'
}) => {
  const { isDarkMode, toggleTheme } = useTheme();
  const [isMenuOpen, setIsMenuOpen] = useState(false);

  const handleSearchTxChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTx(event.target.value);
  };

  return (
    <nav className={`sticky top-0 z-10 shadow-md ${isDarkMode ? 'bg-gray-800 text-white' : 'bg-white text-gray-800'}`}>
      <div className="container mx-auto max-w-7xl px-4 py-3">
        <div className="flex items-center justify-between">
          {/* Logo/Title */}
          <div className="flex items-center">
            <Link href="/">
              <h1 className={`text-xl md:text-2xl font-bold ${isDarkMode ? 'text-white' : 'text-orange-600'}`}>
                Kanari Explorer
              </h1>
            </Link>
          </div>

          {/* Desktop Navigation */}
          <div className="hidden md:flex items-center space-x-4 flex-grow mx-6">
            <div className="relative flex-grow max-w-3xl">
              <input
                type="text"
                placeholder="Search by transaction ID, sender or receiver..."
                value={searchTx}
                onChange={handleSearchTxChange}
                className={`w-full px-4 py-2 rounded-lg shadow-sm border ${isDarkMode
                  ? 'bg-gray-700 border-gray-600 focus:border-orange-500 text-white'
                  : 'bg-white border-orange-200 focus:border-orange-500 text-gray-900'
                } focus:outline-none focus:ring-1 focus:ring-orange-300`}
              />
              <span className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400">🔍</span>
            </div>
          </div>

          {/* Navigation Buttons */}
          <div className="hidden md:flex items-center space-x-3">
            <Link href="/accounts">
              <span className={`px-4 py-2 rounded-lg text-sm font-medium ${
                currentPage === 'accounts' 
                  ? (isDarkMode ? 'bg-orange-600 text-white' : 'bg-orange-500 text-white') 
                  : (isDarkMode ? 'bg-gray-700 hover:bg-gray-600 text-white' : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200')
              }`}>
                Accounts
              </span>
            </Link>
            
            <Link href="/transactions">
              <span className={`px-4 py-2 rounded-lg text-sm font-medium ${
                currentPage === 'transactions' 
                  ? (isDarkMode ? 'bg-orange-600 text-white' : 'bg-orange-500 text-white') 
                  : (isDarkMode ? 'bg-gray-700 hover:bg-gray-600 text-white' : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200')
              }`}>
                Transactions
              </span>
            </Link>
            
            <Link href="/staking">
              <span className={`px-4 py-2 rounded-lg text-sm font-medium ${
                currentPage === 'staking' 
                  ? (isDarkMode ? 'bg-orange-600 text-white' : 'bg-orange-500 text-white') 
                  : (isDarkMode ? 'bg-gray-700 hover:bg-gray-600 text-white' : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200')
              }`}>
                Staking
              </span>
            </Link>
            
            <button
              onClick={toggleTheme}
              className={`p-2 rounded-lg ${isDarkMode
                ? 'bg-gray-700 hover:bg-gray-600 border-gray-600'
                : 'bg-white hover:bg-orange-50 border-orange-200'
              } border`}
              aria-label="Toggle theme"
            >
              {isDarkMode ? '🌞' : '🌙'}
            </button>
          </div>

          {/* Mobile Menu Button */}
          <div className="md:hidden flex items-center">
            <button
              onClick={() => setIsMenuOpen(!isMenuOpen)}
              className={`p-2 rounded-lg ${isDarkMode ? 'bg-gray-700' : 'bg-orange-50'}`}
            >
              <svg className={`w-6 h-6 ${isDarkMode ? 'text-white' : 'text-orange-600'}`} fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                {isMenuOpen ? (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                )}
              </svg>
            </button>
          </div>
        </div>

        {/* Mobile Search and Menu */}
        {isMenuOpen && (
          <div className="md:hidden pt-3 pb-4 space-y-3">
            <div className="relative">
              <input
                type="text"
                placeholder="Search transactions..."
                value={searchTx}
                onChange={handleSearchTxChange}
                className={`w-full px-4 py-2 rounded-lg border ${isDarkMode
                  ? 'bg-gray-700 border-gray-600 text-white'
                  : 'bg-white border-orange-200 text-gray-900'
                } focus:outline-none`}
              />
              <span className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400">🔍</span>
            </div>
            <div className="flex flex-wrap gap-2">
              <Link href="/accounts" className="flex-1">
                <span className={`block text-center px-4 py-2 rounded-lg text-sm font-medium ${
                  currentPage === 'accounts' 
                    ? (isDarkMode ? 'bg-orange-600 text-white' : 'bg-orange-500 text-white') 
                    : (isDarkMode ? 'bg-gray-700 hover:bg-gray-600 text-white' : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200')
                }`}>
                  Accounts
                </span>
              </Link>
              
              <Link href="/transactions" className="flex-1">
                <span className={`block text-center px-4 py-2 rounded-lg text-sm font-medium ${
                  currentPage === 'transactions' 
                    ? (isDarkMode ? 'bg-orange-600 text-white' : 'bg-orange-500 text-white') 
                    : (isDarkMode ? 'bg-gray-700 hover:bg-gray-600 text-white' : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200')
                }`}>
                  Transactions
                </span>
              </Link>
              
              <Link href="/staking" className="flex-1">
                <span className={`block text-center px-4 py-2 rounded-lg text-sm font-medium ${
                  currentPage === 'staking' 
                    ? (isDarkMode ? 'bg-orange-600 text-white' : 'bg-orange-500 text-white') 
                    : (isDarkMode ? 'bg-gray-700 hover:bg-gray-600 text-white' : 'bg-orange-50 hover:bg-orange-100 text-orange-600 border border-orange-200')
                }`}>
                  Staking
                </span>
              </Link>
              
              <button
                onClick={toggleTheme}
                className={`px-4 py-2 rounded-lg ${isDarkMode
                  ? 'bg-gray-700 hover:bg-gray-600 border-gray-600'
                  : 'bg-white hover:bg-orange-50 border-orange-200'
                } border`}
              >
                {isDarkMode ? '🌞' : '🌙'}
              </button>
            </div>
          </div>
        )}
      </div>
    </nav>
  );
};

export default Navbar;
