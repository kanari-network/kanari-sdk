"use client";

import Link from "next/link";
import { useEffect, useState, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { getAllBalances, getTokenBalance, getBalance, getTokens } from "../lib/rpc";

function CoinsContent() {
  const searchParams = useSearchParams();
  const [address, setAddress] = useState<string>("");
  const [balances, setBalances] = useState<any[]>([]);
  const [globalTokens, setGlobalTokens] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const [tokenType, setTokenType] = useState<string>("");
  const [tokenBalance, setTokenBalance] = useState<any | null>(null);
  const [tokenLoading, setTokenLoading] = useState(false);

  useEffect(() => {
    loadGlobalTokens();
    const queryToken = searchParams.get("token");
    if (queryToken) {
      setTokenType(queryToken);
    }
  }, [searchParams]);

  useEffect(() => {
    if (address && address.length > 10) {
      const timer = setTimeout(() => {
        fetchBalances(address);
      }, 500);
      return () => clearTimeout(timer);
    } else if (!address) {
      // เมื่อไม่มีการค้นหากระเป๋า ให้โชว์เหรียญจาก API ส่วนกลาง (Global Tokens)
      setBalances(globalTokens);
    }
  }, [address, globalTokens]);

  // 🚨 ดึงรายชื่อเหรียญทั้งหมดจาก API (Backend) ล้วนๆ ไม่มี LocalStorage
  async function loadGlobalTokens() {
    try {
      const t = await getTokens();
      let arr: any[] = [];
      if (Array.isArray(t)) arr = t as any[];
      else if (t && Array.isArray(t.result)) arr = t.result;
      else if (t && Array.isArray(t.result?.result)) arr = t.result.result;

      let normalized = arr.map((x: any) => ({ ...(x || {}), decimals: x?.decimals ?? 9 }));

      setGlobalTokens(normalized);
      if (!address) {
        setBalances(normalized);
      }
    } catch (e) {
      console.error("Failed to fetch tokens from API", e);
    }
  }

  // 🚨 ดึงเหรียญที่มีอยู่ในกระเป๋าจาก API
  async function fetchBalances(addr?: string) {
    try {
      setLoading(true);
      setErr(null);
      const target = addr ?? address;
      if (!target) {
        setBalances(globalTokens);
        return;
      }

      const data = await getAllBalances(target);
      const payload = data?.result ?? data;
      let arr: any[] = [];
      if (Array.isArray(payload)) arr = payload;
      else if (payload && typeof payload === "object") {
        if (Array.isArray(payload.balances)) arr = payload.balances;
        else if (payload.balances && typeof payload.balances === "object") {
          arr = Object.entries(payload.balances).map(([k, v]) => (v && typeof v === "object" ? { ...v, token_type: k } : { token_type: k, balance: v }));
        }
      }

      setBalances(arr);
    } catch (e: any) {
      setErr(e?.message || String(e));
      setBalances([]);
    } finally {
      setLoading(false);
    }
  }

  async function fetchTokenBalance() {
    try {
      setTokenLoading(true);
      setTokenBalance(null);
      if (!address || !tokenType) return;

      const data = await getTokenBalance(address, tokenType);
      const result = data?.result ?? data;
      setTokenBalance(result);
    } catch (e: any) {
      setTokenBalance(null);
      setErr(e?.message || String(e));
    } finally {
      setTokenLoading(false);
    }
  }

  async function fetchNativeBalance() {
    try {
      setLoading(true);
      if (!address) return;
      const data = await getBalance(address);
      setBalances([{ token_type: "KANARI", balance: data?.result ?? data, decimals: 9 }]);
    } catch (e: any) {
      setErr(e?.message || String(e));
    } finally {
      setLoading(false);
    }
  }

  function fmtBalance(n: any, decimals = 9) {
    if (n === null || n === undefined) return "-";
    try {
      const s = typeof n === "bigint" ? n.toString() : String(n);
      const num = BigInt(s);
      const pow = BigInt(10) ** BigInt(decimals);
      const whole = num / pow;
      const frac = num % pow;
      const wholeStr = String(whole).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
      if (frac === BigInt(0)) return wholeStr;
      let fracStr = String(frac).padStart(decimals, "0");
      fracStr = fracStr.replace(/0+$/, "");
      return `${wholeStr}.${fracStr}`;
    } catch (e) {
      return String(n);
    }
  }

  return (
    <main className="max-w-7xl mx-auto px-6 py-10">
      <div className="bg-white rounded-lg p-6 shadow">
        <div className="flex flex-col md:flex-row md:items-center justify-between mb-6 gap-4">
          <h2 className="text-xl font-semibold">Coins / Balances</h2>
          <div className="flex items-center space-x-2">
            <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder="Enter address (0x...)" className="rounded border border-zinc-200 px-3 py-2 text-sm focus:outline-none focus:border-blue-500 w-64" />
            <button onClick={() => fetchBalances()} className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded transition-colors text-sm">Scan Wallet</button>
          </div>
        </div>

        <div className="flex items-center space-x-3 mb-6 bg-zinc-50 p-4 rounded-lg border">
          <div className="flex-1">
            <label className="block text-xs text-zinc-500 mb-1">Target Token Type (Optional)</label>
            <input value={tokenType} onChange={(e) => setTokenType(e.target.value)} placeholder="e.g. 0x1::james::JAMES" className="w-full rounded border border-zinc-200 px-3 py-2 text-sm focus:outline-none focus:border-blue-500" />
          </div>
          <button onClick={() => fetchTokenBalance()} className="bg-zinc-800 hover:bg-black text-white px-4 py-2 rounded transition-colors text-sm mt-5">Query Token</button>
        </div>

        {err && <div className="mb-4 text-red-600 bg-red-50 p-3 rounded text-sm border border-red-100">Error: {err}</div>}

        <div>
          {loading && <div className="text-sm text-zinc-500 mb-4">Scanning API for tokens...</div>}
          {!loading && balances.length === 0 && <div className="text-sm text-zinc-500 mb-4">No balances available</div>}
          <ul className="divide-y border-t mt-2">
            {balances.map((b, i) => {
              const tokenTypeStr = b.token_type ?? b.tokenType ?? b.token ?? "Unknown";
              const symbol = b.symbol ?? (typeof tokenTypeStr === "string" ? tokenTypeStr.split("::").pop() : "KANARI");

              const rawBalance = b.balance ?? b.amount ?? b.value ?? b.balance_raw ?? b.total_supply ?? null;
              const dec = b.decimals ?? 9;

              return (
                <li key={i} className="py-4 flex items-center justify-between hover:bg-zinc-50 px-2 rounded transition-colors">
                  <div>
                    <div className="text-xs text-zinc-400 mb-1 break-all">{tokenTypeStr}</div>
                    <div className="font-medium text-blue-600">{symbol}</div>
                  </div>
                  <div className="text-sm font-semibold text-zinc-800 bg-zinc-100 px-3 py-1 rounded-full">
                    {rawBalance !== null ? fmtBalance(rawBalance, dec) : "-"}
                  </div>
                </li>
              );
            })}
          </ul>
        </div>

        {tokenLoading && <div className="mt-6 text-sm text-zinc-500">Querying specific token...</div>}
        {tokenBalance && (
          <div className="mt-6 bg-zinc-900 text-green-400 p-4 rounded-lg border border-zinc-800 shadow-inner">
            <div className="text-xs text-zinc-400 mb-2 uppercase tracking-wider">Token Query Result</div>
            <pre className="text-sm overflow-auto">{JSON.stringify(tokenBalance, null, 2)}</pre>
          </div>
        )}
      </div>
    </main>
  );
}

export default function CoinsPage() {
  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-black text-white">
        <div className="max-w-7xl mx-auto px-6 py-6 flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <div className="text-xl font-bold text-blue-600">KANARI Explorer</div>
            <nav className="hidden md:flex space-x-4">
              <Link href="/" className="text-zinc-300 hover:text-white transition-colors">Home</Link>
              <Link href="/coins" className="text-white font-medium">Coins</Link>
              <Link href="/account" className="text-zinc-300 hover:text-white transition-colors">Accounts</Link>
            </nav>
          </div>
        </div>
      </header>
      <Suspense fallback={<div className="p-10 text-center">Loading Explorer...</div>}>
        <CoinsContent />
      </Suspense>
    </div>
  );
}