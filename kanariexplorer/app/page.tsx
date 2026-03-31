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
      {/* Hero Search Section */}
      <section className="w-full pt-32 pb-20 px-6 flex flex-col items-center justify-center relative">
        <div className="absolute inset-0 bg-[url('https://grainy-gradients.vercel.app/noise.svg')] opacity-20 pointer-events-none mix-blend-overlay"></div>

        <h1 className="text-5xl md:text-7xl font-black mb-8 tracking-tighter text-transparent bg-clip-text bg-linear-to-r from-emerald-400 via-cyan-400 to-blue-500 drop-shadow-sm text-center">
          Kanari Explorer
        </h1>
        <p className="text-zinc-400 mb-10 text-lg md:text-xl text-center max-w-2xl font-light">
          Explore transactions, tokens, and accounts on the fast and secure Kanari Network.
        </p>

        <form onSubmit={handleSearch} className="w-full max-w-3xl relative z-10">
          <div className="flex items-center bg-[#111113]/80 backdrop-blur-xl border border-white/10 rounded-2xl p-2 shadow-2xl focus-within:border-emerald-500/50 focus-within:ring-4 focus-within:ring-emerald-500/10 transition-all duration-300">
            <svg className="w-6 h-6 text-zinc-500 ml-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search by Address, Txn Hash, or Module..."
              className="w-full bg-transparent text-white px-4 py-4 outline-none placeholder:text-zinc-600 text-base md:text-lg font-mono"
            />
            <button type="submit" className="bg-white hover:bg-zinc-200 text-black px-8 py-3.5 rounded-xl font-bold text-sm transition-colors shadow-lg shadow-white/10">
              Search
            </button>
          </div>
        </form>
      </section>

      {/* Stats Section */}
      <section className="w-full max-w-7xl mx-auto px-6 py-12">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">

          <div className="bg-[#111113]/60 backdrop-blur-md border border-white/5 hover:border-white/10 p-8 rounded-3xl flex flex-col transition-all duration-300 hover:-translate-y-1 hover:shadow-xl hover:shadow-black/50">
            <div className="w-12 h-12 bg-blue-500/10 text-blue-400 rounded-2xl flex items-center justify-center mb-6">
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
            </div>
            <h3 className="text-zinc-500 text-sm font-bold uppercase tracking-widest mb-2">Total Tokens</h3>
            <div className="text-4xl font-mono font-bold text-white mb-2">{tokenCount !== null ? tokenCount : "-"}</div>
            <div className="mt-auto pt-6 border-t border-white/5">
              <Link href="/coins" className="text-sm text-emerald-400 hover:text-emerald-300 font-medium flex items-center gap-1">View all tokens <span className="text-lg">→</span></Link>
            </div>
          </div>

          <div className="bg-[#111113]/60 backdrop-blur-md border border-white/5 hover:border-white/10 p-8 rounded-3xl flex flex-col transition-all duration-300 hover:-translate-y-1 hover:shadow-xl hover:shadow-black/50">
            <div className="w-12 h-12 bg-purple-500/10 text-purple-400 rounded-2xl flex items-center justify-center mb-6">
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path></svg>
            </div>
            <h3 className="text-zinc-500 text-sm font-bold uppercase tracking-widest mb-2">Transactions</h3>
            <div className="text-4xl font-mono font-bold text-white mb-2">Live</div>
            <div className="mt-auto pt-6 border-t border-white/5">
              <Link href="/tx" className="text-sm text-emerald-400 hover:text-emerald-300 font-medium flex items-center gap-1">View recent activity <span className="text-lg">→</span></Link>
            </div>
          </div>

          <div className="bg-[#111113]/60 backdrop-blur-md border border-white/5 hover:border-white/10 p-8 rounded-3xl flex flex-col transition-all duration-300 hover:-translate-y-1 hover:shadow-xl hover:shadow-black/50 relative overflow-hidden">
            <div className="w-12 h-12 bg-emerald-500/10 text-emerald-400 rounded-2xl flex items-center justify-center mb-6">
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"></path></svg>
            </div>
            <h3 className="text-zinc-500 text-sm font-bold uppercase tracking-widest mb-2 flex items-center gap-2">
              Network Status
              <span className={`w-2.5 h-2.5 rounded-full shadow-lg ${isOnline ? 'bg-emerald-500 shadow-emerald-500/50' : 'bg-red-500 shadow-red-500/50'}`}></span>
            </h3>
            <div className="text-4xl font-mono font-bold text-white mb-1">
              {blockHeight !== null ? blockHeight.toLocaleString() : "-"}
            </div>
            <div className="text-sm text-zinc-500">Current Block Height</div>
            <div className="mt-auto pt-6 border-t border-white/5">
              <span className={`text-sm font-medium ${isOnline ? 'text-emerald-400' : 'text-red-400'}`}>
                {isOnline ? "Mainnet is operating normally" : "Connection lost"}
              </span>
            </div>
          </div>

        </div>
      </section>
    </div>
  );
}