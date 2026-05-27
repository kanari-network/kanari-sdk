// app/lib/rpc.ts

// ตั้งค่า URL ให้ชี้ไปที่ RPC Server ของ Kanari (ค่าเริ่มต้น 127.0.0.1 พอร์ต 19001)
const DEFAULT_RPC_URL = "http://192.168.1.102:19001";
const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || DEFAULT_RPC_URL;

export type RpcEndpoint = {
  name: string;
  url: string;
};

export type NetworkAuthorityStatus = {
  authority_id: string;
  local: boolean;
};

export type NetworkStatus = {
  local_authority_id: string;
  authority_count: number;
  authorities: NetworkAuthorityStatus[];
};

export type NodeHealth = {
  endpoint: RpcEndpoint;
  online: boolean;
  status: string;
  height: number | null;
  totalTransactions: number | null;
  totalAccounts: number | null;
  pendingTransactions: number | null;
  latencyMs: number | null;
  error?: string;
};

function parseRpcEndpoints(): RpcEndpoint[] {
  const configured = process.env.NEXT_PUBLIC_RPC_ENDPOINTS;
  const endpoints = configured
    ? configured
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean)
      .map((item, index) => {
        const separatorIndex = item.indexOf("=");
        if (separatorIndex > 0) {
          return {
            name: item.slice(0, separatorIndex).trim(),
            url: item.slice(separatorIndex + 1).trim(),
          };
        }
        return { name: `Node ${index + 1}`, url: item };
      })
    : [{ name: "Node 1", url: RPC_URL }];

  return endpoints.filter((endpoint) => endpoint.url.length > 0);
}

export const RPC_ENDPOINTS = parseRpcEndpoints();

function replacePort(url: string, port: number): string {
  try {
    const parsed = new URL(url);
    parsed.port = String(port);
    return parsed.toString().replace(/\/$/, "");
  } catch {
    return url;
  }
}

export function deriveAuthorityRpcEndpoints(
  baseUrl: string,
  networkStatus: NetworkStatus
): RpcEndpoint[] {
  const basePort = (() => {
    try {
      const parsed = new URL(baseUrl);
      return Number(parsed.port || (parsed.protocol === "https:" ? 443 : 80));
    } catch {
      return 19001;
    }
  })();

  return networkStatus.authorities.map((authority, index) => ({
    name: `Node ${index + 1} (${authority.authority_id})`,
    url: replacePort(baseUrl, basePort + index * 10),
  }));
}

// Explorer pages currently consume mixed JSON-RPC shapes directly.
export async function callRpc(
  method: string,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  params: any = [],
  rpcUrl: string = RPC_URL
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
): Promise<any> {
  const body = {
    jsonrpc: "2.0",
    method,
    params,
    id: Date.now(),
  };

  try {
    const res = await fetch(rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!res.ok) {
      throw new Error(`RPC HTTP Error: ${res.status}`);
    }

    const data = await res.json();

    // 🚨 FIX 1: ดักจับ Error ให้พิมพ์ชื่อ Method ออกมาด้วย จะได้รู้ว่าตัวไหนพัง
    if (data.error) {
      console.warn(`⚠️ [RPC Alert] Method: ${method} failed.`, data.error.message);
      throw new Error(`[${method}] ${data.error.message || "Unknown RPC Error"}`);
    }

    return data.result ?? data;
  } catch (err) {
    console.error(`[RPC Error] ${method}:`, err);
    throw err;
  }
}
export async function getTokenBalance(address: string, token_type: string) {
  return callRpc("kanari_getTokenBalance", { address, token_type });
}

export async function getAllBalances(address: string) {
  const resp = await callRpc("kanari_getAllBalances", { address });
  return resp?.balances ?? resp;
}

export async function getAccount(address: string) {
  return callRpc("kanari_getAccount", address);
}

export async function getTokens() {
  return callRpc("kanari_listTokens", {});
}

export async function getStats(rpcUrl?: string) {
  return callRpc("kanari_getStats", [], rpcUrl);
}

export async function getHealth(rpcUrl?: string) {
  return callRpc("kanari_health", [], rpcUrl);
}

export async function getNetworkStatus(rpcUrl?: string): Promise<NetworkStatus> {
  return callRpc("kanari_getNetworkStatus", [], rpcUrl);
}

export async function getNodeHealth(endpoint: RpcEndpoint): Promise<NodeHealth> {
  const startedAt = performance.now();
  try {
    const [stats, health] = await Promise.all([
      getStats(endpoint.url),
      getHealth(endpoint.url).catch(() => null),
    ]);
    return {
      endpoint,
      online: true,
      status: String(readField(health, "status") ?? "ok"),
      height: Number(readField(stats, "height") ?? 0),
      totalTransactions: Number(readField(stats, "total_transactions") ?? 0),
      totalAccounts: Number(readField(stats, "total_accounts") ?? 0),
      pendingTransactions: Number(readField(stats, "pending_transactions") ?? 0),
      latencyMs: Math.round(performance.now() - startedAt),
    };
  } catch (err) {
    return {
      endpoint,
      online: false,
      status: "offline",
      height: null,
      totalTransactions: null,
      totalAccounts: null,
      pendingTransactions: null,
      latencyMs: null,
      error: err instanceof Error ? err.message : "RPC unavailable",
    };
  }
}

// 🚨 FIX 2: ดักจับ Error ของ getBlockHeight แบบเงียบๆ ป้องกันหน้า Home พังหากฝั่ง Rust ยังไม่ได้ทำ API นี้ไว้
export async function getBlockHeight() {
  try {
    return await callRpc("kanari_getBlockHeight", []);
  } catch {
    console.warn("getBlockHeight not available yet, returning null.");
    return null;
  }
}

function readField(value: unknown, field: string): unknown {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  return (value as Record<string, unknown>)[field];
}

// ดึงประวัติธุรกรรมทั้งหมด (รองรับ Limit และการกรองด้วย Account)
export async function getAllTransactions(limit: number = 50, account?: string) {
  const params: { limit: number; account?: string } = { limit };
  if (account) params.account = account;
  return callRpc("kanari_getAllTransactions", params);
}

// ค้นหาธุรกรรมแบบเจาะจงด้วย Hash
export async function getTransaction(hash: string) {
  return callRpc("kanari_getTransaction", { hash });
}


export async function getOwnedNfts(address: string) {
  return callRpc("kanari_getOwnedNfts", address);
}

export async function getCollections() {
  return callRpc("kanari_listCollections", []);
}

export async function getNftsByCollection(collectionId: string) {
  return callRpc("kanari_getNftsByCollection", collectionId);
}
