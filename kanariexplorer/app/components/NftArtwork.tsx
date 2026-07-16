"use client";

import { useEffect, useMemo, useState } from "react";
import { asRecord, readString } from "./ExplorerUI";

const IMAGE_KEYS = [
  "image_url",
  "imageUrl",
  "image",
  "image_uri",
  "imageUri",
  "thumbnail_url",
  "thumbnailUrl",
  "thumbnail",
  "cover_url",
  "coverUrl",
];

const NESTED_METADATA_KEYS = ["metadata", "display", "content", "fields", "data"];

function normalizeImageUrl(value: string): string | null {
  const url = value.trim();
  if (!url) return null;
  if (url.startsWith("ipfs://")) return `https://ipfs.io/ipfs/${url.slice("ipfs://".length)}`;
  if (url.startsWith("data:image/") && !url.startsWith("data:image/svg")) return url;

  try {
    const parsed = new URL(url);
    return parsed.protocol === "https:" || parsed.protocol === "http:" ? parsed.toString() : null;
  } catch {
    return null;
  }
}

/** Finds common NFT image fields, including Move/RPC metadata envelopes. */
export function nftImageUrl(value: unknown, depth = 0): string | null {
  if (depth > 3) return null;
  const record = asRecord(value);

  for (const key of IMAGE_KEYS) {
    const imageUrl = normalizeImageUrl(readString(record, key));
    if (imageUrl) return imageUrl;
  }

  for (const key of NESTED_METADATA_KEYS) {
    const candidate = record[key];
    if (candidate && typeof candidate === "object") {
      const imageUrl = nftImageUrl(candidate, depth + 1);
      if (imageUrl) return imageUrl;
    }
  }

  return null;
}

export function NftArtwork({ item, fallback, alt }: { item: unknown; fallback: string; alt: string }) {
  const source = useMemo(() => nftImageUrl(item), [item]);
  const [failed, setFailed] = useState(false);

  useEffect(() => setFailed(false), [source]);

  return (
    <div className="nft-art">
      {source && !failed ? (
        <img src={source} alt={alt} loading="lazy" referrerPolicy="no-referrer" onError={() => setFailed(true)} />
      ) : fallback}
    </div>
  );
}
