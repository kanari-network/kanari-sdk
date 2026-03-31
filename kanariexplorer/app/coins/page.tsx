"use client";

import { useEffect, useState, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { getAllBalances, getTokens } from "../lib/rpc";

function CoinsContent() {
  const [address, setAddress] = useState("");
  const [balances, setBalances] = useState<any[]>([]);
  const [globalTokens, setGlobalTokens] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => { loadGlobalTokens(); }, []);
  useEffect(() => {
    if (address.length > 10) {
      const t = setTimeout(() => fetchBalances(address), 500);
      return () => clearTimeout(t);
    }
    else if (!address) setBalances(globalTokens);
  }, [address, globalTokens]);

  async function loadGlobalTokens() {
    try {
      const t = await getTokens();
      let arr = Array.isArray(t) ? t : (t?.result ? t.result : []);
      setGlobalTokens(arr);
      if (!address) setBalances(arr);
    } catch (e) { }
  }

  async function fetchBalances(addr: string) {
    setLoading(true);
    try {
      const data = await getAllBalances(addr);
      setBalances(Array.isArray(data?.result) ? data.result : (Array.isArray(data) ? data : []));
    } catch (e) { setBalances([]); } finally { setLoading(false); }
  }

  function fmtBalance(n: any, decimals = 9) {
    if (n == null) return "-";
    try {
      const num = BigInt(n.toString());
      const pow = BigInt(10) ** BigInt(decimals);
      const wholeStr = String(num / pow).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
      const frac = num % pow;
      if (frac === BigInt(0)) return wholeStr;
      return `${wholeStr}.${String(frac).padStart(decimals, "0").replace(/0+$/, "")}`;
    } catch { return String(n); }
  }

  return (
    <div className="max-w-7xl mx-auto px-6 py-12 w-full relative">

      {/* 👤 Header Card Style - Emerald & Cyan Theme */}
      <div className="mb-12 bg-[#111113]/60 backdrop-blur-md border border-white/5 p-8 rounded-[40px] shadow-lg relative overflow-hidden">
        {/* Glow Effect */}
        <div className="absolute -top-24 -right-24 w-64 h-64 bg-emerald-500/10 blur-[80px] rounded-full pointer-events-none"></div>

        <div className="flex flex-col md:flex-row md:items-center justify-between gap-8 relative z-10">
          <div className="flex items-center gap-6">
            <div className="w-20 h-20 rounded-[28px] bg-linear-to-tr from-emerald-400 to-cyan-500 flex items-center justify-center shadow-lg shadow-emerald-500/20">
              <svg className="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
            </div>
            <div>
              <h2 className="text-zinc-500 text-[10px] font-black uppercase tracking-[0.2em] mb-2">Network Assets</h2>
              <h1 className="text-3xl md:text-4xl font-black text-white tracking-tighter">Token Explorer</h1>
              <div className="mt-3 inline-flex items-center gap-2 px-3 py-1 bg-white/5 border border-white/10 rounded-full text-[10px] font-mono text-zinc-400">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                {address ? `Viewing Balances for ${address.slice(0, 6)}...${address.slice(-4)}` : `${balances.length} Active Tokens In Registry`}
              </div>
            </div>
          </div>

          {/* Search Input - Emerald Focused */}
          <div className="w-full md:w-100">
            <div className="flex bg-black/40 backdrop-blur-md border border-white/10 rounded-2xl p-1.5 focus-within:border-emerald-500/50 transition-all shadow-inner">
              <div className="pl-3 flex items-center justify-center">
                <svg className="w-4 h-4 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
              </div>
              <input
                value={address}
                onChange={(e) => setAddress(e.target.value)}
                placeholder="Filter by Address (0x...)"
                className="w-full bg-transparent text-white px-3 py-2 text-xs focus:outline-none font-mono placeholder:text-zinc-700"
              />
            </div>
          </div>
        </div>
      </div>

      {/* 📊 Token List - ใช้ 2-Column Grid Style แบบ Account Portfolio */}
      <div className="bg-[#111113]/60 backdrop-blur-md rounded-4xl border border-white/5 overflow-hidden shadow-2xl">
        {loading ? (
          <div className="p-24 text-center text-zinc-500 font-mono text-sm flex flex-col items-center gap-4">
            <div className="w-8 h-8 border-4 border-emerald-500/20 border-t-emerald-500 rounded-full animate-spin"></div>
            Syncing registry assets...
          </div>
        ) : balances.length === 0 ? (
          <div className="p-24 text-center text-zinc-600 font-mono text-sm">No tokens found in this context.</div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 divide-y md:divide-y-0 md:divide-x divide-white/5">
            {balances.map((b, i) => {
              const symbol = b.symbol || "UNK";
              return (
                <div key={i} className="p-6 flex justify-between items-center hover:bg-white/2 transition-colors border-b border-white/5 group">
                  <div className="flex items-center gap-4">
                    <div className="w-12 h-12 rounded-2xl bg-linear-to-br from-zinc-800 to-zinc-900 border border-white/10 flex items-center justify-center overflow-hidden shadow-inner group-hover:border-emerald-500/30 transition-all duration-300">
                      {b.icon_url ? (
                        <img src={b.icon_url} className="w-full h-full object-cover" alt="icon" />
                      ) : (
                        <span className="text-sm font-bold text-zinc-600">{symbol.charAt(0)}</span>
                      )}
                    </div>
                    <div>
                      <div className="text-white text-sm font-bold group-hover:text-emerald-400 transition-colors">{b.name ?? symbol}</div>
                      <div className="text-[10px] text-zinc-500 font-mono uppercase tracking-widest mt-0.5">{symbol}</div>
                      <div className="mt-2 text-[9px] px-2 py-0.5 bg-white/5 rounded-md text-zinc-600 inline-block font-mono truncate max-w-37.5">
                        {b.token_type ?? b.token}
                      </div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-white font-mono font-bold text-lg group-hover:text-cyan-400 transition-colors">
                      {fmtBalance(b.balance ?? b.total_supply, b.decimals)}
                    </div>
                    <div className="text-[9px] text-zinc-600 font-black uppercase tracking-tighter mt-1">
                      {address ? "Confirmed Balance" : "Total Supply"}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* 🛠 Developer Info - Emerald JSON Style */}
      <details className="group mt-12 border-t border-white/5 pt-10">
        <summary className="list-none cursor-pointer flex items-center gap-3 text-zinc-600 hover:text-emerald-400 transition-colors">
          <div className="w-6 h-6 rounded-lg bg-white/5 flex items-center justify-center group-open:rotate-90 transition-transform">
            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M9 5l7 7-7 7" /></svg>
          </div>
          <span className="text-[10px] font-black uppercase tracking-[0.3em]">Developer: Token Registry Data</span>
        </summary>
        <div className="mt-6 bg-[#09090b]/80 backdrop-blur-md rounded-3xl border border-white/5 p-8 max-h-100 overflow-auto custom-scrollbar shadow-inner">
          <pre className="text-[11px] text-emerald-500/60 font-mono leading-relaxed">
            {JSON.stringify(balances, null, 2)}
          </pre>
        </div>
      </details>
    </div>
  );
}

export default function CoinsPage() {
  return (
    <Suspense fallback={<div className="p-32 text-center font-mono text-zinc-600 animate-pulse">Loading Explorer...</div>}>
      <CoinsContent />
    </Suspense>
  );
}