"use client";

import React from 'react';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend,
} from 'chart.js';
import { Bar } from 'react-chartjs-2';

// Register Chart.js components
ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend
);

interface Transaction {
  sender: string;
  receiver: string;
  amount: number;
  block_index?: number;
  timestamp?: number;
  hash?: string;
}

interface TransactionChartProps {
  transactions: Transaction[];
  accountAddress: string;
  isDarkMode: boolean;
}

const TransactionChart: React.FC<TransactionChartProps> = ({ 
  transactions, 
  accountAddress,
  isDarkMode 
}) => {
  // If no transactions, return early
  if (!transactions || transactions.length === 0) {
    return (
      <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
        <p className={`text-center ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
          No transaction data available for visualization
        </p>
      </div>
    );
  }

  // Get the latest transaction (we'll use this for the single transaction view)
  const latestTransaction = transactions[0]; // Assuming transactions are sorted with newest first

  // Determine if the transaction is received by the account
  const isReceived = latestTransaction.receiver === accountAddress;
  
  // Set up colors based on transaction type and theme
  const backgroundColor = isReceived 
    ? isDarkMode ? 'rgba(52, 211, 153, 0.7)' : 'rgba(16, 185, 129, 0.7)' // Green for received
    : isDarkMode ? 'rgba(248, 113, 113, 0.7)' : 'rgba(239, 68, 68, 0.7);'; // Red for sent
  
  const borderColor = isReceived 
    ? isDarkMode ? 'rgb(52, 211, 153)' : 'rgb(16, 185, 129)' 
    : isDarkMode ? 'rgb(248, 113, 113)' : 'rgb(239, 68, 68)';
    
  const textColor = isDarkMode ? 'white' : 'black';

  // Chart configuration
  const options = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'top' as const,
        labels: {
          color: textColor
        }
      },
      title: {
        display: true,
        text: 'Latest Transaction',
        color: textColor,
        font: {
          size: 16,
          weight: 'bold' as const
        }
      },
      tooltip: {
        callbacks: {
          label: function(context: any) {
            return `${context.parsed.y} KARI`;
          }
        }
      }
    },
    scales: {
      x: {
        ticks: {
          color: textColor
        },
        grid: {
          color: isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
        }
      },
      y: {
        ticks: {
          color: textColor
        },
        grid: {
          color: isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
        }
      }
    }
  };

  // Format amount to show in KARI units
  const KA_PER_KARI: number = 1_000_000_000;
  const amountInKari = latestTransaction.amount / KA_PER_KARI;

  // Chart data
  const data = {
    labels: [isReceived ? 'Received' : 'Sent'],
    datasets: [
      {
        label: `Transaction Amount (KARI)`,
        data: [amountInKari],
        backgroundColor: backgroundColor,
        borderColor: borderColor,
        borderWidth: 1,
      }
    ],
  };

  return (
    <div className={`p-6 rounded-xl shadow-sm ${isDarkMode ? 'bg-gray-800' : 'bg-white border border-orange-100'}`}>
      <h3 className={`text-xl font-bold mb-4 ${isDarkMode ? 'text-white' : 'text-orange-700'}`}>
        Transaction Visualization
      </h3>
      <div className="h-64">
        <Bar options={options} data={data} />
      </div>
      <div className="mt-4">
        <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
          <span className="font-medium">Type:</span> {isReceived ? 'Received from' : 'Sent to'} 
          <span className="font-mono mx-1">{isReceived ? latestTransaction.sender.substring(0, 12) : latestTransaction.receiver.substring(0, 12)}...</span>
        </p>
        {latestTransaction.block_index && (
          <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
            <span className="font-medium">Block:</span> {latestTransaction.block_index}
          </p>
        )}
        {latestTransaction.timestamp && (
          <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
            <span className="font-medium">Time:</span> {new Date(latestTransaction.timestamp * 1000).toLocaleString()}
          </p>
        )}
      </div>
    </div>
  );
};

export default TransactionChart;
