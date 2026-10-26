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
  "banner_url",
  "bannerUrl",
];

const NESTED_METADATA_KEYS = [
  "parsed",
  "metadata",
  "display",
  "content",
  "fields",
  "data",
];

export type NftMetadata = {
  name: string;
  imageUrl: string;
  description: string;
  number: string;
  collectionId: string;
  creator: string;
  attributes: Array<{ key: string; value: string }>;
};

function isByteArray(value: unknown): value is number[] {
  return (
    Array.isArray(value) &&
    value.every(
      (byte) =>
        typeof byte === "number" &&
        Number.isInteger(byte) &&
        byte >= 0 &&
        byte <= 255,
    )
  );
}

type BcsCursor = { offset: number };

function readBcsLength(bytes: Uint8Array, cursor: BcsCursor): number | null {
  let value = 0;
  let shift = 0;

  for (let i = 0; i < 5; i += 1) {
    if (cursor.offset >= bytes.length) return null;
    const byte = bytes[cursor.offset];
    cursor.offset += 1;
    value |= (byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) return value >>> 0;
    shift += 7;
  }

  return null;
}

function readBcsBytes(bytes: Uint8Array, cursor: BcsCursor): Uint8Array | null {
  const length = readBcsLength(bytes, cursor);
  if (length === null || cursor.offset + length > bytes.length) return null;
  const value = bytes.subarray(cursor.offset, cursor.offset + length);
  cursor.offset += length;
  return value;
}

function decodeUtf8(value: Uint8Array): string | null {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(value);
  } catch {
    return null;
  }
}

function readBcsString(bytes: Uint8Array, cursor: BcsCursor): string | null {
  const value = readBcsBytes(bytes, cursor);
  return value ? decodeUtf8(value) : null;
}

function readBcsAddress(bytes: Uint8Array, cursor: BcsCursor): string | null {
  if (cursor.offset + 32 > bytes.length) return null;
  const address = Array.from(bytes.subarray(cursor.offset, cursor.offset + 32))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
  cursor.offset += 32;
  return `0x${address}`;
}

function readBcsU64(bytes: Uint8Array, cursor: BcsCursor): string | null {
  if (cursor.offset + 8 > bytes.length) return null;
  let value = BigInt(0);
  for (let i = 0; i < 8; i += 1) {
    value |= BigInt(bytes[cursor.offset + i]) << BigInt(i * 8);
  }
  cursor.offset += 8;
  return value.toString();
}

function readBcsOptionString(
  bytes: Uint8Array,
  cursor: BcsCursor,
): string | null {
  const variant = readBcsLength(bytes, cursor);
  if (variant === 0) return "";
  if (variant !== 1) return null;
  return readBcsString(bytes, cursor);
}

function isKanariPet(value: unknown): boolean {
  const record = asRecord(value);
  const type = readString(record, "type", readString(record, "type_", ""));
  return type.endsWith("::pet::KanariPet");
}

function parseKanariPetBcs(value: unknown): NftMetadata | null {
  const record = asRecord(value);
  if (!isKanariPet(record) || !isByteArray(record.data)) return null;

  const bytes = Uint8Array.from(record.data);
  const cursor = { offset: 32 };
  const name = readBcsString(bytes, cursor);
  const imageUrl = readBcsString(bytes, cursor);
  const description = readBcsString(bytes, cursor);
  const attributeCount = readBcsLength(bytes, cursor);
  if (
    name === null ||
    imageUrl === null ||
    description === null ||
    attributeCount === null
  ) {
    return null;
  }

  const attributes: NftMetadata["attributes"] = [];
  for (let i = 0; i < attributeCount; i += 1) {
    const key = readBcsString(bytes, cursor);
    const attributeValue = readBcsString(bytes, cursor);
    if (key === null || attributeValue === null) return null;
    attributes.push({ key, value: attributeValue });
  }

  const number = readBcsString(bytes, cursor);
  const collectionId = readBcsAddress(bytes, cursor);
  const creator = readBcsAddress(bytes, cursor);
  if (number === null || collectionId === null || creator === null) return null;

  return {
    name,
    imageUrl,
    description,
    number,
    collectionId,
    creator,
    attributes,
  };
}

