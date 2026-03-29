"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { getTokens, getBlockHeight } from "./lib/rpc";

export default function Home() {
  const [search, setSearch] = useState("");
  const router = useRouter();

  const [tokenCount, setTokenCount] = useState<number | null>(null);
  const [blockHeight, setBlockHeight] = useState<number | null>(null);
  const [isOnline, setIsOnline] = useState<boolean>(false);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (search.trim()) router.push(`/account?address=${search.trim()}`);
  };

  useEffect(() => {
    async function fetchNetworkData() {
      try {
        const [tokensRes, heightRes] = await Promise.all([
          getTokens().catch(() => null),
          getBlockHeight().catch(() => null)
        ]);

        if (tokensRes) {
          let count = 0;
          if (Array.isArray(tokensRes)) count = tokensRes.length;
          else if (tokensRes.result && Array.isArray(tokensRes.result)) count = tokensRes.result.length;
          setTokenCount(count);
          setIsOnline(true);
        }

        if (heightRes !== null && heightRes !== undefined) {
          const h = typeof heightRes === 'object' ? heightRes.height : heightRes;
          setBlockHeight(Number(h));
          setIsOnline(true);
        }
      } catch (e) {
        setIsOnline(false);
      }
    }
    fetchNetworkData();
    const interval = setInterval(fetchNetworkData, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex flex-col items-center w-full">
      {/* Search Section */}
      <section className="w-full bg-[#0a0a0a] border-b border-zinc-900 py-24 px-6 flex flex-col items-center justify-center">
        <h1 className="text-4xl md:text-5xl font-black mb-8 tracking-tight text-white">
          Kanari Explorer
        </h1>

        <form onSubmit={handleSearch} className="w-full max-w-3xl">
          <div className="flex items-center bg-black border border-zinc-800 rounded-lg p-1.5 focus-within:border-zinc-500 transition-colors">
            <svg className="w-5 h-5 text-zinc-500 ml-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search by Address, Txn Hash, or Module..."
              className="w-full bg-transparent text-zinc-100 px-4 py-3 outline-none placeholder:text-zinc-600 text-sm font-mono"
            />
            <button type="submit" className="bg-zinc-100 hover:bg-white text-black px-6 py-2.5 rounded-md font-bold text-sm transition-colors">
              Search
            </button>
          </div>
        </form>
      </section>

      {/* Stats Section */}
      <section className="w-full max-w-7xl mx-auto px-6 py-16">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">

          <div className="bg-[#111] border border-zinc-800 p-6 rounded-xl flex flex-col">
            <h3 className="text-zinc-500 text-xs font-bold uppercase tracking-widest mb-4">Total Tokens</h3>
            <div className="text-3xl font-mono text-white mb-2">{tokenCount !== null ? tokenCount : "-"}</div>
            <div className="mt-auto pt-4 border-t border-zinc-800">
              <Link href="/coins" className="text-sm text-blue-400 hover:text-blue-300">View all tokens →</Link>
            </div>
          </div>

          <div className="bg-[#111] border border-zinc-800 p-6 rounded-xl flex flex-col">
            <h3 className="text-zinc-500 text-xs font-bold uppercase tracking-widest mb-4">Transactions</h3>
            <div className="text-3xl font-mono text-white mb-2">Live</div>
            <div className="mt-auto pt-4 border-t border-zinc-800">
              <Link href="/tx" className="text-sm text-blue-400 hover:text-blue-300">View recent activity →</Link>
            </div>
          </div>

          <div className="bg-[#111] border border-zinc-800 p-6 rounded-xl flex flex-col">
            <h3 className="text-zinc-500 text-xs font-bold uppercase tracking-widest mb-4 flex items-center gap-2">
              Network Status
              <span className={`w-2 h-2 rounded-full ${isOnline ? 'bg-emerald-500' : 'bg-red-500'}`}></span>
            </h3>
            <div className="text-3xl font-mono text-white mb-1">
              {blockHeight !== null ? blockHeight.toLocaleString() : "-"}
            </div>
            <div className="text-xs text-zinc-500 font-mono">Current Block Height</div>
            <div className="mt-auto pt-4 border-t border-zinc-800">
              <span className={`text-sm ${isOnline ? 'text-emerald-400' : 'text-red-400'}`}>
                {isOnline ? "Mainnet is operating normally" : "Connection lost"}
              </span>
            </div>
          </div>

        </div>
      </section>
    </div>
  );
}