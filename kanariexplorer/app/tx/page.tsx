"use client";

import Link from "next/link";
import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import TransactionDetailsModal from "../components/TransactionDetailsModal";
import { asArray, EmptyState, PageHeader, RawDetails, readString, SearchForm, shortHash, StatusPill } from "../components/ExplorerUI";
import { getAllTransactions, getTransaction } from "../lib/rpc";

function TxContent() {
  const searchParams = useSearchParams();
  const [search, setSearch] = useState(searchParams.get("hash") ?? "");
  const [transactions, setTransactions] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedTransaction, setSelectedTransaction] = useState<unknown>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [modalLoading, setModalLoading] = useState(false);

  async function fetchTransactions(query = search) {
    setLoading(true);
    try {
      const trimmed = query.trim();
      if (trimmed.length > 40) {
        const transaction = await getTransaction(trimmed);
        setTransactions(transaction ? [transaction] : []);
      } else {
        const response = await getAllTransactions(50, trimmed || undefined);
        setTransactions(asArray(response));
      }
    } catch {
      setTransactions([]);
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
      void fetchTransactions(search);
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
    <section className="panel" style={{ marginTop: 18 }}>
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
            const hash = readString(transaction, "hash", `transaction-${index}`);
            const status = readString(transaction, "status", "unknown");
            return (
              <div className="data-row" key={`${hash}-${index}`}>
                <div>
                  <p className="tiny-label">Txn Hash</p>
                  <button className="hash-button mono" type="button" onClick={() => onOpen(hash)}>
                    {shortHash(hash)}
                  </button>
                </div>
                <div>
                  <p className="tiny-label">Type</p>
                  <span className="tag">{readString(transaction, "tx_type", "operation").replace(/_/g, " ")}</span>
                </div>
                <div>
                  <p className="tiny-label">Sender</p>
                  <Link className="text-link mono" href={`/account?address=${encodeURIComponent(readString(transaction, "sender", ""))}`}>
                    {shortHash(readString(transaction, "sender", "-"))}
                  </Link>
                </div>
                <div>
                  <p className="tiny-label">Status</p>
                  <StatusPill label={status} state={status === "pending" ? "warn" : "ok"} />
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
