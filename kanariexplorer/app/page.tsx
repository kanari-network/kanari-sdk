"use client";

import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import {
  deriveAuthorityRpcEndpoints,
  getBlock,
  getBlockHeight,
  getFullBlock,
  getNetworkStatus,
  getNodeHealth,
  getTokens,
  RPC_ENDPOINTS,
  type NodeHealth,
  type RpcEndpoint,
} from "./lib/rpc";
import { asArray, formatNumber, Panel, readString, SearchForm, StatCard, StatusPill } from "./components/ExplorerUI";

function shortUrl(url: string) {
  return url.replace(/^https?:\/\//, "").replace(/\/$/, "");
}

export default function Home() {
  const [search, setSearch] = useState("");
  const [tokenCount, setTokenCount] = useState<number | null>(null);
  const [blockHeight, setBlockHeight] = useState<number | null>(null);
  const [configuredEndpoints, setConfiguredEndpoints] = useState<RpcEndpoint[]>(RPC_ENDPOINTS);
  const [latestBlock, setLatestBlock] = useState<unknown>(null);
  const [nodes, setNodes] = useState<NodeHealth[]>([]);
  const [lastUpdated, setLastUpdated] = useState("");
  const router = useRouter();

  const onlineNodes = nodes.filter((node) => node.online);
  const maxHeight = Math.max(0, ...nodes.map((node) => node.height ?? 0));
  const syncedNodes = onlineNodes.filter((node) => (node.height ?? 0) >= maxHeight).length;
  const laggingNodes = onlineNodes.length - syncedNodes;
  const offlineNodes = nodes.length - onlineNodes.length;
  const totalTransactions = nodes.find((node) => node.totalTransactions !== null)?.totalTransactions ?? null;
  const totalAccounts = nodes.find((node) => node.totalAccounts !== null)?.totalAccounts ?? null;
  const pendingTransactions = nodes.reduce((sum, node) => sum + (node.pendingTransactions ?? 0), 0);

  const networkStatusLabel = useMemo(() => {
    if (nodes.length === 0) return "Loading network";
    if (onlineNodes.length === nodes.length && laggingNodes === 0) return "All nodes synced";
    if (onlineNodes.length === nodes.length) return "Nodes syncing";
    if (onlineNodes.length > 0) return "Partial outage";
    return "Network offline";
  }, [laggingNodes, nodes.length, onlineNodes.length]);

  useEffect(() => {
    async function fetchNetworkData() {
      const seedEndpoint = RPC_ENDPOINTS[0];
      const [tokensRes, heightRes, networkStatus] = await Promise.all([
        getTokens().catch(() => null),
        getBlockHeight().catch(() => null),
        getNetworkStatus(seedEndpoint?.url).catch(() => null),
      ]);

      const nextEndpoints =
        RPC_ENDPOINTS.length === 1 && seedEndpoint && networkStatus?.authorities?.length
          ? deriveAuthorityRpcEndpoints(seedEndpoint.url, networkStatus)
          : RPC_ENDPOINTS;
      const nodeResults = await Promise.all(nextEndpoints.map((endpoint) => getNodeHealth(endpoint)));
      const parsedHeight = typeof heightRes === "number" ? heightRes : Number(heightRes);
      const latestHeight = Number.isFinite(parsedHeight) ? parsedHeight : null;
      const blockRes =
        latestHeight === null
          ? null
          : await getFullBlock(latestHeight).catch(() => getBlock(latestHeight).catch(() => null));

      setTokenCount(asArray(tokensRes).length || null);
      setBlockHeight(latestHeight);
      setLatestBlock(blockRes);
      setConfiguredEndpoints(nextEndpoints);
      setNodes(nodeResults);
      setLastUpdated(new Date().toLocaleTimeString());
    }

    fetchNetworkData();
    const interval = window.setInterval(fetchNetworkData, 5000);
    return () => window.clearInterval(interval);
  }, []);

  function handleSearch() {
    const query = search.trim();
    if (!query) return;
    router.push(query.length > 44 ? `/tx?hash=${encodeURIComponent(query)}` : `/account?address=${encodeURIComponent(query)}`);
  }

  return (
    <div className="explorer-wrap">
      <section className="hero-section">
        <p className="eyebrow">{networkStatusLabel}</p>
        <h1 className="hero-title">
          Explore
          <br />
          <span>Kanari.</span>
        </h1>
        <p className="hero-copy">
          Track node health, transactions, token registry, accounts, and NFT collections across the Kanari event-driven ledger.
        </p>
        <div className="hero-actions">
          <SearchForm value={search} onChange={setSearch} onSubmit={handleSearch} placeholder="Search address or transaction hash" />
        </div>
      </section>

      <section className="grid-cards">
        <StatCard label="Nodes" value={`${onlineNodes.length}/${nodes.length || configuredEndpoints.length}`} detail={offlineNodes ? `${offlineNodes} offline` : `${laggingNodes} lagging`} />
        <StatCard label="Height" value={formatNumber(maxHeight || blockHeight)} detail="Highest reported block height" />
        <StatCard label="Transactions" value={formatNumber(totalTransactions)} detail={`${formatNumber(pendingTransactions)} pending in mempool`} />
        <StatCard label="Tokens" value={formatNumber(tokenCount)} detail={`${formatNumber(totalAccounts)} accounts indexed`} />
      </section>

      <section className="content-grid">
        <Panel
          title="Node Status"
          subtitle={`Refreshes every 5 seconds${lastUpdated ? `, last ${lastUpdated}` : ""}`}
          action={<StatusPill label={`${syncedNodes} synced`} state={offlineNodes ? "warn" : "ok"} />}
        >
          <div className="data-list">
            {(nodes.length > 0
              ? nodes
              : configuredEndpoints.map(
                (endpoint): NodeHealth => ({
                  endpoint,
                  error: undefined,
                  height: null,
                  latencyMs: null,
                  online: false,
                  pendingTransactions: null,
                  status: "loading",
                  totalAccounts: null,
                  totalTransactions: null,
                }),
              )
            ).map((node) => {
              const lag = maxHeight - (node.height ?? 0);
              return (
                <div className="data-row" key={node.endpoint.url}>
                  <div className="primary-text">
                    <div>
                      <span className={`dot ${node.online ? "" : "dot--down"}`} /> {node.endpoint.name}
                    </div>
                    <div className="muted-text mono">{shortUrl(node.endpoint.url)}</div>
                  </div>
                  <div>
                    <p className="tiny-label">Status</p>
                    <StatusPill label={!node.online ? node.status : lag > 0 ? `Lag ${lag}` : "Synced"} state={!node.online ? "down" : lag > 0 ? "warn" : "ok"} />
                  </div>
                  <div>
                    <p className="tiny-label">Height</p>
                    <span className="mono">{formatNumber(node.height)}</span>
                  </div>
                  <div>
                    <p className="tiny-label">Latency</p>
                    <span className="mono">{node.latencyMs === null ? "-" : `${node.latencyMs} ms`}</span>
                  </div>
                </div>
              );
            })}
          </div>
        </Panel>

        <Panel
          title="Latest Block"
          subtitle={blockHeight === null ? "Waiting for block height" : `Height ${formatNumber(blockHeight)}`}
          action={<StatusPill label={latestBlock ? "Loaded" : "Pending"} state={latestBlock ? "ok" : "warn"} />}
        >
          <div className="block-summary">
            <div className="block-summary__top">
              <div>
                <p className="tiny-label">Height</p>
                <strong className="mono block-summary__height">{readString(latestBlock, "height", formatNumber(blockHeight))}</strong>
              </div>
              <div>
                <p className="tiny-label">Transactions</p>
                <strong className="mono">{readString(latestBlock, "transaction_count", readString(latestBlock, "transactions_len", "-"))}</strong>
              </div>
            </div>
            <div>
              <p className="tiny-label">Hash</p>
              <p className="mono muted-text block-summary__hash">{readString(latestBlock, "hash")}</p>
            </div>
            <div>
              <p className="tiny-label">State Root</p>
              <p className="mono muted-text block-summary__hash">{readString(latestBlock, "state_root")}</p>
            </div>
          </div>
        </Panel>
      </section>
    </div>
  );
}
