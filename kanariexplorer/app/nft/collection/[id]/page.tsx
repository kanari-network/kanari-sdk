"use client";

import Link from "next/link";
import { use, useEffect, useState } from "react";
import { asArray, EmptyState, PageHeader, RawDetails, readString, shortHash, StatusPill } from "../../../components/ExplorerUI";
import { getNftsByCollection } from "../../../lib/rpc";

export default function CollectionPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const [items, setItems] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadItems() {
      setLoading(true);
      try {
        setItems(asArray(await getNftsByCollection(decodeURIComponent(id))));
      } catch {
        setItems([]);
      } finally {
        setLoading(false);
      }
    }

    loadItems();
  }, [id]);

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="Collection Detail"
        title="NFT"
        accent="Objects."
        description={`Inspect NFT objects inside collection ${shortHash(decodeURIComponent(id))}.`}
      >
        <Link className="button button--ghost" href="/nft">
          Back to collections
        </Link>
      </PageHeader>

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">Collection Items</h2>
            <p className="panel-subtitle mono">{decodeURIComponent(id)}</p>
          </div>
          <StatusPill label={loading ? "Syncing" : `${items.length} items`} state={loading ? "warn" : "ok"} />
        </div>
        {loading ? <EmptyState loading label="Loading NFT objects..." /> : null}
        {!loading && items.length === 0 ? <EmptyState label="No NFTs found in this collection." /> : null}
      </section>

      {items.length > 0 ? (
        <div className="nft-grid">
          {items.map((item, index) => {
            const objectId = readString(item, "object_id", `nft-${index}`);
            return (
              <article className="nft-card" key={objectId}>
                <div className="nft-art">#{objectId.slice(-4)}</div>
                <div className="nft-copy">
                  <strong className="primary-text mono">{shortHash(objectId)}</strong>
                  <p className="muted-text">{readString(item, "name", readString(item, "type", "NFT Object"))}</p>
                </div>
              </article>
            );
          })}
        </div>
      ) : null}

      <RawDetails label="Developer: collection JSON" value={items} />
    </div>
  );
}
