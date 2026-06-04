"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import TransactionDetailsModal from "../components/TransactionDetailsModal";
import {
  asArray,
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
      setObjects(asArray(objectData));
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
        <section className="panel" style={{ marginTop: 18 }}>
          <div className="panel-head">
            <div>
              <h2 className="panel-title">Account State</h2>
              <p className="panel-subtitle mono">{readString(account, "address", address)}</p>
            </div>
            <StatusPill label={`Sequence ${readString(account, "sequence_number", "0")}`} />
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
            {balances.map((token, index) => (
              <div className="data-row data-row--account" key={`${readString(token, "symbol", "token")}-${index}`}>
                <div className="primary-text">
                  <strong>{readString(token, "name", readString(token, "symbol", "Token"))}</strong>
                  <div className="muted-text mono">{readString(token, "symbol", readString(token, "token_type", "-"))}</div>
                </div>
                <div>
                  <p className="tiny-label">Balance</p>
                  <span className="mono">{formatBalance(readString(token, "amount", readString(token, "balance", "0")), readString(token, "decimals", "9"))}</span>
                </div>
              </div>
            ))}
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
              const hash = readString(transaction, "hash", `transaction-${index}`);
              return (
                <div className="data-row" key={`${hash}-${index}`}>
                  <div>
                    <p className="tiny-label">Txn Hash</p>
                    <button className="hash-button mono" type="button" onClick={() => openTransaction(hash)}>
                      {shortHash(hash)}
                    </button>
                  </div>
                  <div>
                    <p className="tiny-label">Type</p>
                    <span className="tag">{readString(transaction, "tx_type", "operation")}</span>
                  </div>
                  <div>
                    <p className="tiny-label">Target</p>
                    <span className="mono muted-text">{readString(transaction, "module", "-")}</span>
                  </div>
                  <div>
                    <p className="tiny-label">Status</p>
                    <StatusPill label={readString(transaction, "status", "unknown")} />
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
              return (
                <div className="data-row" key={`${objectId}-${index}`}>
                  <div className="primary-text">
                    <strong className="mono">{shortHash(objectId)}</strong>
                    <div className="muted-text mono">{readString(object, "type", readString(object, "object_type", "object"))}</div>
                  </div>
                  <div>
                    <p className="tiny-label">Owner</p>
                    <span className="mono muted-text">{shortHash(readString(object, "owner", address))}</span>
                  </div>
                  <div>
                    <p className="tiny-label">Version</p>
                    <span className="mono">{readString(object, "version", "-")}</span>
                  </div>
                  <div>
                    <p className="tiny-label">Status</p>
                    <StatusPill label={readString(object, "status", "owned")} />
                  </div>
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
