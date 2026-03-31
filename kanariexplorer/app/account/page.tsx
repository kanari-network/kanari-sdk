"use client";

import { useState, useEffect, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { getAccount, getAllBalances, getAllTransactions, getOwnedNfts, getTransaction } from "../lib/rpc";
import Link from "next/link";

function shortenHash(hash: string) {
  if (!hash) return "";
  if (hash.length <= 14) return hash;
  return `${hash.slice(0, 10)}...${hash.slice(-8)}`;
}

function AccountContent() {
  const searchParams = useSearchParams();
  const [address, setAddress] = useState("");
  const [loading, setLoading] = useState(false);
  const [account, setAccount] = useState<any | null>(null);
  const [balances, setBalances] = useState<any[] | null>(null);

  // State สำหรับตารางธุรกรรม (getAllTransactions)
  const [txs, setTxs] = useState<any[]>([]);
  const [txLoading, setTxLoading] = useState(false);

  // 🚨 State สำหรับ Popup รายละเอียดธุรกรรม (getTransaction)
  const [selectedTx, setSelectedTx] = useState<any | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [modalLoading, setModalLoading] = useState(false);

  // 🚨 เพิ่ม State สำหรับ NFT
  const [nfts, setNfts] = useState<any[]>([]);
  const [nftLoading, setNftLoading] = useState(false);

  useEffect(() => {
    const q = searchParams.get("address");
    if (q) { setAddress(q); fetchAccountData(q); }
  }, [searchParams]);

  async function fetchAccountData(target: string) {
    if (!target) return;
    try {
      setLoading(true); setTxLoading(true); setNftLoading(true);
      const a = await getAccount(target);
      setAccount(a);

      try { const b = await getAllBalances(target); setBalances(b ?? null); } catch (e) { }
      try {
        const t = await getAllTransactions(50, target);
        setTxs(Array.isArray(t?.result) ? t.result : (Array.isArray(t) ? t : []));
      } catch (e) { setTxs([]); }

      // 🚨 เรียกข้อมูล NFT
      try {
        const nftData = await getOwnedNfts(target);
        setNfts(Array.isArray(nftData) ? nftData : []);
      } catch (e) { console.error("NFT fetch failed", e); }

    } catch (e: any) { } finally {
      setLoading(false); setTxLoading(false); setNftLoading(false);
    }
  }

  // 🚨 2. ฟังก์ชันเรียกใช้ handle_get_transaction เมื่อคลิกที่ Hash
  async function handleViewTxDetails(hash: string) {
    setIsModalOpen(true);
    setModalLoading(true);
    setSelectedTx(null);
    try {
      const res = await getTransaction(hash);
      setSelectedTx(res?.result ?? res);
    } catch (e) {
      console.error("Failed to fetch tx details", e);
    } finally {
      setModalLoading(false);
    }
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
    <div className="max-w-7xl mx-auto px-6 py-10 w-full relative">
      <div className="flex bg-[#111] border border-zinc-800 rounded-md p-1 mb-8 w-full md:w-150 focus-within:border-zinc-500 transition-colors">
        <input
          value={address}
          onChange={(e) => setAddress(e.target.value)}
          placeholder="Enter Address 0x..."
          className="flex-1 bg-transparent text-white px-4 py-2 text-sm focus:outline-none font-mono placeholder:text-zinc-600"
        />
        <button onClick={() => fetchAccountData(address)} className="bg-zinc-100 hover:bg-white text-black px-6 py-2 rounded text-sm font-bold transition-colors">
          Search
        </button>
      </div>

      {account && (
        <div className="mb-6 border-b border-zinc-800 pb-6">
          <h2 className="text-zinc-500 text-xs font-bold uppercase tracking-widest mb-2">Account Details</h2>
          <div className="text-2xl font-mono text-white break-all">{account.address}</div>
          <div className="mt-2 text-zinc-500 font-mono text-sm">Sequence: {account.sequence_number || 0}</div>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
        <div className="lg:col-span-2">
          <h3 className="text-zinc-400 font-medium mb-4">Coins / Portfolio</h3>
          <div className="bg-[#111] rounded-lg border border-zinc-800 overflow-hidden">
            {!balances ? (
              <div className="p-10 text-center text-zinc-600 font-mono text-sm">No assets to display</div>
            ) : (
              <ul className="divide-y divide-zinc-800/50">
                {balances.map((b, i) => (
                  <li key={i} className="p-4 flex justify-between items-center hover:bg-[#161616] transition-colors">
                    <div className="flex items-center gap-4">
                      {b.icon_url ? (
                        <img src={b.icon_url} alt="icon" className="w-8 h-8 rounded-full bg-black border border-zinc-800" />
                      ) : (
                        <div className="w-8 h-8 rounded-full bg-zinc-800 flex items-center justify-center text-xs font-bold text-zinc-400">
                          {b.symbol?.charAt(0) || "K"}
                        </div>
                      )}
                      <div>
                        <div className="text-white font-medium">{b.name ?? b.symbol}</div>
                        <div className="text-xs text-zinc-500 font-mono">{b.symbol}</div>
                      </div>
                    </div>
                    <div className="text-right font-mono text-white">
                      {fmtBalance(b.balance, b.decimals)}
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        <div className="lg:col-span-1">
          <h3 className="text-zinc-400 font-medium mb-4">Raw State JSON</h3>
          <div className="bg-[#0D1117] rounded-lg border border-zinc-800 p-4 max-h-100 overflow-auto custom-scrollbar">
            {!account ? (
              <div className="text-zinc-600 font-mono text-xs">Waiting...</div>
            ) : (
              <pre className="text-xs text-emerald-400 font-mono">
                {JSON.stringify(account, null, 2)}
              </pre>
            )}
          </div>
        </div>
      </div>

      {/* แสดงผล NFT Gallery */}
      <div className="lg:col-span-1">
        <h3 className="text-zinc-400 font-medium mb-4 flex justify-between">
          NFTs <span>{nfts.length}</span>
        </h3>
        <div className="bg-[#111] rounded-lg border border-zinc-800 p-4 max-h-100 overflow-y-auto custom-scrollbar">
          {nftLoading ? (
            <div className="text-center text-zinc-600 font-mono text-sm py-10">Loading NFTs...</div>
          ) : nfts.length === 0 ? (
            <div className="text-center text-zinc-600 font-mono text-sm py-10">No NFTs found</div>
          ) : (
            <div className="grid grid-cols-2 gap-3">
              {nfts.map((nft, i) => (
                <div key={i} className="bg-zinc-900 rounded-lg border border-zinc-800 overflow-hidden group hover:border-zinc-500 transition-colors">
                  <div className="aspect-square bg-black flex items-center justify-center relative">
                    {/* หากมีระบบดึงรูปภาพ (image_url) สามารถใส่ <img> ตรงนี้ได้ */}
                    <span className="text-[10px] text-zinc-600 font-mono">KariKid #{nft.object_id.slice(-4)}</span>
                    <div className="absolute top-1 right-1 bg-emerald-500/10 text-emerald-500 text-[8px] px-1 rounded border border-emerald-500/20">NFT</div>
                  </div>
                  <div className="p-2">
                    <div className="text-[10px] text-white font-mono truncate">{nft.object_id}</div>
                    <div className="text-[8px] text-zinc-500 uppercase mt-0.5">{nft.type.split('::').pop()}</div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>


      {/* ตารางแสดงธุรกรรม (List) */}
      <div>
        <h3 className="text-zinc-400 font-medium mb-4">Recent Transactions</h3>
        <div className="bg-[#111] rounded-lg border border-zinc-800 overflow-hidden">
          {txLoading && <div className="p-10 text-center text-zinc-600 font-mono text-sm">Loading transactions...</div>}
          {!txLoading && txs.length === 0 && <div className="p-10 text-center text-zinc-600 font-mono text-sm">No transactions found.</div>}

          <div className="overflow-x-auto">
            {txs.length > 0 && (
              <table className="w-full text-left border-collapse whitespace-nowrap">
                <thead>
                  <tr className="bg-[#161616] border-b border-zinc-800 text-xs font-medium text-zinc-500">
                    <th className="p-4 font-normal">TXN HASH</th>
                    <th className="p-4 font-normal">TYPE</th>
                    <th className="p-4 font-normal">TARGET</th>
                    <th className="p-4 text-right font-normal">STATUS</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-800/50 text-sm font-mono">
                  {txs.map((tx, i) => (
                    <tr key={i} className="hover:bg-[#1a1a1a] transition-colors">
                      <td className="p-4">
                        {/* 🚨 ปุ่มคลิกเพื่อเปิดดูรายละเอียดธุรกรรม 🚨 */}
                        <button
                          onClick={() => handleViewTxDetails(tx.hash)}
                          className="text-blue-400 hover:text-blue-300 hover:underline transition-all"
                        >
                          {shortenHash(tx.hash)}
                        </button>
                      </td>
                      <td className="p-4">
                        <span className="bg-zinc-800 text-zinc-300 px-2 py-1 rounded text-xs uppercase tracking-wider">
                          {tx.tx_type.replace('_', ' ')}
                        </span>
                      </td>
                      <td className="p-4 text-zinc-400">
                        {tx.module ? `${tx.module}${tx.function ? `::${tx.function}` : ''}` : '-'}
                      </td>
                      <td className="p-4 text-right">
                        <div className="flex flex-col items-end">
                          <span className={`flex items-center gap-1.5 text-xs uppercase tracking-wider ${tx.status === 'pending' ? 'text-amber-400' : 'text-emerald-400'}`}>
                            {tx.status}
                          </span>
                          <span className="text-xs text-zinc-600 mt-1">{tx.block_height ? `Block ${tx.block_height}` : 'Mempool'}</span>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      </div>

      {/* 🚨 Modal Popup แสดงรายละเอียดธุรกรรม 🚨 */}
      {
        isModalOpen && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4">
            <div className="bg-[#111] border border-zinc-700 rounded-xl w-full max-w-3xl shadow-2xl flex flex-col max-h-[90vh]">
              <div className="flex justify-between items-center p-6 border-b border-zinc-800">
                <h3 className="text-lg font-bold text-white tracking-tight">Transaction Details</h3>
                <button onClick={() => setIsModalOpen(false)} className="text-zinc-500 hover:text-white transition-colors">
                  <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
                </button>
              </div>

              <div className="p-6 overflow-y-auto custom-scrollbar">
                {modalLoading ? (
                  <div className="text-center text-zinc-500 font-mono py-10 animate-pulse">Fetching details from node...</div>
                ) : selectedTx ? (
                  <div className="space-y-6">
                    {/* แผงข้อมูลหลัก */}
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div className="bg-zinc-900/50 p-4 rounded-lg border border-zinc-800">
                        <div className="text-xs text-zinc-500 uppercase font-bold mb-1">Transaction Hash</div>
                        <div className="text-sm font-mono text-blue-400 break-all">{selectedTx.hash}</div>
                      </div>
                      <div className="bg-zinc-900/50 p-4 rounded-lg border border-zinc-800">
                        <div className="text-xs text-zinc-500 uppercase font-bold mb-1">Status</div>
                        <div className={`text-sm font-bold uppercase ${selectedTx.status === 'pending' ? 'text-amber-400' : 'text-emerald-400'}`}>
                          {selectedTx.status}
                        </div>
                      </div>
                      <div className="bg-zinc-900/50 p-4 rounded-lg border border-zinc-800">
                        <div className="text-xs text-zinc-500 uppercase font-bold mb-1">Sender</div>
                        <div className="text-sm font-mono text-white break-all">{selectedTx.sender}</div>
                      </div>
                      <div className="bg-zinc-900/50 p-4 rounded-lg border border-zinc-800">
                        <div className="text-xs text-zinc-500 uppercase font-bold mb-1">Sequence Number</div>
                        <div className="text-sm font-mono text-white">{selectedTx.sequence_number}</div>
                      </div>
                    </div>

                    {/* Gas & Exec Info */}
                    <div className="flex gap-4">
                      <div className="flex-1 bg-zinc-900/50 p-4 rounded-lg border border-zinc-800">
                        <div className="text-xs text-zinc-500 uppercase font-bold mb-1">Type</div>
                        <div className="text-sm font-mono text-white">{selectedTx.tx_type}</div>
                      </div>
                      <div className="flex-1 bg-zinc-900/50 p-4 rounded-lg border border-zinc-800">
                        <div className="text-xs text-zinc-500 uppercase font-bold mb-1">Gas Limit</div>
                        <div className="text-sm font-mono text-white">{selectedTx.gas_limit}</div>
                      </div>
                    </div>

                    {/* Raw JSON ก้อนเต็มเผื่อ Dev อยากดู */}
                    <div>
                      <div className="text-xs text-zinc-500 uppercase font-bold mb-2">Raw Data (JSON)</div>
                      <div className="bg-[#0D1117] p-4 rounded-lg border border-zinc-800 overflow-x-auto">
                        <pre className="text-xs text-emerald-400 font-mono">
                          {JSON.stringify(selectedTx, null, 2)}
                        </pre>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-center text-red-400 font-mono py-10">Transaction not found.</div>
                )}
              </div>
            </div>
          </div>
        )
      }
    </div >
  );
}

export default function AccountPage() {
  return (
    <Suspense fallback={<div className="p-20 text-center font-mono text-zinc-600">Loading...</div>}>
      <AccountContent />
    </Suspense>
  );
}