"use client";

import Link from "next/link";
import { Suspense, useEffect, useMemo, useState } from "react";
import { useParams } from "next/navigation";
import {
  CopyButton,
  EmptyState,
  PageHeader,
  Panel,
  RawDetails,
  readBoolean,
  StatusPill,
  describeTransactionLifecycle,
  formatBalance,
  formatNumber,
  readAddress,
  readString,
  shortHash,
} from "../../components/ExplorerUI";
import {
  getFungibleAsset,
  getFungibleAssetHolders,
  getFungibleAssetTransactions,
} from "../../lib/rpc";

type AssetTab = "info" | "holders" | "transactions";
const SYSTEM_KANARI_TOKEN_TYPE = "0x2::kanari::KANARI";

function normalizeTokenType(value: string) {
  return value.trim().toLowerCase();
}

function decodeTokenParam(value: string | string[] | undefined) {
  const raw = Array.isArray(value) ? value.join("/") : value || "";
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

function AssetMetric({ label, value, detail }: { label: string; value: string; detail?: string }) {
  return (
    <article className="stat-card">
      <strong>{value}</strong>
      <span>{label}</span>
      {detail ? <p className="stat-card__detail">{detail}</p> : null}
    </article>
  );
}

function FungibleAssetContent() {
  const params = useParams<{ tokenType: string }>();
  const tokenType = useMemo(() => decodeTokenParam(params.tokenType), [params.tokenType]);
  const [asset, setAsset] = useState<unknown>(null);
  const [holders, setHolders] = useState<unknown[]>([]);
  const [transactions, setTransactions] = useState<unknown[]>([]);
  const [activeTab, setActiveTab] = useState<AssetTab>("info");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;

    async function loadAsset() {
      if (!tokenType) return;
      setLoading(true);
      setError("");
      try {
        const [assetInfo, holderList, txList] = await Promise.all([
          getFungibleAsset(tokenType),
          getFungibleAssetHolders(tokenType, 100),
          getFungibleAssetTransactions(tokenType, 50),
        ]);
        if (cancelled) return;
        setAsset(assetInfo);
        setHolders(holderList);
        setTransactions(txList);
      } catch (err) {
        if (cancelled) return;
        setAsset(null);
        setHolders([]);
        setTransactions([]);
        setError(err instanceof Error ? err.message : "Failed to fetch fungible asset");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    void loadAsset();
    return () => {
      cancelled = true;
    };
  }, [tokenType]);

  const symbol = readString(asset, "symbol", tokenType.split("::").slice(-1)[0] || "ASSET");
  const decimals = readString(asset, "decimals", "9");
  const verified = readBoolean(
    asset,
    "verified",
    normalizeTokenType(tokenType) === normalizeTokenType(SYSTEM_KANARI_TOKEN_TYPE),
  );

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="Fungible Asset"
        title={symbol}
        accent="Ledger."
        description="Inspect asset metadata, current holders, and recent transactions from the Kanari RPC asset index."
      >
        <div className="copy-row copy-row--wrap">
          <span className="tag mono">{shortHash(tokenType, 18, 12)}</span>
          <CopyButton value={tokenType} label="Copy token type" />
          <StatusPill label={verified ? "Verified" : "Unverified"} state={verified ? "ok" : "warn"} />
        </div>
      </PageHeader>

      {error ? <div className="empty-state">{error}</div> : null}

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">Asset Overview</h2>
            <p className="panel-subtitle">{tokenType}</p>
          </div>
          <StatusPill label={loading ? "Syncing" : "Loaded"} state={loading ? "warn" : "ok"} />
        </div>
        {loading && !asset ? <EmptyState loading label="Loading asset..." /> : null}
        {asset ? (
          <div className="stat-grid">
            <AssetMetric label="Total Supply" value={formatBalance(readString(asset, "total_supply", "0"), decimals)} detail={symbol} />
            <AssetMetric label="Wallet Visible" value={formatBalance(readString(asset, "wallet_visible_supply", "0"), decimals)} detail="held by wallets" />
            <AssetMetric label="Object Locked" value={formatBalance(readString(asset, "object_locked_supply", "0"), decimals)} detail="held inside objects" />
            <AssetMetric label="Holders" value={formatNumber(readString(asset, "holders_count", String(holders.length)))} detail="positive balances" />
          </div>
        ) : null}
      </section>

      <div className="tabs">
        {(["info", "holders", "transactions"] as AssetTab[]).map((tab) => (
          <button
            className={`tab ${activeTab === tab ? "tab--active" : ""}`}
            key={tab}
            type="button"
            onClick={() => setActiveTab(tab)}
          >
            {tab === "info" ? "Info" : tab === "holders" ? `Holders ${holders.length}` : `Transactions ${transactions.length}`}
          </button>
        ))}
      </div>

      {activeTab === "info" ? <AssetInfoPanel asset={asset} tokenType={tokenType} decimals={decimals} /> : null}
      {activeTab === "holders" ? <AssetHoldersPanel holders={holders} decimals={decimals} symbol={symbol} loading={loading} /> : null}
      {activeTab === "transactions" ? <AssetTransactionsPanel transactions={transactions} loading={loading} /> : null}

      <RawDetails label="Developer: raw fungible asset data" value={{ asset, holders, transactions }} />
    </div>
  );
}

