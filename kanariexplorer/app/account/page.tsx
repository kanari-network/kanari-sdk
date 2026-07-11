"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import TransactionDetailsModal from "../components/TransactionDetailsModal";
import {
  asArray,
  CopyButton,
  describeTransactionLifecycle,
  EmptyState,
  formatBalance,
  PageHeader,
  RawDetails,
  readString,
  SearchForm,
  shortHash,
  StatusPill,
} from "../components/ExplorerUI";
import { getAccount, getAllBalances, getAllTransactions, getOwnedNfts, getOwnedObjects, getTransaction } from "../lib/rpc";

type AccountTab = "coins" | "nfts" | "objects" | "activity";

function readBytes(value: unknown, key: string) {
  const record = typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
  const item = record[key];
  if (!Array.isArray(item)) return [];
  return item.filter((entry): entry is number => typeof entry === "number" && Number.isFinite(entry));
}

function readArrayField(value: unknown, key: string) {
  const record = typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
  const item = record[key];
  return Array.isArray(item) ? item : [];
}

function formatHex(bytes: number[]) {
  if (bytes.length === 0) return "-";
  return `0x${bytes.map((byte) => Math.max(0, Math.min(255, byte)).toString(16).padStart(2, "0")).join("")}`;
}

function readCoinBalanceMist(bytes: number[]) {
  if (bytes.length < 40) return null;
  let value = BigInt(0);
  for (let index = 0; index < 8; index += 1) {
    value += BigInt(Math.max(0, Math.min(255, bytes[32 + index]))) << BigInt(index * 8);
  }
  return value.toString();
}

function isCoinType(type: string) {
  return type.includes("::coin::Coin<");
}

function dedupeObjects(values: unknown[]) {
  const seen = new Set<string>();
  return values.filter((value, index) => {
    const id = readString(value, "object_id", readString(value, "id", `object-${index}`));
    const normalized = id.toLowerCase();
    if (seen.has(normalized)) return false;
    seen.add(normalized);
    return true;
  });
}

function readTransactionHash(transaction: unknown, fallback: string) {
  return readString(
    transaction,
    "hash",
    readString(transaction, "tx_hash", readString(transaction, "transaction_hash", readString(transaction, "digest", fallback))),
  );
}

function readObject(source: unknown, key: string) {
  if (typeof source !== "object" || source === null || Array.isArray(source)) return null;
  const value = (source as Record<string, unknown>)[key];
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function readArrayLength(source: unknown, key: string) {
  if (typeof source !== "object" || source === null || Array.isArray(source)) return 0;
  const value = (source as Record<string, unknown>)[key];
  return Array.isArray(value) ? value.length : 0;
}

function readEffectArrayLength(transaction: unknown, key: string) {
  const effects = readObject(transaction, "effects");
  if (!effects) return 0;
  const value = effects[key];
  return Array.isArray(value) ? value.length : 0;
}

function readOwnerKindLabel(value: unknown) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return "address owner";
  const ownerKind = (value as Record<string, unknown>).owner_kind;
  if (typeof ownerKind === "string" && ownerKind) return ownerKind.replace(/_/g, " ");
  if (ownerKind && typeof ownerKind === "object" && !Array.isArray(ownerKind)) {
    const record = ownerKind as Record<string, unknown>;
    if ("AddressOwner" in record) return "address owner";
    if ("Shared" in record) return "shared";
    if ("Immutable" in record) return "immutable";
  }
  return "address owner";
}

