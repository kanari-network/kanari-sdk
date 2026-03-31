"use client";
import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { getNftsByCollection } from "../../../lib/rpc";

export default function CollectionDetailPage() {
    const { id } = useParams();
    const [nfts, setNfts] = useState<any[]>([]);

    useEffect(() => {
        if (id) getNftsByCollection(id as string).then(setNfts);
    }, [id]);

    return (
        <div className="max-w-6xl mx-auto p-10">
            <h1 className="text-2xl font-bold text-white mb-2">Collection Assets</h1>
            <p className="text-zinc-500 font-mono text-xs mb-8">ID: {id}</p>

            <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
                {nfts.map((nft) => (
                    <div key={nft.object_id} className="bg-[#111] border border-zinc-800 rounded-xl overflow-hidden shadow-lg">
                        <div className="aspect-square bg-zinc-900 flex items-center justify-center text-zinc-700">
                            {/* Image Placeholder */}
                            KariKid #{nft.object_id.slice(-4)}
                        </div>
                        <div className="p-4">
                            <div className="text-white text-sm font-bold truncate">Object ID</div>
                            <div className="text-zinc-500 text-[10px] font-mono truncate">{nft.object_id}</div>

                            {/* รายละเอียดเพิ่มเติมจาก struct KariKid  */}
                            <button className="w-full mt-3 bg-zinc-800 hover:bg-zinc-700 text-white text-[10px] py-1.5 rounded transition-colors">
                                View Details
                            </button>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}