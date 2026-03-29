"use client";

import { useEffect, useState, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { getAllTransactions, getTransaction } from "../lib/rpc";
import Link from "next/link";

function shortenHash(hash: string) {
    if (!hash) return "";
    if (hash.length <= 14) return hash;
    return `${hash.slice(0, 10)}...${hash.slice(-8)}`;
}

function TxContent() {
    const [search, setSearch] = useState("");
    const [txs, setTxs] = useState<any[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        fetchTransactions();
        const interval = setInterval(fetchTransactions, 10000);
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

    const handleSearch = (e: React.FormEvent) => {
        e.preventDefault();
        fetchTransactions(search);
    };

    return (
        <div className="max-w-7xl mx-auto px-6 py-10 w-full">
            <div className="flex flex-col md:flex-row md:items-end justify-between mb-6 gap-4">
                <div>
                    <h1 className="text-2xl font-bold text-white tracking-tight">Transactions</h1>
                </div>

                <form onSubmit={handleSearch} className="w-full md:w-100">
                    <div className="flex bg-[#111] border border-zinc-800 rounded-md focus-within:border-zinc-500 transition-colors p-1">
                        <input
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            placeholder="Filter by Hash / Address"
                            className="w-full bg-transparent text-white px-3 py-2 text-sm focus:outline-none font-mono placeholder:text-zinc-600"
                        />
                        <button type="submit" className="bg-zinc-800 text-zinc-300 hover:text-white px-4 rounded text-sm font-medium transition-colors">
                            Filter
                        </button>
                    </div>
                </form>
            </div>

            <div className="bg-[#111] rounded-lg border border-zinc-800 overflow-hidden">
                {loading && txs.length === 0 && <div className="p-10 text-center text-zinc-600 font-mono text-sm">Loading data...</div>}
                {!loading && txs.length === 0 && <div className="p-10 text-center text-zinc-600 font-mono text-sm">No transactions found.</div>}

                <div className="overflow-x-auto">
                    {txs.length > 0 && (
                        <table className="w-full text-left border-collapse whitespace-nowrap">
                            <thead>
                                <tr className="bg-[#161616] border-b border-zinc-800 text-xs font-medium text-zinc-500">
                                    <th className="p-4 font-normal">TXN HASH</th>
                                    <th className="p-4 font-normal">TYPE</th>
                                    <th className="p-4 font-normal">SENDER</th>
                                    <th className="p-4 font-normal">TARGET</th>
                                    <th className="p-4 text-right font-normal">STATUS</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-zinc-800/50 text-sm font-mono">
                                {txs.map((tx, i) => (
                                    <tr key={i} className="hover:bg-[#1a1a1a] transition-colors">
                                        <td className="p-4">
                                            <span className="text-blue-400 cursor-pointer hover:text-blue-300">{shortenHash(tx.hash)}</span>
                                        </td>
                                        <td className="p-4">
                                            <span className="bg-zinc-800 text-zinc-300 px-2 py-1 rounded text-xs uppercase tracking-wider">
                                                {tx.tx_type.replace('_', ' ')}
                                            </span>
                                        </td>
                                        <td className="p-4">
                                            <Link href={`/account?address=${tx.sender}`} className="text-blue-400 hover:text-blue-300">
                                                {shortenHash(tx.sender)}
                                            </Link>
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
    );
}

export default function TxPage() {
    return (
        <Suspense fallback={<div className="p-20 text-center font-mono text-zinc-600">Loading...</div>}>
            <TxContent />
        </Suspense>
    );
}