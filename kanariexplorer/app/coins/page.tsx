"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { getAllBalances, getTokenBalance, getBalance, getTokens } from "../lib/rpc";

export default function CoinsPage() {
  const [address, setAddress] = useState<string>("");
  const [balances, setBalances] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const [tokenType, setTokenType] = useState<string>("");
  const [tokenBalance, setTokenBalance] = useState<any | null>(null);
  const [tokenLoading, setTokenLoading] = useState(false);

  useEffect(() => {
    // fetch known tokens to show list without searching
    (async () => {
      try {
        const t = await getTokens();
        console.debug("getTokens() ->", t);
        // Normalize response: can be either an array or an RpcResponse with `result`.
        let arr: any[] = [];
        if (Array.isArray(t)) arr = t as any[];
        else if (t && Array.isArray(t.result)) arr = t.result;
        else if (t && Array.isArray(t.result?.result)) arr = t.result.result; // defensive

        if (arr.length > 0) {
          // Ensure decimals present for formatting (default to 9)
          const normalized = arr.map((x: any) => ({ ...(x || {}), decimals: x?.decimals ?? 9 }));
          setBalances(normalized as any[]);
        }
      } catch (e) {
        console.error("Failed to fetch tokens", e);
      }
    })();
  }, []);


  async function fetchBalances(addr?: string) {
    try {
      setLoading(true);
      const target = addr ?? address;
      if (!target) {
        // No address provided — don't treat as an error. Clear balances.
        setBalances([]);
        return;
      }

      const data = await getAllBalances(target);
      // normalize response shape
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
      if (!address) {
        // No address provided for token query — simply return without error
        return;
      }
      if (!tokenType) {
        // No token type entered — no-op
        return;
      }
      const data = await getTokenBalance(address, tokenType);
      setTokenBalance(data?.result ?? data);
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
      if (!address) {
        // No address provided — nothing to fetch
        return;
      }
      const data = await getBalance(address);
      setBalances([{ token_type: "KANARI", balance: data?.result ?? data }]);
    } catch (e: any) {
      setErr(e?.message || String(e));
    } finally {
      setLoading(false);
    }
  }

  function fmtBalance(n: any, decimals = 9) {
    if (n === null || n === undefined) return "-";
    try {
      // support number or numeric string
      const s = typeof n === "bigint" ? n.toString() : String(n);
      const num = BigInt(s);
      const pow = BigInt(10) ** BigInt(decimals);
      const whole = num / pow;
      const frac = num % pow;
      const wholeStr = String(whole).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
      if (frac === BigInt(0)) return wholeStr;
      // pad fractional part with leading zeros
      let fracStr = String(frac).padStart(decimals, "0");
      // trim trailing zeros
      fracStr = fracStr.replace(/0+$/, "");
      return `${wholeStr}.${fracStr}`;
    } catch (e) {
      return String(n);
    }
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-black text-white">
        <div className="max-w-7xl mx-auto px-6 py-6 flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <div className="text-xl font-bold text-blue-600">KANARI Explorer</div>
            <nav className="hidden md:flex space-x-4">
              <Link href="/" className="text-zinc-300 hover:text-white">Home</Link>
              <Link href="/coins" className="text-zinc-300 hover:text-white">Coins</Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 py-10">
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold">Coins / Balances</h2>
            <div className="flex items-center space-x-2">
              <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder="Enter address (e.g. 0x...)" className="rounded border border-zinc-200 px-3 py-1 text-sm" />
              <button onClick={() => fetchBalances()} className="bg-blue-600 text-white px-3 py-1 rounded">Refresh</button>
              <button onClick={() => fetchNativeBalance()} className="px-3 py-1 border rounded">Native</button>
            </div>
          </div>

          <div className="flex items-center space-x-3 mb-4">
            <input value={tokenType} onChange={(e) => setTokenType(e.target.value)} placeholder="Token type (e.g. 0x1::james::JAMES)" className="rounded border border-zinc-200 px-3 py-1 text-sm flex-1" />
            <button onClick={() => fetchTokenBalance()} className="bg-zinc-800 text-white px-3 py-1 rounded">Query Token</button>
          </div>

            {err && <div className="mb-4 text-red-600">Error: {err}</div>}

          <div>
            {loading && <div>Loading balances...</div>}
            {!loading && balances.length === 0 && <div className="text-sm text-zinc-500">No balances available</div>}
            <ul className="divide-y">
              {balances.map((b, i) => {
                // If this is a token metadata entry from getTokens(), it may have
                // `token_type`, `total_supply`, and `symbol` fields. If it's a
                // balance entry it may have `balance` or similar.
                const tokenType = b.token_type ?? b.tokenType ?? b.token;
                const symbol = b.symbol ?? (typeof tokenType === "string" ? tokenType.split("::").pop() : "KANARI");
                const rawBalance = b.total_supply ?? b.balance ?? b.amount ?? b.value ?? b.balance_raw ?? null;
                return (
                  <li key={i} className="py-3 flex items-center justify-between">
                    <div>
                      <div className="text-sm text-zinc-500">{tokenType}</div>
                      <div className="font-medium">{symbol}</div>
                    </div>
                    <div className="text-sm text-zinc-700">{rawBalance === null ? "-" : fmtBalance(rawBalance, b.decimals ?? 9)}</div>
                  </li>
                );
              })}
            </ul>
          </div>

          {tokenLoading && <div className="mt-4">Loading token balance...</div>}
          {tokenBalance && (
            <div className="mt-4 bg-gray-50 p-3 rounded">
              <div className="text-sm text-zinc-500">Token Result</div>
              <pre className="text-sm">{JSON.stringify(tokenBalance, null, 2)}</pre>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
