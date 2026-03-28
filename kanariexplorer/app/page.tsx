"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { callRpc } from "./lib/rpc";

export default function Home() {
  const router = useRouter();
  const [searchQuery, setSearchQuery] = useState("");

  const [latestHeight, setLatestHeight] = useState<number | null>(null);
  const [blocks, setBlocks] = useState<any[]>([]);
  const [txs, setTxs] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [stats, setStats] = useState<any>(null);
  const [health, setHealth] = useState<any>(null);

  useEffect(() => {
    fetchHome();
  }, []);

  async function fetchHome() {
    try {
      setLoading(true);
      setErr(null);
      const hResp = await callRpc("kanari_getBlockHeight", []);
      const h = hResp?.result ?? hResp;
      if (typeof h === "number") {
        setLatestHeight(h);

        const fetchCount = 4;
        const calls = [];
        for (let i = 0; i < fetchCount; i++) calls.push(callRpc("kanari_getBlock", [h - i]));
        const settled = await Promise.allSettled(calls);
        const fetched: any[] = settled
          .filter((s) => s.status === "fulfilled")
          .map((s: any) => (s.status === "fulfilled" ? s.value?.result ?? s.value : null))
          .filter(Boolean);
        setBlocks(fetched);

        const txList: any[] = [];
        for (const b of fetched) {
          if (!b) continue;
          if (Array.isArray(b.transactions)) txList.push(...b.transactions);
          else if (Array.isArray(b.txs)) txList.push(...b.txs);
          else if (Array.isArray(b.transactions ?? b.result?.transactions)) txList.push(...(b.transactions ?? b.result?.transactions));
        }
        setTxs(txList.slice(0, 10));

        try {
          const s = await callRpc("kanari_getStats", []);
          setStats(s?.result ?? s);
        } catch (e) { }
        try {
          const he = await callRpc("kanari_health", []);
          setHealth(he?.result ?? he);
        } catch (e) { }
      }
    } catch (e: any) {
      setErr(e.message || String(e));
    } finally {
      setLoading(false);
    }
  }

  // 🚨 ระบบค้นหาอัจฉริยะ (Smart Search)
  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    const q = searchQuery.trim();
    if (!q) return;

    if (q.includes("::")) {
      // ถ้ามี :: ให้มองว่าเป็น Token Type แล้วไปหน้า Coins
      router.push(`/coins?token=${encodeURIComponent(q)}`);
    } else if (/^\d+$/.test(q)) {
      // ถ้าเป็นตัวเลขล้วน ให้แจ้งเตือนก่อน (เพราะเรายังไม่มีหน้า Block Details)
      alert(`ระบบค้นหา Block ${q} กำลังอยู่ระหว่างการพัฒนา`);
    } else {
      // ค่าเริ่มต้นให้มองว่าเป็น Address กระเป๋า หรือ Tx Hash
      router.push(`/account?address=${encodeURIComponent(q)}`);
    }
  };

  function short(x: string | number | undefined) {
    if (!x) return "--";
    const s = String(x);
    if (s.length <= 12) return s;
    return s.slice(0, 8) + "..." + s.slice(-4);
  }

  function fmtNum(n: number | string | undefined) {
    if (n === null || n === undefined) return "-";
    const num = typeof n === "string" ? Number(n) : n;
    if (Number.isNaN(num)) return String(n);
    return new Intl.NumberFormat().format(num);
  }

  function fmtSupplyWhole(n: number | string | undefined, decimals = 9) {
    if (n === null || n === undefined) return "-";
    try {
      const num = typeof n === "string" ? BigInt(n) : BigInt(n);
      const pow = BigInt(10) ** BigInt(decimals);
      const integer = num / pow;
      return String(integer).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
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
              <Link href="/account" className="text-zinc-300 hover:text-white">Accounts</Link>
            </nav>
          </div>
          <div className="text-sm text-zinc-300">Testnet</div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 py-10">
        <div className="bg-black rounded-lg p-10 text-white mb-8" style={{ backgroundImage: 'radial-gradient(circle at 10% 10%, rgba(96,165,250,0.06), transparent), linear-gradient(90deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01))' }}>
          <h2 className="text-2xl font-semibold mb-4">Kanari Testnet Explorer</h2>
          <div className="max-w-3xl">
            {/* 🚨 เปลี่ยน Input เดิมให้เป็น Form ค้นหาที่กด Enter ได้ */}
            <form onSubmit={handleSearch} className="relative">
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search by Address / Block / Token (e.g. 0x...)"
                className="w-full rounded-full border border-zinc-700 bg-transparent py-3 px-5 text-white placeholder-zinc-400 focus:outline-none focus:border-blue-500 transition-colors"
              />
              <button type="submit" className="absolute right-2 top-2 bottom-2 bg-blue-600 hover:bg-blue-700 text-white px-6 rounded-full transition-colors">
                Search
              </button>
            </form>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <div className="col-span-1 md:col-span-2 grid grid-cols-1 md:grid-cols-2 md:grid-rows-3 gap-6">
            <div className="bg-white rounded-lg p-6 shadow">
              <div className="text-sm text-zinc-500">Height</div>
              <div className="text-2xl text-zinc-500 font-bold mt-2">{fmtNum(stats?.height ?? latestHeight)}</div>
            </div>
            <div className="bg-white rounded-lg p-6 shadow">
              <div className="text-sm text-zinc-500">Pending Tx</div>
              <div className="text-2xl text-zinc-500 font-bold mt-2">{fmtNum(stats?.pending_transactions)}</div>
            </div>
            <div className="bg-white rounded-lg p-6 shadow">
              <div className="text-sm text-zinc-500">Total Accounts</div>
              <div className="text-2xl text-zinc-500 font-bold mt-2">{fmtNum(stats?.total_accounts)}</div>
            </div>
            <div className="bg-white rounded-lg p-6 shadow">
              <div className="text-sm text-zinc-500">Total Blocks</div>
              <div className="text-2xl text-zinc-500 font-bold mt-2">{fmtNum(stats?.total_blocks)}</div>
            </div>
            <div className="bg-white rounded-lg p-6 shadow">
              <div className="text-sm text-zinc-500">Total Tx</div>
              <div className="text-2xl text-zinc-500 font-bold mt-2">{fmtNum(stats?.total_transactions)}</div>
            </div>
            <div className="bg-white rounded-lg p-6 shadow">
              <div className="text-sm text-zinc-500">Total Supply</div>
              <div className="text-2xl text-zinc-500 font-bold mt-2">{fmtSupplyWhole(stats?.total_supply)} kanari</div>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="bg-white rounded-lg p-6 shadow">
            <h3 className="text-lg font-semibold mb-4">Latest Blocks</h3>
            <ul className="space-y-4">
              {blocks.length === 0 && <li className="text-sm text-zinc-500">No blocks loaded</li>}
              {blocks.map((b, idx) => {
                const h = b?.height ?? b?.header?.height ?? b?.block?.height ?? (latestHeight ? latestHeight - idx : "-");
                const hash = b?.hash ?? b?.id ?? b?.header?.hash ?? b?.block?.hash;
                const txCount = Array.isArray(b?.transactions) ? b.transactions.length : (Array.isArray(b?.txs) ? b.txs.length : "-");
                const proposer = b?.proposer ?? b?.header?.proposer ?? b?.block?.proposer ?? null;
                return (
                  <li key={idx} className="flex items-center justify-between">
                    <div>
                      <a className="text-blue-600 font-medium">{h}</a>
                      <div className="text-sm text-zinc-500">Validator <span className="text-zinc-700">{short(proposer ?? hash)}</span></div>
                    </div>
                    <div className="text-sm text-zinc-400">{txCount} txns</div>
                  </li>
                );
              })}
            </ul>
          </div>

          <div className="bg-white rounded-lg p-6 shadow">
            <h3 className="text-lg font-semibold mb-4">Latest Transactions</h3>
            <ul className="space-y-4">
              {txs.length === 0 && <li className="text-sm text-zinc-500">No recent transactions</li>}
              {txs.map((t, i) => {
                const hash = t?.hash ?? t?.tx_hash ?? t?.id ?? short(JSON.stringify(t).slice(0, 24));
                const from = t?.from ?? t?.sender ?? t?.payload?.from ?? "-";
                const to = t?.to ?? t?.recipient ?? t?.payload?.to ?? "-";
                const value = t?.value ?? t?.amount ?? "-";
                return (
                  <li key={i} className="flex items-start justify-between">
                    <div className="max-w-xs">
                      <a className="text-blue-600 font-medium">{short(hash)}</a>
                      <div className="text-sm text-zinc-500">From {short(from)} To {short(to)}</div>
                    </div>
                    <div className="text-sm text-zinc-400">{String(value)}</div>
                  </li>
                );
              })}
            </ul>
          </div>
        </div>
      </main>
    </div>
  );
}