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
    if (address.length > 10) { const t = setTimeout(() => fetchBalances(address), 500); return () => clearTimeout(t); }
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
    <div className="max-w-7xl mx-auto px-6 py-10 w-full">
      <div className="flex flex-col md:flex-row md:items-end justify-between mb-6 gap-4">
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">Tokens</h1>
        </div>
        <div className="w-full md:w-100">
          <div className="flex bg-[#111] border border-zinc-800 rounded-md focus-within:border-zinc-500 transition-colors p-1">
            <input
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="Filter by Address (0x...)"
              className="w-full bg-transparent text-white px-3 py-2 text-sm focus:outline-none font-mono placeholder:text-zinc-600"
            />
          </div>
        </div>
      </div>

      <div className="bg-[#111] rounded-lg border border-zinc-800 overflow-hidden">
        {loading && <div className="p-10 text-center text-zinc-600 font-mono text-sm">Loading...</div>}
        {!loading && balances.length === 0 && <div className="p-10 text-center text-zinc-600 font-mono text-sm">No assets found.</div>}

        <div className="overflow-x-auto">
          {balances.length > 0 && (
            <table className="w-full text-left border-collapse whitespace-nowrap">
              <thead>
                <tr className="bg-[#161616] border-b border-zinc-800 text-xs font-medium text-zinc-500">
                  <th className="p-4 font-normal">ASSET</th>
                  <th className="p-4 font-normal">TYPE</th>
                  <th className="p-4 text-right font-normal">{address ? "BALANCE" : "TOTAL SUPPLY"}</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-800/50 text-sm">
                {balances.map((b, i) => {
                  const symbol = b.symbol || "UNK";
                  return (
                    <tr key={i} className="hover:bg-[#1a1a1a] transition-colors">
                      <td className="p-4">
                        <div className="flex items-center gap-3">
                          {b.icon_url ? (
                            <img src={b.icon_url} alt="icon" className="w-8 h-8 rounded-full border border-zinc-700 bg-black" />
                          ) : (
                            <div className="w-8 h-8 rounded-full bg-zinc-800 text-zinc-400 flex items-center justify-center font-bold text-xs">
                              {symbol.charAt(0)}
                            </div>
                          )}
                          <div>
                            <div className="text-white font-medium">{b.name ?? symbol}</div>
                            <div className="text-xs text-zinc-500">{symbol}</div>
                          </div>
                        </div>
                      </td>
                      <td className="p-4 font-mono text-zinc-400 text-xs">
                        {b.token_type ?? b.token}
                      </td>
                      <td className="p-4 text-right font-mono text-white">
                        {fmtBalance(b.balance ?? b.total_supply, b.decimals)}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}

export default function CoinsPage() {
  return (
    <Suspense fallback={<div className="p-20 text-center font-mono text-zinc-600">Loading...</div>}>
      <CoinsContent />
    </Suspense>
  );
}