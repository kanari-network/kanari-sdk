"use client";
import { useEffect, useState, Suspense } from "react";
import { getCollections } from "../lib/rpc";
import Link from "next/link";

function CollectionsContent() {
    const [collections, setCollections] = useState<any[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        getCollections().then((data) => {
            setCollections(data);
            setLoading(false);
        }).catch(() => setLoading(false));
    }, []);

    return (
        <div className="max-w-7xl mx-auto px-6 py-12 w-full relative">
            {/* 👤 Header Style แบบ Account Page */}
            <div className="mb-12 bg-[#111113]/60 backdrop-blur-md border border-white/5 p-8 rounded-[40px] shadow-lg relative overflow-hidden">
                <div className="absolute -top-24 -right-24 w-64 h-64 bg-emerald-500/10 blur-[80px] rounded-full pointer-events-none"></div>
                <div className="flex items-center gap-6 relative z-10">
                    <div className="w-20 h-20 rounded-[28px] bg-linear-to-tr from-emerald-400 to-cyan-500 flex items-center justify-center shadow-lg shadow-emerald-500/20">
                        <svg className="w-10 h-10 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
                        </svg>
                    </div>
                    <div>
                        <h2 className="text-zinc-500 text-[10px] font-black uppercase tracking-[0.2em] mb-2">Marketplace</h2>
                        <h1 className="text-3xl md:text-4xl font-black text-white tracking-tighter">NFT Collections</h1>
                        <div className="mt-3 inline-flex items-center gap-2 px-3 py-1 bg-white/5 border border-white/10 rounded-full text-[10px] font-mono text-zinc-400">
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span> Total: {collections.length} Verified Projects
                        </div>
                    </div>
                </div>
            </div>

            {loading ? (
                <div className="py-40 text-center text-zinc-500 font-mono text-sm flex flex-col items-center gap-4">
                    <div className="w-10 h-10 border-4 border-emerald-500/20 border-t-emerald-500 rounded-full animate-spin"></div>
                    Syncing Marketplace...
                </div>
            ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
                    {collections.map((coll) => (
                        <Link key={coll.id} href={`/nft/collection/${coll.id}`}>
                            <div className="group bg-[#111113]/60 border border-white/5 rounded-[40px] overflow-hidden hover:border-emerald-500/50 transition-all duration-500 shadow-xl relative">
                                <div className="h-32 w-full bg-zinc-900 overflow-hidden relative">
                                    <img src={coll.banner_url || "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe"} className="w-full h-full object-cover group-hover:scale-110 transition-transform duration-1000 opacity-40" alt="banner" />
                                    <div className="absolute inset-0 bg-linear-to-t from-[#111113] to-transparent"></div>
                                </div>

                                <div className="p-8 pt-0 -mt-10 relative z-10">
                                    <div className="w-20 h-20 rounded-[28px] bg-linear-to-br from-emerald-400 to-cyan-500 p-1 mb-6 shadow-2xl group-hover:rotate-6 transition-transform duration-500 overflow-hidden">
                                        <div className="w-full h-full bg-[#111113] rounded-3xl flex items-center justify-center overflow-hidden">
                                            {coll.banner_url ? <img src={coll.banner_url} className="w-full h-full object-cover" /> : <span className="text-4xl">🖼️</span>}
                                        </div>
                                    </div>
                                    <h2 className="text-2xl font-bold text-white mb-2 group-hover:text-emerald-400 transition-colors">{coll.name || "Unnamed"}</h2>
                                    <p className="text-zinc-500 text-sm line-clamp-2 mb-8 font-medium h-10 leading-relaxed">{coll.description || "No description provided."}</p>
                                    <div className="flex justify-between items-center pt-6 border-t border-white/5">
                                        <div>
                                            <div className="text-[10px] text-zinc-600 font-black uppercase tracking-widest">Supply</div>
                                            <div className="text-white font-mono font-bold text-lg">{coll.max_supply || '∞'}</div>
                                        </div>
                                        <div className="w-12 h-12 rounded-2xl bg-white/5 flex items-center justify-center group-hover:bg-emerald-500 text-white transition-all">
                                            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M14 5l7 7m0 0l-7 7m7-7H3" /></svg>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </Link>
                    ))}
                </div>
            )}
        </div>
    );
}

export default function CollectionsPage() {
    return <Suspense fallback={null}><CollectionsContent /></Suspense>;
}