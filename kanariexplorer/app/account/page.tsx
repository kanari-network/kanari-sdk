"use client";

import { useState, useEffect, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { getAccount, getAllBalances } from "../lib/rpc";

function AccountContent() {
  const searchParams = useSearchParams();
  const [address, setAddress] = useState("");
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [account, setAccount] = useState<any | null>(null);
  const [balances, setBalances] = useState<any[] | null>(null);

  // 🚨 ดึงค่าจาก URL หากถูก Route มาจากหน้า Home
  useEffect(() => {
    const queryAddress = searchParams.get("address");
    if (queryAddress) {
      setAddress(queryAddress);
      fetchAccountData(queryAddress);
    }
  }, [searchParams]);

  async function fetchAccountData(targetAddress: string) {
    setErr(null);
    setAccount(null);
    setBalances(null);
    if (!targetAddress) return setErr("Please enter an address");

    try {
      setLoading(true);
      const a = await getAccount(targetAddress);
      setAccount(a);
      try {
        const b = await getAllBalances(targetAddress);
        setBalances(b ?? null);
      } catch (e) {
        // ignore
      }
    } catch (e: any) {
      setErr(e?.message ?? String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="max-w-4xl mx-auto px-6 py-10">
      <h1 className="text-2xl font-semibold mb-6">Account</h1>

      <div className="bg-white rounded-lg p-6 shadow mb-6">
        <label className="text-sm text-zinc-600">Address / Tx Hash</label>
        <div className="mt-2 flex gap-2">
          <input
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            placeholder="0x..."
            className="flex-1 rounded border px-3 py-2 focus:outline-none focus:border-blue-500"
          />
          <button onClick={() => fetchAccountData(address)} className="bg-blue-600 hover:bg-blue-700 transition-colors text-white px-6 rounded">
            {loading ? "Loading..." : "Fetch"}
          </button>
        </div>
        {err && <div className="text-sm text-red-600 mt-2">{err}</div>}
      </div>

      <div className="grid grid-cols-1 gap-6">
        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-lg font-medium mb-3">Account Info</h2>
          {!account && <div className="text-sm text-zinc-500">No account loaded</div>}
          {account && (
            <div className="text-sm text-zinc-700">
              <pre className="text-xs overflow-auto max-h-72 bg-zinc-50 p-4 rounded border">{JSON.stringify(account, null, 2)}</pre>
            </div>
          )}
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-lg font-medium mb-3">Balances</h2>
          {!balances && <div className="text-sm text-zinc-500">No balances loaded</div>}
          {Array.isArray(balances) && (
            <ul className="space-y-2">
              {balances.map((b, i) => (
                <li key={i} className="flex justify-between border-b pb-2">
                  <div className="text-sm text-blue-600 font-medium">{b.token_type ?? b.token ?? b.symbol ?? "KANARI"}</div>
                  <div className="text-sm font-medium">{String(b.balance ?? b.value ?? b.amount ?? "-")}</div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </main>
  );
}

// ห่อหุ้มด้วย Suspense เพื่อให้รองรับ useSearchParams ตามมาตรฐาน Next.js
export default function AccountPage() {
  return (
    <div className="min-h-screen bg-gray-50">
      <Suspense fallback={<div className="p-10 text-center">Loading Data...</div>}>
        <AccountContent />
      </Suspense>
    </div>
  );
}