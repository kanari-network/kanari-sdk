"use client";

import Link from "next/link";
import { use, useEffect, useState } from "react";
import {
  asRecord,
  EmptyState,
  PageHeader,
  RawDetails,
  readString,
  shortHash,
  StatusPill,
} from "../../../components/ExplorerUI";
import { NftArtwork, nftMetadata } from "../../../components/NftArtwork";
import { getObject } from "../../../lib/rpc";

export default function NftItemPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const objectId = decodeURIComponent(id);
  const [item, setItem] = useState<unknown>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;

    async function loadItem() {
      setLoading(true);
      setError("");
      try {
        const result = await getObject(objectId);
        if (active) setItem(result);
      } catch (loadError) {
        if (active) {
          setItem(null);
          setError(
            loadError instanceof Error ? loadError.message : String(loadError),
          );
        }
      } finally {
        if (active) setLoading(false);
      }
    }

    loadItem();
    return () => {
      active = false;
    };
  }, [objectId]);

  const metadata = nftMetadata(item);
  const object = asRecord(item);

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="NFT Details"
        title={metadata?.name || "NFT"}
        accent="Object."
        description={
          metadata?.description || `Inspect NFT object ${shortHash(objectId)}.`
        }
      >
        <Link className="button button--ghost" href="/nft">
          Back to collections
        </Link>
      </PageHeader>

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">NFT Details</h2>
            <p className="panel-subtitle mono break-anywhere">{objectId}</p>
          </div>
          <StatusPill
            label={loading ? "Loading" : error ? "Unavailable" : "Loaded"}
            state={loading ? "warn" : error ? "down" : "ok"}
          />
        </div>

        {loading ? <EmptyState loading label="Loading NFT details..." /> : null}
        {!loading && error ? (
          <EmptyState label={`Could not load NFT: ${error}`} />
        ) : null}
        {!loading && !error && !item ? (
          <EmptyState label="NFT object not found." />
        ) : null}
        {!loading && !error && item ? (
          <div className="nft-detail-layout">
            <NftArtwork
              item={item}
              fallback={`#${objectId.slice(-4)}`}
              alt={metadata?.name || "NFT artwork"}
            />
            <div className="nft-detail-content">
              <div className="object-detail-grid">
                <div className="object-detail-field object-detail-field--wide">
                  <p className="tiny-label">Name</p>
                  <span>{metadata?.name || "Unknown NFT"}</span>
                </div>
                <div className="object-detail-field object-detail-field--wide">
                  <p className="tiny-label">Description</p>
                  <span>{metadata?.description || "No description"}</span>
                </div>
                <div className="object-detail-field">
                  <p className="tiny-label">Object ID</p>
                  <span className="mono break-anywhere">
                    {readString(object, "id", objectId)}
                  </span>
                </div>
                <div className="object-detail-field">
                  <p className="tiny-label">Collection</p>
                  <span className="mono break-anywhere">
                    {metadata?.collectionId || "Unavailable"}
                  </span>
                </div>
                <div className="object-detail-field">
                  <p className="tiny-label">Creator</p>
                  <span className="mono break-anywhere">
                    {metadata?.creator || "Unavailable"}
                  </span>
                </div>
                {metadata?.number ? (
                  <div className="object-detail-field">
                    <p className="tiny-label">Number</p>
                    <span>{metadata.number}</span>
                  </div>
                ) : null}
                <div className="object-detail-field object-detail-field--wide">
                  <p className="tiny-label">Type</p>
                  <span className="mono break-anywhere">
                    {readString(
                      object,
                      "type_",
                      readString(object, "type", "Unknown"),
                    )}
                  </span>
                </div>
              </div>

              <section className="nft-attributes">
                <h3 className="panel-title">Attributes</h3>
                {metadata?.attributes.length ? (
                  <div className="data-list">
                    {metadata.attributes.map(({ key, value }, index) => (
                      <div className="data-row" key={`${key}-${index}`}>
                        <div>
                          <p className="tiny-label">{key}</p>
                          <span className="mono break-anywhere">{value}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="muted-text">This NFT has no attributes.</p>
                )}
              </section>
            </div>
          </div>
        ) : null}
      </section>

      {item ? (
        <RawDetails label="Developer: NFT object JSON" value={item} />
      ) : null}
    </div>
  );
}
