"use client";

import Image from "next/image";
import { Suspense, useEffect, useState } from "react";
import { asArray, EmptyState, formatBalance, PageHeader, RawDetails, readString, SearchForm, StatusPill } from "../components/ExplorerUI";
import { getAllBalances, getTokens } from "../lib/rpc";

function pickSupply(token: unknown) {
  const fields = ["accounted_supply", "total_supply", "wallet_visible_supply", "circulating_supply", "amount", "balance"];
  for (const field of fields) {
    const value = readString(token, field, "");
    if (value && value !== "0") return value;
  }
  return "0";
}

function getTokenIcon(token: unknown, symbol: string) {
  const iconUrl = readString(token, "icon_url", readString(token, "logo_url", readString(token, "image_url", "")));
  if (iconUrl) return iconUrl;
  return symbol.toUpperCase() === "KANARI" ? "/kariicon1.png" : "";
}

function CoinsContent() {
  const [address, setAddress] = useState("");
  const [tokens, setTokens] = useState<unknown[]>([]);
  const [registryTokens, setRegistryTokens] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(true);

  async function loadRegistry() {
    setLoading(true);
    try {
      const response = await getTokens();
      const items = asArray(response);
      setRegistryTokens(items);
      setTokens(items);
    } catch {
      setRegistryTokens([]);
      setTokens([]);
    } finally {
      setLoading(false);
    }
  }

  async function loadBalances(target: string) {
    if (!target.trim()) {
      setTokens(registryTokens);
      return;
    }

    setLoading(true);
    try {
      setTokens(asArray(await getAllBalances(target.trim())));
    } catch {
      setTokens([]);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      void loadRegistry();
    }, 0);
    return () => window.clearTimeout(timeout);
  }, []);

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="Network Assets"
        title="Token"
        accent="Explorer."
        description="Browse token registry data or enter an address to view balances in one consistent Kanari explorer surface."
      >
        <SearchForm value={address} onChange={setAddress} onSubmit={() => loadBalances(address)} placeholder="Filter by address" buttonLabel="View" />
      </PageHeader>

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">{address ? "Address Balances" : "Token Registry"}</h2>
            <p className="panel-subtitle">Showing {tokens.length} assets</p>
          </div>
          <StatusPill label={loading ? "Syncing" : "Ready"} state={loading ? "warn" : "ok"} />
        </div>
        {loading ? <EmptyState loading label="Syncing registry assets..." /> : null}
        {!loading && tokens.length === 0 ? <EmptyState label="No tokens found." /> : null}
        {tokens.length > 0 ? (
          <div className="data-list">
            {tokens.map((token, index) => {
              const symbol = readString(token, "symbol", "UNK");
              const decimals = readString(token, "decimals", "9");
              const amount = address ? readString(token, "amount", readString(token, "balance", "0")) : pickSupply(token);
              const icon = getTokenIcon(token, symbol);
              return (
                <div className="data-row data-row--tokens" key={`${symbol}-${index}`}>
                  <div className="token-identity primary-text">
                    <span className="token-logo" aria-hidden="true">
                      {icon ? (
                        <Image src={icon} alt="" width={42} height={42} unoptimized={icon.startsWith("http")} />
                      ) : (
                        symbol.slice(0, 2).toUpperCase()
                      )}
                    </span>
                    <span>
                      <strong>{readString(token, "name", symbol)}</strong>
                      <span className="muted-text mono">{readString(token, "token_type", readString(token, "token", symbol))}</span>
                    </span>
                  </div>
                  <div>
                    <p className="tiny-label">Symbol</p>
                    <span className="tag">{symbol}</span>
                  </div>
                  <div>
                    <p className="tiny-label">{address ? "Balance" : "Supply"}</p>
                    <span className="mono">{formatBalance(amount, decimals)}</span>
                  </div>
                </div>
              );
            })}
          </div>
        ) : null}
      </section>

      <RawDetails label="Developer: token registry data" value={tokens} />
    </div>
  );
}

export default function CoinsPage() {
  return (
    <Suspense fallback={<EmptyState loading label="Loading explorer..." />}>
      <CoinsContent />
    </Suspense>
  );
}
