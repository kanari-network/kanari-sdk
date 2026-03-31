"use client";
import { useEffect, useState, Suspense } from "react";
import { useParams } from "next/navigation";
import { getNftsByCollection, getCollections } from "../../../lib/rpc";
import Link from "next/link";

function shortenHash(hash: string) {
    if (!hash) return "";
    if (hash.length <= 14) return hash;
    return `${hash.slice(0, 10)}...${hash.slice(-8)}`;
}

function CollectionDetailContent() {
    const { id } = useParams();
    const [nfts, setNfts] = useState<any[]>([]);
    const [collection, setCollection] = useState<any>(null);
    const [loading, setLoading] = useState(true);
    const [selectedNft, setSelectedNft] = useState<any | null>(null);

    useEffect(() => {
        if (id) {
            Promise.all([getNftsByCollection(id as string), getCollections()]).then(([nftData, allCollections]) => {
                setNfts(nftData);
                setCollection(allCollections.find((c: any) => c.id === id));
                setLoading(false);
            }).catch(() => setLoading(false));
        }
    }, [id]);

    return (
        <div className="max-w-7xl mx-auto px-6 py-12 w-full relative">
            {/* 👤 Hero Section แบบหน้า Account Header */}
            <div className="mb-12 bg-[#111113]/60 backdrop-blur-md border border-white/5 p-8 rounded-[40px] shadow-lg relative overflow-hidden group">
                <div className="absolute inset-0 bg-black opacity-40 group-hover:opacity-20 transition-opacity">
                    <img src={collection?.banner_url || "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe"} className="w-full h-full object-cover blur-sm group-hover:blur-0 transition-all duration-1000" />
                </div>
                <div className="absolute inset-0 bg-linear-to-t from-[#111113] via-[#111113]/60 to-transparent"></div>

                <div className="flex flex-col md:flex-row items-center gap-8 relative z-10">
                    <div className="w-32 h-32 rounded-4xl bg-linear-to-tr from-emerald-500 to-cyan-500 p-1 shadow-2xl">
                        <div className="w-full h-full bg-[#111113] rounded-[28px] flex items-center justify-center overflow-hidden">
                            {collection?.banner_url ? <img src={collection.banner_url} className="w-full h-full object-cover" /> : <span className="text-5xl">🖼️</span>}
                        </div>
                    </div>
                    <div className="text-center md:text-left">
                        <h2 className="text-zinc-400 text-[10px] font-black uppercase tracking-[0.3em] mb-2">Collection Detail</h2>
                        <h1 className="text-4xl md:text-5xl font-black text-white tracking-tighter mb-4">{collection?.name || "KariKid Gallery"}</h1>
                        <div className="flex flex-wrap justify-center md:justify-start gap-4">
                            <div className="px-4 py-2 bg-white/5 backdrop-blur-md border border-white/10 rounded-2xl text-[10px] font-mono text-zinc-300">
                                <span className="text-emerald-500 font-bold mr-2 uppercase">Contract</span> {shortenHash(id as string)}
                            </div>
                            {collection?.website_url && (
                                <a href={collection.website_url} target="_blank" className="bg-white/5 px-4 py-2 rounded-2xl border border-white/10 text-[10px] font-black text-zinc-300 hover:text-white transition-all flex items-center gap-2">
                                    PROJECT WEBSITE ↗
                                </a>
                            )}
                        </div>
                    </div>

                    {/* 📊 Stats Compact แบบหน้า Portfolio */}
                    <div className="ml-auto grid grid-cols-2 gap-4">
                        <div className="bg-white/5 px-6 py-4 rounded-[28px] border border-white/10 text-center shadow-inner">
                            <div className="text-[9px] text-zinc-500 font-bold uppercase mb-1 tracking-widest">Items</div>
                            <div className="text-xl font-mono font-black text-white">{nfts.length}</div>
                        </div>
                        <div className="bg-white/5 px-6 py-4 rounded-[28px] border border-white/10 text-center shadow-inner">
                            <div className="text-[9px] text-zinc-500 font-bold uppercase mb-1 tracking-widest">Supply</div>
                            <div className="text-xl font-mono font-black text-white">{collection?.max_supply || '∞'}</div>
                        </div>
                    </div>
                </div>
            </div>

            {/* 🖼️ NFT Grid Style แบบหน้า Account Gallery */}
            <div className="mb-8 flex justify-between items-center">
                <h3 className="text-xl font-black text-white uppercase tracking-widest flex items-center gap-3">
                    <div className="w-2 h-6 bg-emerald-500 rounded-full"></div>
                    Gallery Assets
                </h3>
                <Link href="/nft" className="text-xs font-black text-zinc-500 hover:text-emerald-400 transition-colors">
                    BACK TO MARKETPLACE
                </Link>
            </div>

            {loading ? (
                <div className="py-32 text-center text-zinc-600 font-mono text-sm animate-pulse flex flex-col items-center gap-4">
                    <div className="w-8 h-8 border-4 border-emerald-500/20 border-t-emerald-500 rounded-full animate-spin"></div>
                    Syncing collection metadata...
                </div>
            ) : (
                <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-6">
                    {nfts.map((nft, i) => (
                        <div
                            key={i}
                            onClick={() => setSelectedNft(nft)}
                            className="bg-[#111113]/60 border border-white/5 rounded-4xl overflow-hidden hover:border-emerald-500/50 transition-all group cursor-pointer shadow-xl flex flex-col"
                        >
                            {/* Preview Area */}
                            <div className="aspect-square bg-white/2 flex items-center justify-center relative overflow-hidden p-3">
                                {nft.parsed?.image_url ? (
                                    <img
                                        src={nft.parsed.image_url}
                                        className="w-full h-full object-cover rounded-3xl group-hover:scale-110 transition-transform duration-700"
                                        alt="NFT Preview"
                                    />
                                ) : (
                                    <span className="text-xl text-zinc-800 font-black opacity-40">#{nft.object_id.slice(-4)}</span>
                                )}

                                <div className="absolute top-4 right-4 bg-emerald-500/10 text-emerald-400 text-[8px] font-black px-2 py-1 rounded-lg border border-emerald-500/20 backdrop-blur-md">
                                    NFT
                                </div>
                            </div>

                            {/* Info Area */}
                            <div className="p-5 bg-black/40 border-t border-white/5">
                                <div className="text-[10px] text-zinc-300 font-mono truncate mb-1">
                                    {shortenHash(nft.object_id)}
                                </div>
                                <div className="text-[9px] text-zinc-600 font-black uppercase tracking-widest">
                                    {nft.type.split('::').pop()}
                                </div>

                                <div className="mt-4 text-white text-[11px] font-bold group-hover:text-emerald-400 transition-colors flex items-center justify-between">
                                    VIEW DETAIL
                                    <span className="opacity-0 group-hover:opacity-100 transition-all -translate-x-2.5 group-hover:translate-x-0">
                                        →
                                    </span>
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            )}


            {/* Modal Detail (Refined) */}

            {selectedNft && (
                <div className="fixed inset-0 z-100 flex items-center justify-center bg-black/90 backdrop-blur-3xl p-4 md:p-10 animate-in fade-in duration-300">
                    <div className="bg-[#09090b] border border-white/10 rounded-[56px] w-full max-w-6xl shadow-[0_0_100px_rgba(16,185,129,0.1)] flex flex-col md:flex-row max-h-[90vh] overflow-hidden relative">

                        {/* ปุ่มปิด - เปลี่ยน hover เป็นสี Emerald */}
                        <button onClick={() => setSelectedNft(null)} className="absolute top-8 right-8 z-50 w-12 h-12 flex items-center justify-center rounded-full bg-white/5 hover:bg-emerald-500 text-white transition-all border border-white/10">
                            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M6 18L18 6M6 6l12 12" /></svg>
                        </button>

                        {/* ส่วนแสดงรูปภาพ */}
                        <div className="w-full md:w-[45%] bg-[#111113] flex items-center justify-center p-8 border-b md:border-b-0 md:border-r border-white/5">
                            <div className="w-full aspect-square rounded-[40px] overflow-hidden shadow-2xl border border-white/5">
                                {selectedNft.parsed?.image_url ? (
                                    <img src={selectedNft.parsed.image_url} alt="NFT" className="w-full h-full object-cover" />
                                ) : (
                                    <div className="w-full h-full bg-zinc-900 flex items-center justify-center text-8xl">🎨</div>
                                )}
                            </div>
                        </div>

                        {/* ส่วนข้อมูลรายละเอียด */}
                        <div className="w-full md:w-[55%] p-12 overflow-y-auto custom-scrollbar flex flex-col">
                            <div className="mb-12">
                                {/* Badge - เปลี่ยนเป็นสี Emerald */}
                                <div className="inline-block px-4 py-1.5 bg-emerald-500/10 border border-emerald-500/20 rounded-full text-emerald-400 text-[10px] font-black tracking-widest uppercase mb-4">
                                    {selectedNft.type.split("::").pop()}
                                </div>
                                <h2 className="text-5xl font-black text-white mb-6 leading-none tracking-tighter">
                                    {selectedNft.parsed?.name || `NFT #${selectedNft.object_id.slice(-4)}`}
                                </h2>
                                <p className="text-zinc-400 text-lg leading-relaxed font-medium">
                                    {selectedNft.parsed?.description || "This digital asset is a part of the official collection verified on Kanari Network."}
                                </p>
                            </div>

                            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-10">
                                <div className="bg-white/5 p-6 rounded-[28px] border border-white/10">
                                    <div className="text-[10px] text-zinc-500 uppercase font-black tracking-widest mb-2">Object ID</div>
                                    <div className="text-xs font-mono text-zinc-300 break-all">{selectedNft.object_id}</div>
                                </div>
                                <div className="bg-white/5 p-6 rounded-[28px] border border-white/10">
                                    <div className="text-[10px] text-zinc-500 uppercase font-black tracking-widest mb-2">Metadata Status</div>
                                    <div className="text-xs font-mono text-emerald-400 flex items-center gap-2 font-bold">
                                        <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse shadow-[0_0_8px_rgba(52,211,153,0.5)]"></div> Verified Standard
                                    </div>
                                </div>
                            </div>

                            {/* Properties Section - เปลี่ยนสีแท่งข้างหน้าเป็น Emerald */}
                            {selectedNft.parsed?.attributes?.keys && (
                                <div className="mb-10">
                                    <h3 className="text-sm font-bold text-white mb-6 flex items-center gap-3">
                                        <div className="w-2 h-8 bg-emerald-500 rounded-full shadow-[0_0_10px_rgba(52,211,153,0.3)]"></div>
                                        PROPERTIES
                                    </h3>
                                    <div className="grid grid-cols-2 sm:grid-cols-3 gap-4">
                                        {selectedNft.parsed.attributes.keys.map((key: string, i: number) => (
                                            <div key={i} className="bg-white/3 border border-white/5 rounded-3xl p-5 hover:border-emerald-500/30 transition-all group/prop">
                                                <div className="text-[9px] text-zinc-500 uppercase font-black mb-1 group-hover/prop:text-emerald-500/70 transition-colors">{key}</div>
                                                <div className="text-md font-bold text-white">
                                                    {selectedNft.parsed.attributes.values[i] || "-"}
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

export default function CollectionDetailPage() {
    return (
        <Suspense fallback={null}>
            <CollectionDetailContent />
        </Suspense>
    );
}