function AccountContent() {
  const searchParams = useSearchParams();
  const [address, setAddress] = useState(searchParams.get("address") ?? "");
  const [account, setAccount] = useState<unknown>(null);
  const [balances, setBalances] = useState<unknown[]>([]);
  const [transactions, setTransactions] = useState<unknown[]>([]);
  const [nfts, setNfts] = useState<unknown[]>([]);
  const [objects, setObjects] = useState<unknown[]>([]);
  const [activeTab, setActiveTab] = useState<AccountTab>("coins");
  const [loading, setLoading] = useState(false);
  const [selectedTransaction, setSelectedTransaction] = useState<unknown>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [modalLoading, setModalLoading] = useState(false);

  async function loadAccount(target = address) {
    const trimmed = target.trim();
    if (!trimmed) return;

    setLoading(true);
    try {
      const [accountData, balanceData, transactionData, nftData, objectData] = await Promise.all([
        getAccount(trimmed).catch(() => null),
        getAllBalances(trimmed).catch(() => []),
        getAllTransactions(50, trimmed).catch(() => []),
        getOwnedNfts(trimmed).catch(() => []),
        getOwnedObjects(trimmed).catch(() => []),
      ]);
      setAccount(accountData);
      setBalances(asArray(balanceData));
      setTransactions(asArray(transactionData));
      setNfts(asArray(nftData));
      setObjects(dedupeObjects([...asArray(objectData), ...readArrayField(accountData, "owned_objects")]));
    } finally {
      setLoading(false);
    }
  }

  async function openTransaction(hash: string) {
    setModalOpen(true);
    setModalLoading(true);
    setSelectedTransaction(null);
    try {
      setSelectedTransaction(await getTransaction(hash));
    } catch {
      setSelectedTransaction(null);
    } finally {
      setModalLoading(false);
    }
  }

  useEffect(() => {
    const queryAddress = searchParams.get("address");
    if (queryAddress) {
      const timeout = window.setTimeout(() => {
        void loadAccount(queryAddress);
      }, 0);
      return () => window.clearTimeout(timeout);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchParams]);

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="Account Lookup"
        title="Account"
        accent="Explorer."
        description="Inspect balances, NFTs, and transaction activity for any Kanari account address."
      >
        <SearchForm value={address} onChange={setAddress} onSubmit={() => loadAccount()} placeholder="Enter address 0x..." />
      </PageHeader>

      {account ? (
        <section className="panel account-state-panel">
          <div className="panel-head">
            <div>
              <h2 className="panel-title">Account State</h2>
              <div className="panel-subtitle mono account-address-copy">
                <span className="copy-row copy-row--wrap">
                  <span className="break-anywhere">{readString(account, "address", address)}</span>
                  <CopyButton value={readString(account, "address", address)} label="Copy account address" />
                </span>
              </div>
            </div>
            <StatusPill label={`Objects ${readString(account, "owned_object_count", String(objects.length))}`} />
          </div>
        </section>
      ) : null}

      <div className="tabs">
        {[
          ["coins", `Coins ${balances.length}`],
          ["nfts", `NFTs ${nfts.length}`],
          ["objects", `Objects ${objects.length}`],
          ["activity", `Activity ${transactions.length}`],
        ].map(([id, label]) => (
          <button className={`tab ${activeTab === id ? "tab--active" : ""}`} key={id} type="button" onClick={() => setActiveTab(id as AccountTab)}>
            {label}
          </button>
        ))}
      </div>

      {loading ? <EmptyState loading label="Loading account data..." /> : null}

      {!loading && activeTab === "coins" ? (
        <section className="panel">
          <div className="panel-head">
            <h2 className="panel-title">Coin Balances</h2>
          </div>
          {balances.length === 0 ? <EmptyState label="No balances found." /> : null}
          <div className="data-list">
            {balances.map((token, index) => {
              const tokenType = readString(token, "token_type", readString(token, "token", readString(token, "symbol", "-")));
              const symbol = readString(token, "symbol", tokenType.split("::").slice(-1)[0] || "Token");
              return (
                <div className="data-row data-row--account" key={`${tokenType}-${index}`}>
                  <div className="primary-text">
                    <strong>
                      <Link className="text-link" href={`/coins/${encodeURIComponent(tokenType)}`}>
                        {readString(token, "name", symbol)}
                      </Link>
                    </strong>
                    <div className="muted-text mono break-anywhere">{tokenType}</div>
                  </div>
                  <div>
                    <p className="tiny-label">Balance</p>
                    <span className="mono">
                      {formatBalance(readString(token, "amount", readString(token, "balance", "0")), readString(token, "decimals", "9"))} {symbol}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      ) : null}

      {!loading && activeTab === "nfts" ? (
        nfts.length === 0 ? (
          <EmptyState label="No NFTs owned by this address." />
        ) : (
          <div className="nft-grid">
            {nfts.map((nft, index) => {
              const objectId = readString(nft, "object_id", `nft-${index}`);
              return (
                <article className="nft-card" key={objectId}>
                  <div className="nft-art">#{objectId.slice(-4)}</div>
                  <div className="nft-copy">
                    <strong className="primary-text mono">{shortHash(objectId)}</strong>
                    <p className="muted-text">{readString(nft, "type", "NFT")}</p>
                  </div>
                </article>
              );
            })}
          </div>
        )
      ) : null}

      {!loading && activeTab === "activity" ? (
        <section className="panel">
          <div className="panel-head">
            <h2 className="panel-title">Account Activity</h2>
          </div>
          {transactions.length === 0 ? <EmptyState label="No transactions found." /> : null}
          <div className="data-list">
            {transactions.map((transaction, index) => {
              const fallbackHash = `transaction-${index}`;
              const hash = readTransactionHash(transaction, fallbackHash);
              const canOpen = hash !== fallbackHash;
              const lifecycle = describeTransactionLifecycle(transaction);
              const objectInputs = readArrayLength(transaction, "object_inputs");
              const objectChanges = readEffectArrayLength(transaction, "object_changes");
              const graphEdges = readEffectArrayLength(transaction, "causal_edges");
              return (
                <div className="data-row" key={`${hash}-${index}`}>
                  <div>
                    <p className="tiny-label">Txn Hash</p>
                    <span className="copy-row copy-row--wrap">
                      {canOpen ? (
                        <button className="hash-button mono break-anywhere" type="button" onClick={() => openTransaction(hash)}>
                          {hash}
                        </button>
                      ) : (
                        <span className="mono muted-text">{hash}</span>
                      )}
                      {canOpen ? <CopyButton value={hash} label="Copy transaction hash" /> : null}
                    </span>
                  </div>
                  <div>
                    <p className="tiny-label">Type</p>
                    <span className="tag">{readString(transaction, "tx_type", "operation")}</span>
                  </div>
                  <div>
                    <p className="tiny-label">Target</p>
                    <span className="mono muted-text">{shortHash(readString(transaction, "module", "-"))}</span>
                  </div>
                  <div>
                    <p className="tiny-label">Status</p>
                    <StatusPill label={lifecycle.label} state={lifecycle.state} />
                    {lifecycle.detail ? <div className="mono muted-text">{lifecycle.detail}</div> : null}
                  </div>
                  <div>
                    <p className="tiny-label">Objects</p>
                    <span className="mono muted-text">
                      {objectInputs} inputs / {objectChanges} changes / {graphEdges} edges
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      ) : null}

      {!loading && activeTab === "objects" ? (
        <section className="panel">
          <div className="panel-head">
            <h2 className="panel-title">Owned Objects</h2>
          </div>
          {objects.length === 0 ? <EmptyState label="No objects owned by this address." /> : null}
          <div className="data-list">
            {objects.map((object, index) => {
              const objectId = readString(object, "object_id", readString(object, "id", `object-${index}`));
              const objectType = readString(object, "type_", readString(object, "type", readString(object, "object_type", "-")));
              const owner = readString(object, "owner", address);
              const ownerKind = readOwnerKindLabel(object);
              const dataBytes = readBytes(object, "data");
              const coinBalanceMist = isCoinType(objectType) ? readCoinBalanceMist(dataBytes) : null;
              const objectJson =
                object && typeof object === "object" && !Array.isArray(object)
                  ? { ...(object as Record<string, unknown>), data_hex: formatHex(dataBytes) }
                  : { value: object, data_hex: formatHex(dataBytes) };
              return (
                <div className="data-row data-row--objects" key={`${objectId}-${index}`}>
                  <div className="object-main primary-text">
                    <p className="tiny-label">Object ID</p>
                    <span className="copy-row copy-row--inline">
                      <strong className="mono break-anywhere">{objectId}</strong>
                      <CopyButton value={objectId} label="Copy object id" />
                    </span>
                  </div>

                  <div className="object-detail-grid">
                    <div className="object-detail-field object-detail-field--wide">
                      <p className="tiny-label">Type</p>
                      <span className="mono muted-text break-anywhere">{objectType}</span>
                    </div>
                    <div className="object-detail-field object-detail-field--wide">
                      <p className="tiny-label">Owner</p>
                      <span className="copy-row copy-row--inline">
                        <span className="mono muted-text break-anywhere">{owner}</span>
                        <CopyButton value={owner} label="Copy owner address" />
                      </span>
                    </div>
                    <div className="object-detail-field">
                      <p className="tiny-label">Owner Kind</p>
                      <span className="mono">{ownerKind}</span>
                    </div>
                    <div className="object-detail-field">
                      <p className="tiny-label">Version</p>
                      <span className="mono">{readString(object, "version", "-")}</span>
                    </div>
                    <div className="object-detail-field">
                      <p className="tiny-label">Digest</p>
                      <span className="mono break-anywhere">{readString(object, "digest", "-")}</span>
                    </div>
                    <div className="object-detail-field">
                      <p className="tiny-label">Data Bytes</p>
                      <span className="mono">{dataBytes.length.toLocaleString()}</span>
                    </div>
                    {coinBalanceMist !== null ? (
                      <div className="object-detail-field">
                        <p className="tiny-label">Coin Balance</p>
                        <span className="mono">{formatBalance(coinBalanceMist, "9")}</span>
                      </div>
                    ) : null}
                    <div className="object-detail-field">
                      <p className="tiny-label">Status</p>
                      <StatusPill label={readString(object, "status", "owned")} />
                    </div>
                  </div>

                  <details className="object-json-details">
                    <summary>Object JSON</summary>
                    <pre className="custom-scrollbar">{JSON.stringify(objectJson, null, 2)}</pre>
                  </details>
                </div>
              );
            })}
          </div>
        </section>
      ) : null}

      <RawDetails label="Developer: raw account state" value={{ account, balances, nfts, objects, transactions }} />
      <TransactionDetailsModal open={modalOpen} loading={modalLoading} transaction={selectedTransaction} onClose={() => setModalOpen(false)} />
    </div>
  );
}

export default function AccountPage() {
  return (
    <Suspense fallback={<EmptyState loading label="Loading account explorer..." />}>
      <AccountContent />
    </Suspense>
  );
}