/** Parses the current `kanari_system::krc_721::Nft` object layout. */
function parseKrc721Bcs(value: unknown): NftMetadata | null {
  const record = asRecord(value);
  const type = readString(record, "type", readString(record, "type_", ""));
  if (!type.endsWith("::krc_721::Nft") || !isByteArray(record.data))
    return null;

  const bytes = Uint8Array.from(record.data);
  const cursor = { offset: 32 }; // Nft.id: UID (ID wraps a 32-byte address)
  const collectionId = readBcsAddress(bytes, cursor);
  const number = readBcsU64(bytes, cursor);
  const name = readBcsString(bytes, cursor);
  const description = readBcsOptionString(bytes, cursor);
  const imageUrl = readBcsOptionString(bytes, cursor);
  const attributeCount = readBcsLength(bytes, cursor);

  if (
    collectionId === null ||
    number === null ||
    name === null ||
    description === null ||
    imageUrl === null ||
    attributeCount === null
  ) {
    return null;
  }

  const attributes: NftMetadata["attributes"] = [];
  for (let i = 0; i < attributeCount; i += 1) {
    const key = readBcsString(bytes, cursor);
    const attributeValue = readBcsString(bytes, cursor);
    if (key === null || attributeValue === null) return null;
    attributes.push({ key, value: attributeValue });
  }

  if (cursor.offset !== bytes.length) return null;
  return {
    name,
    imageUrl,
    description,
    number,
    collectionId,
    creator: "",
    attributes,
  };
}

function parsedMetadata(value: unknown): NftMetadata | null {
  const record = asRecord(value);
  const parsed = asRecord(record.parsed);
  if (!Object.keys(parsed).length) return null;

  const keys = Array.isArray(asRecord(parsed.attributes).keys)
    ? (asRecord(parsed.attributes).keys as unknown[])
    : [];
  const values = Array.isArray(asRecord(parsed.attributes).values)
    ? (asRecord(parsed.attributes).values as unknown[])
    : [];
  const attributes = keys.flatMap((key, index) => {
    const attributeValue = values[index];
    return typeof key === "string" && typeof attributeValue === "string"
      ? [{ key, value: attributeValue }]
      : [];
  });

  return {
    name: readString(parsed, "name", ""),
    imageUrl: readString(parsed, "image_url", ""),
    description: readString(parsed, "description", ""),
    number: readString(parsed, "number", ""),
    collectionId: readString(parsed, "collection_id", ""),
    creator: readString(parsed, "creator", ""),
    attributes,
  };
}

export function nftMetadata(value: unknown): NftMetadata | null {
  return (
    parseKrc721Bcs(value) ?? parseKanariPetBcs(value) ?? parsedMetadata(value)
  );
}

function normalizeImageUrl(value: string): string | null {
  const url = value.trim();
  if (!url) return null;
  if (url.startsWith("ipfs://"))
    return `https://ipfs.io/ipfs/${url.slice("ipfs://".length)}`;
  if (url.startsWith("data:image/") && !url.startsWith("data:image/svg"))
    return url;

  try {
    const parsed = new URL(url);
    return parsed.protocol === "https:" || parsed.protocol === "http:"
      ? parsed.toString()
      : null;
  } catch {
    return null;
  }
}

/** Finds common NFT image fields, including Move/RPC metadata envelopes. */
export function nftImageUrl(value: unknown, depth = 0): string | null {
  if (depth > 3) return null;
  const record = asRecord(value);

  const petImage = normalizeImageUrl(nftMetadata(record)?.imageUrl ?? "");
  if (petImage) return petImage;

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

export function NftArtwork({
  item,
  fallback,
  alt,
}: {
  item: unknown;
  fallback: string;
  alt: string;
}) {
  const source = useMemo(() => nftImageUrl(item), [item]);
  const [failed, setFailed] = useState(false);

  useEffect(() => setFailed(false), [source]);

  return (
    <div className="nft-art">
      {source && !failed ? (
        <img
          src={source}
          alt={alt}
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setFailed(true)}
        />
      ) : (
        fallback
      )}
    </div>
  );
}
