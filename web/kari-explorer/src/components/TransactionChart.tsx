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

  // Format amount to show in KARI units
  const KA_PER_KARI: number = 1_000_000_000;
  
  // Process all transactions
  const receivedTransactions = transactions.filter(tx => tx.receiver === accountAddress);
  const sentTransactions = transactions.filter(tx => tx.sender === accountAddress);
  
  // Calculate total amount received and sent
  const totalReceived = receivedTransactions.reduce((sum, tx) => sum + tx.amount / KA_PER_KARI, 0);
  const totalSent = sentTransactions.reduce((sum, tx) => sum + tx.amount / KA_PER_KARI, 0);
  
  // Chart data for all transactions
  const data = {
    labels: ['Received', 'Sent'],
    datasets: [
      {
        label: `Transaction Amount (KARI)`,
        data: [totalReceived, totalSent],
        backgroundColor: [
          isDarkMode ? 'rgba(52, 211, 153, 0.7)' : 'rgba(16, 185, 129, 0.7)', // Green for received
          isDarkMode ? 'rgba(248, 113, 113, 0.7)' : 'rgba(239, 68, 68, 0.7)', // Red for sent
        ],
        borderColor: [
          isDarkMode ? 'rgb(52, 211, 153)' : 'rgb(16, 185, 129)',
          isDarkMode ? 'rgb(248, 113, 113)' : 'rgb(239, 68, 68)',
        ],
        borderWidth: 1,
      }
    ],
  };

  // Update chart options for all transactions
  const options = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'top' as const,
        labels: {
          color: isDarkMode ? 'white' : 'black'
        }
      },
      title: {
        display: true,
        text: 'All Transactions Summary',
        color: isDarkMode ? 'white' : 'black',
        font: {
          size: 16,
          weight: 'bold' as const
        }
      },
      tooltip: {
        callbacks: {
          label: function(context: any) {
            return `${context.parsed.y.toFixed(2)} KARI`;
          }
        }
      }
    },
    scales: {
      x: {
        ticks: {
          color: isDarkMode ? 'white' : 'black'
        },
        grid: {
          color: isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
        }
      },
      y: {
        ticks: {
          color: isDarkMode ? 'white' : 'black'
        },
        grid: {
          color: isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
        }
      }
    }
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
          <span className="font-medium">Total Transactions:</span> {transactions.length}
        </p>
        <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
          <span className="font-medium">Received:</span> {receivedTransactions.length} transactions ({totalReceived.toFixed(2)} KARI)
        </p>
        <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
          <span className="font-medium">Sent:</span> {sentTransactions.length} transactions ({totalSent.toFixed(2)} KARI)
        </p>
        {transactions.length > 0 && (
          <p className={`text-sm ${isDarkMode ? 'text-gray-400' : 'text-gray-600'}`}>
            <span className="font-medium">Latest Transaction:</span> {new Date(transactions[0].timestamp! * 1000).toLocaleString()}
          </p>
        )}
      </div>
    </div>
  );
};

export default TransactionChart;
