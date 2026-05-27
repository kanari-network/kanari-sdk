"use client";

import { useEffect, useState, Suspense } from "react";
import { getAllTransactions, getTransaction } from "../lib/rpc";
import Link from "next/link";
import TransactionDetailsModal from "../components/TransactionDetailsModal";

// ฟังก์ชันย่อ Hash สำหรับแสดงผล
function shortenHash(hash: string) {
    if (!hash) return "";
    if (hash.length <= 14) return hash;
    return `${hash.slice(0, 10)}...${hash.slice(-8)}`;
}

function TxContent() {
    const [search, setSearch] = useState("");
    const [txs, setTxs] = useState<any[]>([]);
    const [loading, setLoading] = useState(true);

    // State สำหรับจัดการ Modal รายละเอียดธุรกรรม
    const [selectedTx, setSelectedTx] = useState<any | null>(null);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [modalLoading, setModalLoading] = useState(false);

    useEffect(() => {
        fetchTransactions();
        const interval = setInterval(fetchTransactions, 10000); // Auto-refresh ทุก 10 วินาที
        return () => clearInterval(interval);
    }, []);

    async function fetchTransactions(query?: string) {
        try {
            setLoading(true);
            const q = query ?? search;
            if (q && q.length > 40) {
                const res = await getTransaction(q);
                setTxs(res?.result ? [res.result] : (res ? [res] : []));
            } else {
                const res = await getAllTransactions(50, q || undefined);
                setTxs(Array.isArray(res?.result) ? res.result : (Array.isArray(res) ? res : []));
            }
        } catch (e) {
            setTxs([]);
        } finally {
            setLoading(false);
        }
    }

    // ฟังก์ชันดึงข้อมูลรายละเอียดเมื่อคลิกที่ Hash
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

    const handleSearch = (e: React.FormEvent) => {
        e.preventDefault();
        fetchTransactions(search);
    };

    return (
        <div className="max-w-7xl mx-auto px-6 py-12 w-full relative">

            {/* 👤 Header Card Style - Emerald & Cyan Theme */}
            <div className="mb-12 bg-[#111113]/60 backdrop-blur-md border border-white/5 p-8 rounded-[40px] shadow-lg relative overflow-hidden">
                {/* 🚨 เปลี่ยน Glow เป็นสี Emerald */}
                <div className="absolute -top-24 -right-24 w-64 h-64 bg-emerald-500/10 blur-[80px] rounded-full pointer-events-none"></div>
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-8 relative z-10">
                    <div className="flex items-center gap-6">
                        {/* 🚨 เปลี่ยน Gradient เป็นสีเขียว-ฟ้า แบบหน้า Account */}
                        <div className="w-20 h-20 rounded-[28px] bg-linear-to-tr from-emerald-400 to-cyan-500 flex items-center justify-center shadow-lg shadow-emerald-500/20">
                            <svg className="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                            </svg>
                        </div>
                        <div>
                            <h2 className="text-zinc-500 text-[10px] font-black uppercase tracking-[0.2em] mb-2">Network Activity</h2>
                            <h1 className="text-3xl md:text-4xl font-black text-white tracking-tighter">Transactions</h1>
                            <div className="mt-3 inline-flex items-center gap-2 px-3 py-1 bg-white/5 border border-white/10 rounded-full text-[10px] font-mono text-zinc-400">
                                {/* 🚨 เปลี่ยนจุดสถานะเป็นสี Cyan */}
                                <span className="w-1.5 h-1.5 rounded-full bg-cyan-400"></span>
                                {loading ? "Syncing blocks..." : `Monitoring ${txs.length} Latest Operations`}
                            </div>
                        </div>
                    </div>

                    {/* Search Input - ปรับ Focus ให้เป็นสี Emerald */}
                    <form onSubmit={handleSearch} className="w-full md:w-112.5">
                        <div className="flex bg-black/40 backdrop-blur-md border border-white/10 rounded-2xl p-1.5 focus-within:border-emerald-500/50 transition-all shadow-inner">
                            <div className="pl-3 flex items-center justify-center">
                                <svg className="w-4 h-4 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                            </div>
                            <input
                                value={search}
                                onChange={(e) => setSearch(e.target.value)}
                                placeholder="Filter by Hash or Address..."
                                className="w-full bg-transparent text-white px-3 py-2 text-xs focus:outline-none font-mono placeholder:text-zinc-700"
                            />
                            <button type="submit" className="bg-white hover:bg-zinc-200 text-black px-5 py-2 rounded-xl text-[11px] font-black uppercase transition-all shadow-sm">
                                Filter
                            </button>
                        </div>
                    </form>
                </div>
            </div>

            {/* 📊 Transactions Table */}
            <div className="bg-[#111113]/60 backdrop-blur-md rounded-4xl border border-white/5 overflow-hidden shadow-2xl">
                {loading && txs.length === 0 && (
                    <div className="p-24 text-center text-zinc-500 font-mono text-sm flex flex-col items-center gap-4">
                        {/* 🚨 ปรับ Spinner เป็นสี Emerald */}
                        <div className="w-8 h-8 border-4 border-emerald-500/20 border-t-emerald-500 rounded-full animate-spin"></div>
                        Fetching network activity...
                    </div>
                )}
                {!loading && txs.length === 0 && (
                    <div className="p-24 text-center text-zinc-600 font-mono text-sm">
                        No transactions found matching your criteria.
                    </div>
                )}

                <div className="overflow-x-auto">
                    {txs.length > 0 && (
                        <table className="w-full text-left border-collapse whitespace-nowrap">
                            <thead>
                                <tr className="bg-white/3 border-b border-white/5 text-[10px] font-black text-zinc-500 tracking-[0.2em] uppercase">
                                    <th className="p-6 pl-8">Txn Hash</th>
                                    <th className="p-6">Type</th>
                                    <th className="p-6">Sender</th>
                                    <th className="p-6">Target</th>
                                    <th className="p-6 pr-8 text-right">Status</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-white/5 text-sm font-mono">
                                {txs.map((tx, i) => (
                                    <tr key={tx.hash ?? i} className="hover:bg-white/2 transition-colors group">
                                        <td className="p-6 pl-8">
                                            {/* 🚨 ปุ่มกดรายละเอียดใช้สี Cyan-400 แบบหน้า Account */}
                                            <button
                                                onClick={() => handleViewTxDetails(tx.hash)}
                                                className="text-cyan-400 hover:text-cyan-300 transition-colors font-medium flex items-center gap-3 group/hash"
                                            >
                                                <svg className="w-4 h-4 opacity-50 group-hover/hash:opacity-100 transition-opacity" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path>
                                                </svg>
                                                {shortenHash(tx.hash)}
                                            </button>
                                        </td>
                                        <td className="p-6">
                                            <span className="bg-zinc-900/80 border border-white/5 text-zinc-400 px-3 py-1.5 rounded-lg text-[9px] font-black uppercase tracking-widest">
                                                {tx.tx_type.replace('_', ' ')}
                                            </span>
                                        </td>
                                        <td className="p-6">
                                            {/* 🚨 Sender link ใช้สี Emerald-400 แบบหน้า Account */}
                                            <Link href={`/account?address=${tx.sender}`} className="text-emerald-400 hover:text-emerald-300 flex items-center gap-2 group/addr">
                                                <div className="w-6 h-6 rounded-lg border border-white/10 bg-zinc-900 flex items-center justify-center group-hover/addr:border-emerald-500/30 transition-all">
                                                    <svg className="w-3 h-3 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path></svg>
                                                </div>
                                                {shortenHash(tx.sender)}
                                            </Link>
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
                                                    {tx.status === 'pending' && <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse shadow-[0_0_8px_rgba(251,191,36,0.5)]"></span>}
                                                    {tx.status !== 'pending' && <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="3" d="M5 13l4 4L19 7"></path></svg>}
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

            {/* 🚨 Modal Details - Emerald & Cyan Edition */}
            {false && isModalOpen && (
                <div className="fixed inset-0 z-100 flex items-center justify-center bg-black/80 backdrop-blur-xl p-4 animate-in fade-in duration-300">
                    <div className="bg-[#111113] border border-white/10 rounded-[40px] w-full max-w-3xl shadow-[0_0_100px_rgba(16,185,129,0.1)] flex flex-col max-h-[90vh] overflow-hidden">
                        <div className="flex justify-between items-center p-8 border-b border-white/5">
                            <h3 className="text-xl font-black text-white flex items-center gap-4">
                                {/* 🚨 ไอคอนสี Emerald แบบหน้า Account */}
                                <div className="w-10 h-10 rounded-2xl bg-emerald-500/10 flex items-center justify-center border border-emerald-500/20">
                                    <svg className="w-5 h-5 text-emerald-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
                                </div>
                                Transaction Details
                            </h3>
                            <button onClick={() => setIsModalOpen(false)} className="w-10 h-10 flex items-center justify-center rounded-full bg-white/5 hover:bg-emerald-500 text-white transition-all border border-white/10">
                                <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
                            </button>
                        </div>
                        <div className="p-10 overflow-y-auto custom-scrollbar">
                            {modalLoading ? (
                                <div className="text-center text-zinc-600 font-mono py-20 animate-pulse">Syncing transaction data...</div>
                            ) : selectedTx ? (
                                <div className="space-y-8">
                                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                        <div className="bg-white/5 p-6 rounded-3xl border border-white/10">
                                            <div className="text-[10px] text-zinc-500 uppercase font-black tracking-widest mb-3">Txn Hash</div>
                                            <div className="text-xs font-mono text-cyan-400 break-all">{selectedTx.hash}</div>
                                        </div>
                                        <div className="bg-white/5 p-6 rounded-3xl border border-white/10">
                                            <div className="text-[10px] text-zinc-500 uppercase font-black tracking-widest mb-3">Execution Status</div>
                                            <div className={`text-xs font-black uppercase flex items-center gap-2 ${selectedTx.status === 'pending' ? 'text-amber-400' : 'text-emerald-400'}`}>
                                                {selectedTx.status}
                                            </div>
                                        </div>
                                    </div>
                                    <div className="bg-[#09090b] p-8 rounded-3xl border border-white/5 overflow-x-auto shadow-inner">
                                        {/* 🚨 ตัวหนังสือ JSON ใช้สี Emerald-500/60 */}
                                        <pre className="text-[11px] text-emerald-500/60 font-mono leading-relaxed">
                                            {JSON.stringify(selectedTx, null, 2)}
                                        </pre>
                                    </div>
                                </div>
                            ) : (
                                <div className="text-center text-zinc-600 font-mono py-20">No data found.</div>
                            )}
                        </div>
                    </div>
                </div>
            )}

            {/* 🛠 Developer Info - Emerald Theme */}
            <TransactionDetailsModal
                open={isModalOpen}
                loading={modalLoading}
                transaction={selectedTx}
                onClose={() => setIsModalOpen(false)}
            />

            <details className="group mt-12 border-t border-white/5 pt-10">
                <summary className="list-none cursor-pointer flex items-center gap-3 text-zinc-600 hover:text-emerald-400 transition-colors">
                    <div className="w-6 h-6 rounded-lg bg-white/5 flex items-center justify-center group-open:rotate-90 transition-transform">
                        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M9 5l7 7-7 7" /></svg>
                    </div>
                    <span className="text-[10px] font-black uppercase tracking-[0.3em]">Developer: Latest Tx Raw JSON</span>
                </summary>
                <div className="mt-6 bg-[#09090b]/80 backdrop-blur-md rounded-3xl border border-white/5 p-8 max-h-100 overflow-auto custom-scrollbar shadow-inner">
                    <pre className="text-[11px] text-emerald-500/60 font-mono leading-relaxed">
                        {JSON.stringify(txs, null, 2)}
                    </pre>
                </div>
            </details>
        </div>
    );
}

export default function TxPage() {
    return (
        <Suspense fallback={<div className="p-32 text-center font-mono text-zinc-600 animate-pulse">Loading Explorer...</div>}>
            <TxContent />
        </Suspense>
    );
}
