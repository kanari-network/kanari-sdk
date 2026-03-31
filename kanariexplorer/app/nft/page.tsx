"use client";
import { useEffect, useState } from "react";
import { getCollections } from "../lib/rpc";
import Link from "next/link";

export default function CollectionsPage() {
    const [collections, setCollections] = useState<any[]>([]);

    useEffect(() => {
        getCollections().then(setCollections);
    }, []);

    return (
        <div className="max-w-6xl mx-auto p-10">
            <h1 className="text-3xl font-bold text-white mb-8">NFT Collections</h1>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                {collections.map((coll) => (
                    <Link key={coll.id} href={`/nft/collection/${coll.id}`}>
                        <div className="bg-[#111] border border-zinc-800 p-6 rounded-xl hover:border-zinc-500 transition-all cursor-pointer">
                            <div className="w-16 h-16 bg-zinc-800 rounded-lg mb-4 flex items-center justify-center text-2xl">📦</div>
                            <h2 className="text-xl font-bold text-white">Collection</h2>
                            <p className="text-zinc-500 text-sm mt-2 font-mono truncate">{coll.id}</p>
                            <div className="mt-4 text-emerald-400 text-xs font-bold uppercase">View Assets →</div>
                        </div>
                    </Link>
                ))}
            </div>
        </div>
    );
}