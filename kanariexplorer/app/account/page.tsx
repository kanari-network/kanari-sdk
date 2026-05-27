"use client";

import { useState, useEffect, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { getAccount, getAllBalances, getAllTransactions, getOwnedNfts, getTransaction } from "../lib/rpc";
import TransactionDetailsModal from "../components/TransactionDetailsModal";

// ฟังก์ชันย่อ Address/Hash
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

  // ระบบ Tab
  const [activeTab, setActiveTab] = useState('portfolio');

  // State สำหรับธุรกรรม
  const [txs, setTxs] = useState<any[]>([]);
  const [txLoading, setTxLoading] = useState(false);

  // State สำหรับ Modal ธุรกรรม
  const [selectedTx, setSelectedTx] = useState<any | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [modalLoading, setModalLoading] = useState(false);

  // State สำหรับ NFT
  const [nfts, setNfts] = useState<any[]>([]);
  const [nftLoading, setNftLoading] = useState(false);

  useEffect(() => {
    const q = searchParams.get("address");
    if (q) {
      setAddress(q);
      fetchAccountData(q);
    }
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

      try {
        const nftData = await getOwnedNfts(target);
        setNfts(Array.isArray(nftData) ? nftData : []);
      } catch (e) { console.error("NFT fetch failed", e); }

    } catch (e: any) {
    } finally {
      setLoading(false); setTxLoading(false); setNftLoading(false);
    }
  }

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
    <div className="max-w-7xl mx-auto px-6 py-12 w-full relative">

      {/* 🔍 Search Input */}
      <div className="flex bg-[#111113]/80 backdrop-blur-md border border-white/10 rounded-xl p-1.5 mb-10 w-full md:w-150 focus-within:border-emerald-500/50 focus-within:ring-2 focus-within:ring-emerald-500/20 transition-all shadow-lg">
        <div className="pl-3 flex items-center justify-center">
          <svg className="w-5 h-5 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
        </div>
        <input
          value={address}
          onChange={(e) => setAddress(e.target.value)}
          placeholder="Enter Address 0x..."
          className="flex-1 bg-transparent text-white px-3 py-2 text-sm focus:outline-none font-mono placeholder:text-zinc-600"
        />
        <button onClick={() => fetchAccountData(address)} className="bg-white hover:bg-zinc-200 text-black px-6 py-2.5 rounded-lg text-sm font-bold transition-colors shadow-sm">
          Search
        </button>
      </div>

      {/* 👤 Account Header */}
      {account && (
        <div className="mb-10 bg-[#111113]/60 backdrop-blur-md border border-white/5 p-8 rounded-[40px] shadow-lg relative overflow-hidden">
          <div className="absolute -top-24 -right-24 w-64 h-64 bg-emerald-500/10 blur-[80px] rounded-full pointer-events-none"></div>
          <div className="flex items-center gap-6 relative z-10">
            <div className="w-20 h-20 rounded-[28px] bg-linear-to-tr from-emerald-400 to-cyan-500 flex items-center justify-center shadow-lg shadow-emerald-500/20">
              <svg className="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path></svg>
            </div>
            <div>
              <h2 className="text-zinc-500 text-[10px] font-black uppercase tracking-[0.2em] mb-2">Account Explorer</h2>
              <div className="text-2xl md:text-3xl font-mono font-bold text-white break-all leading-tight">{account.address}</div>
              <div className="mt-3 inline-flex items-center gap-2 px-3 py-1 bg-white/5 border border-white/10 rounded-full text-[10px] font-mono text-zinc-400">
                <span className="w-1.5 h-1.5 rounded-full bg-cyan-400"></span> Sequence: {account.sequence_number || 0}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 📊 Dashboard Tabs System */}
      <div className="mb-20">
        <div className="flex border-b border-white/10 mb-8 gap-8">
          {[
            { id: 'portfolio', label: 'Coins', count: balances?.length || 0 },
            { id: 'nfts', label: 'NFTs', count: nfts.length },
            { id: 'activity', label: 'Activity', count: txs.length }
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`pb-4 text-sm font-bold transition-all relative ${activeTab === tab.id ? 'text-emerald-400' : 'text-zinc-500 hover:text-white'
                }`}
            >
              {tab.label}
              <span className="ml-2 px-2 py-0.5 rounded-md bg-white/5 text-[10px] font-mono">{tab.count}</span>
              {activeTab === tab.id && (
                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.5)]"></div>
              )}
            </button>
          ))}
        </div>

        {/* Tab Content: Portfolio */}
        {activeTab === 'portfolio' && (
          <div className="bg-[#111113]/60 backdrop-blur-md rounded-4xl border border-white/5 overflow-hidden shadow-xl animate-in fade-in slide-in-from-bottom-2 duration-300">
            {!balances || balances.length === 0 ? (
              <div className="p-20 text-center text-zinc-600 font-mono text-sm">No assets found in this portfolio</div>
            ) : (
              // 🚨 เปลี่ยนตรงนี้: ถอด Grid ออก ใช้เป็น flex-col เรียงลงมาแถวเดียว พร้อมเส้นคั่น divide-y
              <div className="flex flex-col divide-y divide-white/5">
                {balances.map((b, i) => (
                  // 🚨 เอา border-b ออก และเพิ่มคลาส group เพื่อให้เอฟเฟกต์โฮเวอร์ทำงานร่วมกัน
                  <div key={i} className="p-6 flex justify-between items-center hover:bg-white/2 transition-colors group">
                    <div className="flex items-center gap-4">
                      <div className="w-12 h-12 rounded-2xl bg-linear-to-br from-zinc-800 to-zinc-900 border border-white/10 flex items-center justify-center overflow-hidden shadow-inner group-hover:border-emerald-500/30 transition-all duration-300">
                        {b.icon_url ? (
                          <img src={b.icon_url} className="w-full h-full object-cover" />
                        ) : (
                          <span className="text-xs font-bold text-zinc-500">{b.symbol?.charAt(0)}</span>
                        )}
                      </div>
                      <div>
                        <div className="text-white text-sm font-bold group-hover:text-emerald-400 transition-colors">{b.name ?? b.symbol}</div>
                        <div className="text-[10px] text-zinc-500 font-mono uppercase tracking-widest mt-0.5">{b.symbol}</div>
                      </div>
                    </div>
                    <div className="text-right">
                      {/* 🚨 อัปเดตการดึงค่ายอดเงิน ให้รองรับทั้ง amount และ balance */}
                      <div className="text-white font-mono font-bold text-lg group-hover:text-cyan-400 transition-colors">
                        {fmtBalance(b.amount ?? b.balance, b.decimals)}
                      </div>
                      <div className="text-[9px] text-zinc-600 font-bold uppercase tracking-tighter mt-1">Current Balance</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Tab Content: NFT Gallery (Compact Grid) */}
        {activeTab === 'nfts' && (
          <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
            {nftLoading ? (
              <div className="py-32 text-center text-zinc-500 font-mono text-sm flex flex-col items-center gap-4">
                <div className="w-8 h-8 border-4 border-emerald-500/20 border-t-emerald-500 rounded-full animate-spin"></div>
                Syncing assets...
              </div>
            ) : nfts.length === 0 ? (
              <div className="p-24 bg-[#111113]/60 rounded-[40px] border border-dashed border-white/10 text-center text-zinc-600 font-mono text-sm">
                No NFTs owned by this address
              </div>
            ) : (
              <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-6">
                {nfts.map((nft, i) => (
                  <div key={i} className="bg-[#111113]/60 border border-white/5 rounded-3xl overflow-hidden hover:border-emerald-500/50 transition-all group cursor-pointer shadow-xl flex flex-col">
                    <div className="aspect-square bg-white/2 flex items-center justify-center relative overflow-hidden p-2">
                      {nft.parsed?.image_url ? (
                        <img src={nft.parsed.image_url} className="w-full h-full object-cover rounded-2xl group-hover:scale-110 transition-transform duration-700" />
                      ) : (
                        <span className="text-xl text-zinc-800 font-mono font-black opacity-40">#{nft.object_id.slice(-4)}</span>
                      )}
                      <div className="absolute top-4 right-4 bg-emerald-500/10 text-emerald-400 text-[8px] font-black px-2 py-1 rounded-lg border border-emerald-500/20 backdrop-blur-md">NFT</div>
                    </div>
                    <div className="p-5 bg-black/40 border-t border-white/5">
                      <div className="text-[10px] text-zinc-300 font-mono truncate mb-1">{shortenHash(nft.object_id)}</div>
                      <div className="text-[9px] text-zinc-600 font-black uppercase tracking-widest">{nft.type.split('::').pop()}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Tab Content: Activity (Transactions) */}
        {activeTab === 'activity' && (
          <div className="bg-[#111113]/60 backdrop-blur-md rounded-4xl border border-white/5 overflow-hidden shadow-xl animate-in fade-in slide-in-from-bottom-2 duration-300">
            {txLoading && (
              <div className="p-24 text-center text-zinc-500 font-mono text-sm flex flex-col items-center gap-4">
                <div className="w-8 h-8 border-4 border-emerald-500/20 border-t-emerald-500 rounded-full animate-spin"></div>
                Loading transactions...
              </div>
            )}
            {!txLoading && txs.length === 0 && <div className="p-24 text-center text-zinc-600 font-mono text-sm">No transactions found.</div>}

            <div className="overflow-x-auto">
              {txs.length > 0 && (
                <table className="w-full text-left border-collapse whitespace-nowrap">
                  <thead>
                    <tr className="bg-white/3 border-b border-white/5 text-[10px] font-black text-zinc-500 tracking-[0.2em] uppercase">
                      <th className="p-6 pl-8">Txn Hash</th>
                      <th className="p-6">Method</th>
                      <th className="p-6">Target</th>
                      <th className="p-6 pr-8 text-right">Status</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-white/5 text-sm font-mono">
                    {txs.map((tx, i) => (
                      <tr key={i} className="hover:bg-white/2 transition-colors group">
                        <td className="p-6 pl-8">
                          <button
                            onClick={() => handleViewTxDetails(tx.hash)}
                            className="text-cyan-400 hover:text-cyan-300 transition-colors font-medium flex items-center gap-3"
                          >
                            <svg className="w-4 h-4 opacity-50 group-hover:opacity-100 transition-opacity" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path></svg>
                            {shortenHash(tx.hash)}
                          </button>
                        </td>
                        <td className="p-6">
                          <span className="bg-zinc-900/80 border border-white/5 text-zinc-400 px-3 py-1.5 rounded-lg text-[9px] font-black uppercase tracking-widest">
                            {tx.tx_type.replace('_', ' ')}
                          </span>
                        </td>
                        <td className="p-6 text-zinc-500">
                          {tx.module ? (
                            <div className="inline-flex bg-white/5 border border-white/5 px-2.5 py-1 rounded-lg text-[11px]">
                              <span className="text-zinc-300">{tx.module}</span>
                              {tx.function && <span className="text-zinc-600 ml-1">::{tx.function}</span>}
                            </div>
                          ) : '-'}
                        </td>
                        <td className="p-6 pr-8 text-right">
                          <div className="flex flex-col items-end">
                            <span className={`flex items-center gap-2 text-[10px] font-black uppercase tracking-widest ${tx.status === 'pending' ? 'text-amber-400' : 'text-emerald-400'}`}>
                              {tx.status === 'pending' && <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse"></span>}
                              {tx.status}
                            </span>
                            <span className="text-[10px] text-zinc-700 mt-1">{tx.block_height ? `Block ${tx.block_height}` : 'Mempool'}</span>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        )}
      </div>

      {/* 🛠 Developer Raw Data (Compact Toggle) */}
      <details className="group mb-20 border-t border-white/5 pt-10">
        <summary className="list-none cursor-pointer flex items-center gap-3 text-zinc-600 hover:text-zinc-400 transition-colors">
          <div className="w-6 h-6 rounded-lg bg-white/5 flex items-center justify-center group-open:rotate-90 transition-transform">
            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M9 5l7 7-7 7" /></svg>
          </div>
          <span className="text-[10px] font-black uppercase tracking-[0.3em]">Developer: Raw Account State</span>
        </summary>
        <div className="mt-6 bg-[#09090b]/80 backdrop-blur-md rounded-3xl border border-white/5 p-8 max-h-100 overflow-auto custom-scrollbar shadow-inner">
          {!account ? (
            <div className="text-zinc-700 font-mono text-xs italic">Waiting for data...</div>
          ) : (
            <pre className="text-[11px] text-emerald-500/60 font-mono leading-relaxed">
              {JSON.stringify(account, null, 2)}
            </pre>
          )}
        </div>
      </details>

      {/* 🚨 Modal Details (คงเดิมจากโค้ดที่คุณส่งมา) */}
      <TransactionDetailsModal
        open={isModalOpen}
        loading={modalLoading}
        transaction={selectedTx}
        onClose={() => setIsModalOpen(false)}
      />

      {false && isModalOpen && (
        <div className="fixed inset-0 z-100 flex items-center justify-center bg-black/80 backdrop-blur-xl p-4 transition-all animate-in fade-in duration-300">
          <div className="bg-[#111113] border border-white/10 rounded-[40px] w-full max-w-3xl shadow-2xl flex flex-col max-h-[90vh] overflow-hidden">
            <div className="flex justify-between items-center p-8 border-b border-white/5">
              <h3 className="text-xl font-black text-white tracking-tight flex items-center gap-4">
                <div className="w-10 h-10 rounded-2xl bg-emerald-500/10 flex items-center justify-center border border-emerald-500/20">
                  <svg className="w-5 h-5 text-emerald-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
                </div>
                Transaction Details
              </h3>
              <button onClick={() => setIsModalOpen(false)} className="w-10 h-10 flex items-center justify-center rounded-full bg-white/5 hover:bg-emerald-500 text-white transition-all">
                <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
              </button>
            </div>
            <div className="p-10 overflow-y-auto custom-scrollbar">
              {modalLoading ? (
                <div className="text-center text-zinc-600 font-mono py-20 animate-pulse">Syncing transaction data...</div>
              ) : selectedTx ? (
                <div className="space-y-8">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div className="bg-white/2 p-6 rounded-3xl border border-white/5">
                      <div className="text-[10px] text-zinc-500 uppercase font-black tracking-widest mb-3">Txn Hash</div>
                      <div className="text-xs font-mono text-emerald-400 break-all">{selectedTx.hash}</div>
                    </div>
                    <div className="bg-white/2 p-6 rounded-3xl border border-white/5">
                      <div className="text-[10px] text-zinc-500 uppercase font-black tracking-widest mb-3">Execution Status</div>
                      <div className={`text-xs font-black uppercase flex items-center gap-2 ${selectedTx.status === 'pending' ? 'text-amber-400' : 'text-emerald-400'}`}>
                        {selectedTx.status}
                      </div>
                    </div>
                  </div>
                  {/* ... ข้อมูลอื่นๆ แสดงตาม JSON ... */}
                  <div className="bg-[#09090b] p-8 rounded-3xl border border-white/5 overflow-x-auto shadow-inner">
                    <pre className="text-[11px] text-emerald-500/60 font-mono">{JSON.stringify(selectedTx, null, 2)}</pre>
                  </div>
                </div>
              ) : null}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default function AccountPage() {
  return (
    <Suspense fallback={null}>
      <AccountContent />
    </Suspense>
  );
}