function AssetInfoPanel({ asset, tokenType, decimals }: { asset: unknown; tokenType: string; decimals: string }) {
  const rows = [
    ["Name", readString(asset, "name", "-")],
    ["Symbol", readString(asset, "symbol", "-")],
    ["Decimals", decimals],
    ["Token Type", tokenType],
    ["Circulating Supply", formatBalance(readString(asset, "circulating_supply", "0"), decimals)],
    ["Accounted Supply", formatBalance(readString(asset, "accounted_supply", "0"), decimals)],
    ["Untracked Supply", formatBalance(readString(asset, "untracked_supply", "0"), decimals)],
    ["Description", readString(asset, "description", "-")],
    ["Icon URL", readString(asset, "icon_url", "-")],
  ];

  return (
    <Panel title="Info" subtitle="Canonical metadata and supply fields returned by RPC.">
      <div className="data-list">
        {rows.map(([label, value]) => (
          <div className="data-row" key={label}>
            <div>
              <p className="tiny-label">{label}</p>
              <span className="mono break-anywhere">{value}</span>
            </div>
          </div>
        ))}
      </div>
    </Panel>
  );
}

function AssetHoldersPanel({
  holders,
  decimals,
  symbol,
  loading,
}: {
  holders: unknown[];
  decimals: string;
  symbol: string;
  loading: boolean;
}) {
  return (
    <Panel title="Holders" subtitle="Wallets with current positive asset balance." action={<StatusPill label={loading ? "Syncing" : "Ready"} state={loading ? "warn" : "ok"} />}>
      {loading && holders.length === 0 ? <EmptyState loading label="Loading holders..." /> : null}
      {!loading && holders.length === 0 ? <EmptyState label="No holders found." /> : null}
      {holders.length > 0 ? (
        <div className="data-list">
          {holders.map((holder, index) => {
            const owner = readString(holder, "owner", "-");
            return (
              <div className="data-row data-row--account" key={`${owner}-${index}`}>
                <div>
                  <p className="tiny-label">Holder</p>
                  <span className="copy-row copy-row--wrap">
                    <Link className="text-link mono break-anywhere" href={`/account?address=${encodeURIComponent(owner)}`}>
                      {owner}
                    </Link>
                    <CopyButton value={owner} label="Copy holder address" />
                  </span>
                </div>
                <div>
                  <p className="tiny-label">Balance</p>
                  <span className="mono">{formatBalance(readString(holder, "balance", "0"), decimals)} {symbol}</span>
                </div>
                <div>
                  <p className="tiny-label">Coin Objects</p>
                  <span className="mono">{readString(holder, "coin_object_count", "0")}</span>
                </div>
              </div>
            );
          })}
        </div>
      ) : null}
    </Panel>
  );
}

function AssetTransactionsPanel({ transactions, loading }: { transactions: unknown[]; loading: boolean }) {
  return (
    <Panel title="Transactions" subtitle="Recent transactions that mention this asset." action={<StatusPill label={loading ? "Syncing" : "Live"} state={loading ? "warn" : "ok"} />}>
      {loading && transactions.length === 0 ? <EmptyState loading label="Loading transactions..." /> : null}
      {!loading && transactions.length === 0 ? <EmptyState label="No asset transactions found." /> : null}
      {transactions.length > 0 ? (
        <div className="data-list">
          {transactions.map((transaction, index) => {
            const hash = readString(transaction, "hash", `tx-${index}`);
            const lifecycle = describeTransactionLifecycle(transaction);
            const sender = readAddress(transaction, "sender_address", "sender");
            return (
              <div className="data-row" key={`${hash}-${index}`}>
                <div>
                  <p className="tiny-label">Txn Hash</p>
                  <span className="copy-row copy-row--wrap">
                    <span className="mono break-anywhere">{shortHash(hash, 18, 14)}</span>
                    <CopyButton value={hash} label="Copy transaction hash" />
                  </span>
                </div>
                <div>
                  <p className="tiny-label">Type</p>
                  <span className="tag">{readString(transaction, "tx_type", "operation").replace(/_/g, " ")}</span>
                </div>
                <div>
                  <p className="tiny-label">Sender</p>
                  <Link className="text-link mono break-anywhere" href={`/account?address=${encodeURIComponent(sender)}`}>
                    {shortHash(sender, 14, 10)}
                  </Link>
                </div>
                <div>
                  <p className="tiny-label">Status</p>
                  <StatusPill label={lifecycle.label} state={lifecycle.state} />
                </div>
              </div>
            );
          })}
        </div>
      ) : null}
    </Panel>
  );
}

export default function FungibleAssetPage() {
  return (
    <Suspense fallback={<EmptyState loading label="Loading asset..." />}>
      <FungibleAssetContent />
    </Suspense>
  );
}
