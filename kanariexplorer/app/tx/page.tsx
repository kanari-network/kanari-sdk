"use client";

import Link from "next/link";
import { Suspense, useEffect, useRef, useState } from "react";
import { useSearchParams } from "next/navigation";
import TransactionDetailsModal from "../components/TransactionDetailsModal";
import {
  asArray,
  CopyButton,
  describeTransactionLifecycle,
  EmptyState,
  PageHeader,
  RawDetails,
  readAddress,
  readString,
  SearchForm,
  StatusPill,
} from "../components/ExplorerUI";
import { getAllTransactions, getTransaction } from "../lib/rpc";

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

function TxContent() {
  const searchParams = useSearchParams();
  const [search, setSearch] = useState(searchParams.get("hash") ?? "");
  const [transactions, setTransactions] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedTransaction, setSelectedTransaction] = useState<unknown>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [modalLoading, setModalLoading] = useState(false);
  const searchRef = useRef(search);

  useEffect(() => {
    searchRef.current = search;
  }, [search]);

  async function fetchTransactions(query = search) {
    setLoading(true);
    try {
      const trimmed = query.trim();
      searchRef.current = query;
      if (trimmed.length > 40) {
        const transaction = await getTransaction(trimmed);
        setTransactions(transaction ? [transaction] : []);
      } else {
        const response = await getAllTransactions(50, trimmed || undefined);
        const nextTransactions = asArray(response);
        setTransactions((current) => (nextTransactions.length > 0 || trimmed ? nextTransactions : current));
      }
    } catch {
      if (query.trim()) setTransactions([]);
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
    const initialQuery = searchParams.get("hash") ?? "";
    const timeout = window.setTimeout(() => {
      void fetchTransactions(initialQuery);
    }, 0);
    const interval = window.setInterval(() => {
      void fetchTransactions(searchRef.current);
    }, 10000);
    return () => {
      window.clearTimeout(timeout);
      window.clearInterval(interval);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="Network Activity"
        title="Transaction"
        accent="Explorer."
        description="Search transaction hashes, inspect sender activity, and watch the latest Kanari operations refresh in near real time."
      >
        <SearchForm value={search} onChange={setSearch} onSubmit={() => fetchTransactions()} placeholder="Filter by hash or address" buttonLabel="Filter" />
      </PageHeader>

      <PanelTransactions transactions={transactions} loading={loading} onOpen={openTransaction} />
      <RawDetails label="Developer: latest transaction JSON" value={transactions} />

      <TransactionDetailsModal open={modalOpen} loading={modalLoading} transaction={selectedTransaction} onClose={() => setModalOpen(false)} />
    </div>
  );
}

function PanelTransactions({
  transactions,
  loading,
  onOpen,
}: {
  transactions: unknown[];
  loading: boolean;
  onOpen: (hash: string) => void;
}) {
  return (
    <section className="panel">
      <div className="panel-head">
        <div>
          <h2 className="panel-title">Latest Transactions</h2>
          <p className="panel-subtitle">Showing {transactions.length} operations</p>
        </div>
        <StatusPill label={loading ? "Syncing" : "Live"} state={loading ? "warn" : "ok"} />
      </div>
      {loading && transactions.length === 0 ? <EmptyState loading label="Fetching network activity..." /> : null}
      {!loading && transactions.length === 0 ? <EmptyState label="No transactions found." /> : null}
      {transactions.length > 0 ? (
        <div className="data-list">
          {transactions.map((transaction, index) => {
            const fallbackHash = `transaction-${index}`;
            const hash = readTransactionHash(transaction, fallbackHash);
            const canOpen = hash !== fallbackHash;
            const lifecycle = describeTransactionLifecycle(transaction);
            const senderAddress = readAddress(transaction, "sender_address", "sender");
            const objectInputs = readArrayLength(transaction, "object_inputs");
            const objectChanges = readEffectArrayLength(transaction, "object_changes");
            const graphEdges = readEffectArrayLength(transaction, "causal_edges");
            return (
              <div className="data-row" key={`${hash}-${index}`}>
                <div>
                  <p className="tiny-label">Txn Hash</p>
                  <span className="copy-row copy-row--wrap">
                    {canOpen ? (
                      <button className="hash-button mono break-anywhere" type="button" onClick={() => onOpen(hash)}>
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
                  <span className="tag">{readString(transaction, "tx_type", "operation").replace(/_/g, " ")}</span>
                </div>
                <div>
                  <p className="tiny-label">Sender</p>
                  {senderAddress === "-" ? (
                    <span className="mono muted-text">-</span>
                  ) : (
                    <span className="copy-row copy-row--wrap">
                      <Link className="text-link mono break-anywhere" href={`/account?address=${encodeURIComponent(senderAddress)}`}>
                        {senderAddress}
                      </Link>
                      <CopyButton value={senderAddress} label="Copy sender address" />
                    </span>
                  )}
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
      ) : null}
    </section>
  );
}

export default function TxPage() {
  return (
    <Suspense fallback={<EmptyState loading label="Loading explorer..." />}>
      <TxContent />
    </Suspense>
  );
}
