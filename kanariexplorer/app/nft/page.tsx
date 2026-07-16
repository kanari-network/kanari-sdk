"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { asArray, EmptyState, PageHeader, RawDetails, readString, shortHash, StatusPill } from "../components/ExplorerUI";
import { NftArtwork } from "../components/NftArtwork";
import { getCollections } from "../lib/rpc";

export default function NftPage() {
  const [collections, setCollections] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadCollections() {
      setLoading(true);
      try {
        setCollections(asArray(await getCollections()));
      } catch {
        setCollections([]);
      } finally {
        setLoading(false);
      }
    }

    loadCollections();
  }, []);

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="NFT Marketplace"
        title="Collection"
        accent="Explorer."
        description="Browse Kanari NFT collections and open a collection to inspect the objects inside."
      />

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">Collections</h2>
            <p className="panel-subtitle">Showing {collections.length} collections</p>
          </div>
          <StatusPill label={loading ? "Syncing" : "Ready"} state={loading ? "warn" : "ok"} />
        </div>
        {loading ? <EmptyState loading label="Loading collections..." /> : null}
        {!loading && collections.length === 0 ? <EmptyState label="No collections found." /> : null}
      </section>

      {collections.length > 0 ? (
        <div className="nft-grid">
          {collections.map((collection, index) => {
            const id = readString(collection, "id", readString(collection, "collection_id", readString(collection, "object_id", String(index))));
            return (
              <Link className="nft-card" href={`/nft/collection/${encodeURIComponent(id)}`} key={id}>
                <NftArtwork
                  item={collection}
                  fallback={readString(collection, "symbol", "NFT").slice(0, 2)}
                  alt={readString(collection, "name", "NFT collection artwork")}
                />
                <div className="nft-copy">
                  <strong className="primary-text">{readString(collection, "name", "Unnamed Collection")}</strong>
                  <p className="muted-text">{readString(collection, "description", shortHash(id))}</p>
                </div>
              </Link>
            );
          })}
        </div>
      ) : null}

      <RawDetails label="Developer: collections JSON" value={collections} />
    </div>
  );
}
