"use client";

import { useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import {
  getBlockHeight,
  deriveAuthorityRpcEndpoints,
  getNodeHealth,
  getNetworkStatus,
  getTokens,
  RPC_ENDPOINTS,
  type RpcEndpoint,
  type NodeHealth,
} from "./lib/rpc";

function formatNumber(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) return "-";
  return value.toLocaleString();
}

function shortUrl(url: string) {
  return url.replace(/^https?:\/\//, "").replace(/\/$/, "");
}

function readField(value: unknown, field: string): unknown {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  return (value as Record<string, unknown>)[field];
}

function StatCard({
  label,
  value,
  sub,
  tone = "emerald",
}: {
  label: string;
  value: string;
  sub: string;
  tone?: "emerald" | "cyan" | "amber" | "rose";
}) {
  const toneClass = {
    emerald: "text-emerald-300 bg-emerald-400/10 border-emerald-400/20",
    cyan: "text-cyan-300 bg-cyan-400/10 border-cyan-400/20",
    amber: "text-amber-300 bg-amber-400/10 border-amber-400/20",
    rose: "text-rose-300 bg-rose-400/10 border-rose-400/20",
  }[tone];

  return (
    <div className="rounded-lg border border-white/10 bg-[#111113] p-5 shadow-lg shadow-black/20">
      <div className={`mb-5 inline-flex h-9 w-9 items-center justify-center rounded-md border ${toneClass}`}>
        <span className="h-2.5 w-2.5 rounded-full bg-current" />
      </div>
      <div className="text-xs font-semibold uppercase tracking-widest text-zinc-500">{label}</div>
      <div className="mt-2 font-mono text-3xl font-bold text-white">{value}</div>
      <div className="mt-2 text-sm text-zinc-400">{sub}</div>
    </div>
  );
}

export default function Home() {
  const [search, setSearch] = useState("");
  const [tokenCount, setTokenCount] = useState<number | null>(null);
  const [blockHeight, setBlockHeight] = useState<number | null>(null);
  const [configuredEndpoints, setConfiguredEndpoints] = useState<RpcEndpoint[]>(RPC_ENDPOINTS);
  const [nodes, setNodes] = useState<NodeHealth[]>([]);
  const [lastUpdated, setLastUpdated] = useState<string>("");
  const router = useRouter();

  const onlineNodes = nodes.filter((node) => node.online);
  const offlineNodes = nodes.length - onlineNodes.length;
  const maxHeight = Math.max(0, ...nodes.map((node) => node.height ?? 0));
  const syncedNodes = onlineNodes.filter((node) => (node.height ?? 0) >= maxHeight).length;
  const laggingNodes = onlineNodes.length - syncedNodes;
  const totalTransactions = nodes.find((node) => node.totalTransactions !== null)?.totalTransactions ?? null;
  const totalAccounts = nodes.find((node) => node.totalAccounts !== null)?.totalAccounts ?? null;
  const pendingTransactions = nodes.reduce((sum, node) => sum + (node.pendingTransactions ?? 0), 0);
  const networkOnline = onlineNodes.length > 0;

  const networkStatusLabel = useMemo(() => {
    if (nodes.length === 0) return "Loading";
    if (onlineNodes.length === nodes.length && laggingNodes === 0) return "All nodes synced";
    if (onlineNodes.length === nodes.length) return "Nodes syncing";
    if (onlineNodes.length > 0) return "Partial outage";
    return "Offline";
  }, [nodes.length, onlineNodes.length, laggingNodes]);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (search.trim()) router.push(`/account?address=${search.trim()}`);
  };

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

      if (tokensRes) {
        if (Array.isArray(tokensRes)) setTokenCount(tokensRes.length);
        else {
          const result = readField(tokensRes, "result");
          if (Array.isArray(result)) setTokenCount(result.length);
        }
      }

      if (heightRes !== null && heightRes !== undefined) {
        const height = typeof heightRes === "object" ? readField(heightRes, "height") : heightRes;
        setBlockHeight(Number(height));
      }

      setConfiguredEndpoints(nextEndpoints);
      setNodes(nodeResults);
      setLastUpdated(new Date().toLocaleTimeString());
    }

    fetchNetworkData();
    const interval = setInterval(fetchNetworkData, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex w-full flex-col">
      <section className="w-full border-b border-white/10 px-6 py-10">
        <div className="mx-auto grid w-full max-w-7xl gap-8 lg:grid-cols-[minmax(0,1.1fr)_420px] lg:items-end">
          <div>
            <div className="mb-5 inline-flex items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] px-3 py-2 text-sm text-zinc-300">
              <span className={`h-2.5 w-2.5 rounded-full ${networkOnline ? "bg-emerald-400" : "bg-rose-400"}`} />
              {networkStatusLabel}
            </div>
            <h1 className="text-4xl font-black tracking-normal text-white md:text-6xl">
              Kanari Explorer
            </h1>
            <p className="mt-4 max-w-2xl text-base leading-7 text-zinc-400 md:text-lg">
              Network overview, node status, blocks, transactions, tokens, and accounts in one place.
            </p>
          </div>

          <form onSubmit={handleSearch} className="w-full">
            <div className="rounded-lg border border-white/10 bg-[#111113] p-2 shadow-2xl shadow-black/30 focus-within:border-emerald-400/60">
              <div className="flex items-center gap-2">
                <svg className="ml-3 h-5 w-5 shrink-0 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="m21 21-6-6m2-5a7 7 0 1 1-14 0 7 7 0 0 1 14 0Z" />
                </svg>
                <input
                  type="text"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Address, transaction hash, or module"
                  className="min-w-0 flex-1 bg-transparent px-2 py-3 font-mono text-sm text-white outline-none placeholder:text-zinc-600 md:text-base"
                />
                <button type="submit" className="rounded-md bg-white px-5 py-3 text-sm font-bold text-black transition-colors hover:bg-zinc-200">
                  Search
                </button>
              </div>
            </div>
          </form>
        </div>
      </section>

      <section className="mx-auto w-full max-w-7xl px-6 py-8">
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
          <StatCard
            label="Nodes"
            value={`${onlineNodes.length}/${nodes.length || configuredEndpoints.length}`}
            sub={offlineNodes > 0 ? `${offlineNodes} offline` : `${laggingNodes} lagging`}
          />
          <StatCard label="Block Height" value={formatNumber(maxHeight || blockHeight)} sub="Highest reported height" tone="cyan" />
          <StatCard label="Transactions" value={formatNumber(totalTransactions)} sub={`${formatNumber(pendingTransactions)} pending`} tone="amber" />
          <StatCard label="Tokens" value={formatNumber(tokenCount)} sub={`${formatNumber(totalAccounts)} accounts`} tone="rose" />
        </div>
      </section>

      <section className="mx-auto grid w-full max-w-7xl gap-6 px-6 pb-12 lg:grid-cols-[minmax(0,1fr)_360px]">
        <div className="rounded-lg border border-white/10 bg-[#111113] shadow-lg shadow-black/20">
          <div className="flex flex-col gap-3 border-b border-white/10 p-5 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="text-lg font-bold text-white">Node Status</h2>
              <p className="mt-1 text-sm text-zinc-500">Updated every 5 seconds{lastUpdated ? `, last ${lastUpdated}` : ""}</p>
            </div>
            <div className="rounded-md border border-white/10 px-3 py-2 text-sm text-zinc-300">
              {syncedNodes} synced / {nodes.length || configuredEndpoints.length} configured
            </div>
          </div>

          <div className="divide-y divide-white/10">
            {(nodes.length > 0 ? nodes : configuredEndpoints.map((endpoint): NodeHealth => ({
              endpoint,
              online: false,
              status: "loading",
              height: null,
              totalTransactions: null,
              totalAccounts: null,
              pendingTransactions: null,
              latencyMs: null,
              error: undefined,
            }))).map((node) => (
              <div key={node.endpoint.url} className="grid gap-4 p-5 md:grid-cols-[minmax(0,1fr)_120px_120px_120px] md:items-center">
                <div className="min-w-0">
                  <div className="flex items-center gap-3">
                    <span className={`h-2.5 w-2.5 rounded-full ${node.online ? "bg-emerald-400" : node.status === "loading" ? "bg-zinc-500" : "bg-rose-400"}`} />
                    <div className="truncate font-semibold text-white">{node.endpoint.name}</div>
                  </div>
                  <div className="mt-1 truncate font-mono text-xs text-zinc-500">{shortUrl(node.endpoint.url)}</div>
                  {node.error ? <div className="mt-2 text-xs text-rose-300">{node.error}</div> : null}
                </div>
                <div>
                  <div className="text-xs uppercase tracking-widest text-zinc-600">Status</div>
                  <div className={`mt-1 text-sm font-semibold ${!node.online
                      ? "text-rose-300"
                      : (node.height ?? 0) < maxHeight
                        ? "text-amber-300"
                        : "text-emerald-300"
                    }`}>
                    {!node.online ? node.status : (node.height ?? 0) < maxHeight ? `lagging ${maxHeight - (node.height ?? 0)}` : "synced"}
                  </div>
                </div>
                <div>
                  <div className="text-xs uppercase tracking-widest text-zinc-600">Height</div>
                  <div className="mt-1 font-mono text-sm text-zinc-200">{formatNumber(node.height)}</div>
                </div>
                <div>
                  <div className="text-xs uppercase tracking-widest text-zinc-600">Latency</div>
                  <div className="mt-1 font-mono text-sm text-zinc-200">{node.latencyMs === null ? "-" : `${node.latencyMs} ms`}</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        <aside className="rounded-lg border border-white/10 bg-[#111113] p-5 shadow-lg shadow-black/20">
          <h2 className="text-lg font-bold text-white">Network Summary</h2>
          <div className="mt-5 space-y-4">
            <div className="flex items-center justify-between gap-4">
              <span className="text-sm text-zinc-500">RPC endpoints</span>
              <span className="font-mono text-sm text-white">{configuredEndpoints.length}</span>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-sm text-zinc-500">Online nodes</span>
              <span className="font-mono text-sm text-emerald-300">{onlineNodes.length}</span>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-sm text-zinc-500">Synced nodes</span>
              <span className="font-mono text-sm text-emerald-300">{syncedNodes}</span>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-sm text-zinc-500">Lagging nodes</span>
              <span className="font-mono text-sm text-amber-300">{laggingNodes}</span>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-sm text-zinc-500">Offline nodes</span>
              <span className="font-mono text-sm text-rose-300">{offlineNodes}</span>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-sm text-zinc-500">Highest height</span>
              <span className="font-mono text-sm text-white">{formatNumber(maxHeight || blockHeight)}</span>
            </div>
          </div>
          <div className="mt-6 border-t border-white/10 pt-5">
            <Link href="/tx" className="inline-flex items-center text-sm font-semibold text-emerald-300 hover:text-emerald-200">
              View recent activity
              <span className="ml-2" aria-hidden="true">-&gt;</span>
            </Link>
          </div>
        </aside>
      </section>
    </div>
  );
}
