// app/lib/rpc.ts

import { asArray } from "../components/ExplorerUI";

// ตั้งค่า URL ให้ชี้ไปที่ RPC Server ของ Kanari (ค่าเริ่มต้น 127.0.0.1 พอร์ต 19001)
const DEFAULT_RPC_URL = "http://192.168.1.101:19001";
const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || DEFAULT_RPC_URL;
export const ACTIVE_RPC_STORAGE_KEY = "kanari-explorer-rpc-url";

/**
 * Returns the RPC selected by the visitor. This deliberately resolves at request
 * time (rather than module-load time) so every explorer page follows a newly
 * selected endpoint without needing a rebuild.
 */
export function getActiveRpcUrl(): string {
  if (typeof window === "undefined") {
    return RPC_URL;
  }

  const selected = window.localStorage.getItem(ACTIVE_RPC_STORAGE_KEY)?.trim();
  return selected || RPC_URL;
}

export function setActiveRpcUrl(rpcUrl: string): void {
  window.localStorage.setItem(ACTIVE_RPC_STORAGE_KEY, rpcUrl);
}

export const RPC_METHODS = {
  GET_ACCOUNT: "kanari_getOwner",
  GET_TOKEN_BALANCE: "kanari_getTokenBalance",
  LIST_TOKENS: "kanari_listTokens",
  GET_FUNGIBLE_ASSET: "kanari_getFungibleAsset",
  GET_FUNGIBLE_ASSET_HOLDERS: "kanari_getFungibleAssetHolders",
  GET_FUNGIBLE_ASSET_TRANSACTIONS: "kanari_getFungibleAssetTransactions",
  GET_ALL_BALANCES: "kanari_getOwnerBalances",
  GET_BLOCK: "kanari_getBlock",
  GET_FULL_BLOCK: "kanari_getFullBlock",
  GET_TRANSACTION: "kanari_getTransaction",
  GET_ALL_TRANSACTIONS: "kanari_getAllTransactions",
  GET_BLOCK_HEIGHT: "kanari_getBlockHeight",
  GET_STATS: "kanari_getStats",
  GET_SMT_STATUS: "kanari_getSmtStatus",
  SUBMIT_TRANSACTION: "kanari_submitObjectTransfer",
  HEALTH: "kanari_health",
  GET_NETWORK_STATUS: "kanari_getNetworkStatus",
  PUBLISH_MODULE: "kanari_publishModule",
  GET_MODULE: "kanari_getModule",
  LIST_MODULES: "kanari_listModules",
  VERIFY_MODULE: "kanari_verifyModule",
  CALL_FUNCTION: "kanari_callFunction",
  VIEW_FUNCTION: "kanari_viewFunction",
  GET_OBJECT: "kanari_getObject",
  GET_OBJECTS: "kanari_getObjects",
  GET_OBJECT_BY_REF: "kanari_getObjectByRef",
  GET_OWNED_OBJECTS: "kanari_getOwnedObjects",
  GET_OWNED_NFTS: "kanari_getOwnedNfts",
  LIST_COLLECTIONS: "kanari_listCollections",
  GET_NFTS_BY_COLLECTION: "kanari_getNftsByCollection",
} as const;

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
  stateRoot: string | null;
  totalTransactions: number | null;
  totalAccounts: number | null;
  pendingTransactions: number | null;
  latencyMs: number | null;
  error?: string;
};

export type SmtStatus = {
  height: number;
  checkpoint_state_root: string;
  enabled: boolean;
  persisted_root: string | null;
  effective_root: string;
  overlay_entries: number;
  overlay_updates: number;
  overlay_deletes: number;
  canonical_membership_changed: boolean;
  runtime_schema_version: number | null;
  expected_runtime_schema_version: number;
  wallet_supply_index_version: number | null;
  expected_wallet_supply_index_version: number;
  audit_requested: boolean;
  audit_performed: boolean;
  persisted_leaf_count: number | null;
  consistent: boolean | null;
  consistency_error: string | null;
};

export type RpcBytes = number[];

export type SignedTransactionPayload = {
  sender: string;
  recipient?: string | null;
  amount?: number | null;
  gas_limit: number;
  gas_price: number;
  nonce?: number;
  signature?: RpcBytes | null;
};

export type PublishModulePayload = {
  sender: string;
  module_bytes: RpcBytes;
  module_name: string;
  gas_limit: number;
  gas_price: number;
  nonce?: number;
  signature?: RpcBytes | null;
  execute_immediate?: boolean;
};

export type FunctionCallPayload = {
  package: string;
  module: string;
  function: string;
  type_args?: string[];
  args?: RpcBytes[];
};

export type CallFunctionPayload = FunctionCallPayload & {
  sender: string;
  gas_limit: number;
  gas_price: number;
  nonce?: number;
  signature?: RpcBytes | null;
  execute_immediate?: boolean;
};

type NoncePayload = {
  nonce?: number;
};

export type GetOwnedObjectsOptions = {
  object_type?: string | null;
};

export type RpcObjectOwnerKindFilter = "address" | "shared" | "immutable";

export type GetObjectsRequest = {
  owner?: string | null;
  owner_kind?: RpcObjectOwnerKindFilter | null;
  object_type?: string | null;
  min_version?: number | null;
  max_version?: number | null;
};

export type ObjectRefPayload = {
  object_id: string;
  version?: number | null;
  digest?: string | null;
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

/** Endpoints that can be selected without typing. Deployment configuration is
 * included too, so an operator-provided NEXT_PUBLIC_RPC_URL remains available. */
export const RPC_PRESETS: RpcEndpoint[] = [
  { name: "Local", url: "http://127.0.0.1:6767" },
  { name: "devnet", url: "http://192.168.1.101:19001" },
  { name: "testnet", url: "1" },
  { name: "mainnet", url: "2" },
  ...RPC_ENDPOINTS,
].filter((endpoint, index, endpoints) =>
  endpoints.findIndex((candidate) => candidate.url === endpoint.url) === index
);

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
  rpcUrl?: string
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
): Promise<any> {
  const targetRpcUrl = rpcUrl || getActiveRpcUrl();
  const body = {
    jsonrpc: "2.0",
    method,
    params,
    id: Date.now(),
  };

  try {
    const res = await fetch(targetRpcUrl, {
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
      console.warn(`[RPC Alert] Method: ${method} failed.`, data.error.message);
      throw new Error(`[${method}] ${data.error.message || "Unknown RPC Error"}`);
    }

    return data.result ?? data;
  } catch (err) {
    console.error(`[RPC Error] ${method}:`, err);
    throw err;
  }
}
export async function getTokenBalance(address: string, token_type: string) {
  return callRpc(RPC_METHODS.GET_TOKEN_BALANCE, { owner: address, token_type });
}

export async function getAllBalances(address: string) {
  const resp = await callRpc(RPC_METHODS.GET_ALL_BALANCES, { owner: address });
  return asBalanceArray(resp);
}

export async function getAccount(address: string) {
  return normalizeAccount(address, await callRpc(RPC_METHODS.GET_ACCOUNT, address));
}

export async function getTokens() {
  return callRpc(RPC_METHODS.LIST_TOKENS, []);
}

export async function getFungibleAsset(token_type: string) {
  return callRpc(RPC_METHODS.GET_FUNGIBLE_ASSET, { token_type });
}

export async function getFungibleAssetHolders(token_type: string, limit: number = 100) {
  const response = await callRpc(RPC_METHODS.GET_FUNGIBLE_ASSET_HOLDERS, { token_type, limit });
  const holders = readField(response, "holders");
  return Array.isArray(holders) ? holders : asArray(response);
}

export async function getFungibleAssetTransactions(token_type: string, limit: number = 50, owner?: string) {
  const response = await callRpc(RPC_METHODS.GET_FUNGIBLE_ASSET_TRANSACTIONS, { token_type, limit, owner: owner || null });
  const transactions = readField(response, "transactions");
  if (Array.isArray(transactions)) return dedupeTransactions(transactions.map(normalizeTransaction));
  return asArray(response).map(normalizeTransaction);
}

export async function getStats(rpcUrl?: string) {
  return callRpc(RPC_METHODS.GET_STATS, [], rpcUrl);
}

export async function getSmtStatus(audit = false, rpcUrl?: string): Promise<SmtStatus> {
  return callRpc(RPC_METHODS.GET_SMT_STATUS, audit ? { audit: true } : {}, rpcUrl);
}

export async function getHealth(rpcUrl?: string) {
  return callRpc(RPC_METHODS.HEALTH, [], rpcUrl);
}

export async function getNetworkStatus(rpcUrl?: string): Promise<NetworkStatus> {
  return callRpc(RPC_METHODS.GET_NETWORK_STATUS, [], rpcUrl);
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
      stateRoot: String(readField(stats, "state_root") ?? readField(stats, "stateRoot") ?? "") || null,
      totalTransactions: Number(readField(stats, "total_transactions") ?? 0),
      totalAccounts: Number(readField(stats, "total_accounts") ?? readField(stats, "total_owners") ?? 0),
      pendingTransactions: Number(readField(stats, "pending_transactions") ?? 0),
      latencyMs: Math.round(performance.now() - startedAt),
    };
  } catch (err) {
    return {
      endpoint,
      online: false,
      status: "offline",
      height: null,
      stateRoot: null,
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
    return await callRpc(RPC_METHODS.GET_BLOCK_HEIGHT, []);
  } catch {
    console.warn("getBlockHeight not available yet, returning null.");
    return null;
  }
}

export async function getBlock(height: number, rpcUrl?: string) {
  return normalizeBlock(await callRpc(RPC_METHODS.GET_BLOCK, height, rpcUrl));
}

export async function getFullBlock(height: number, rpcUrl?: string) {
  return normalizeBlock(await callRpc(RPC_METHODS.GET_FULL_BLOCK, height, rpcUrl));
}

function readField(value: unknown, field: string): unknown {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  return (value as Record<string, unknown>)[field];
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function asBalanceArray(value: unknown) {
  if (Array.isArray(value)) return value;
  const record = asRecord(value);
  const account = asRecord(record.account);
  if (Array.isArray(account.balances)) return account.balances;
  if (account.balances && typeof account.balances === "object") {
    return Object.entries(account.balances as Record<string, unknown>).map(([token_type, balance]) => ({
      balance: typeof balance === "number" || typeof balance === "string" ? balance : 0,
      amount: typeof balance === "number" || typeof balance === "string" ? balance : 0,
      decimals: 9,
      symbol: token_type.split("::").slice(-1)[0] || token_type,
      token_type,
    }));
  }

  if (Array.isArray(record.balances)) return record.balances;

  if (record.balances && typeof record.balances === "object") {
    return Object.entries(record.balances as Record<string, unknown>).map(([token_type, balance]) => ({
      balance: typeof balance === "number" || typeof balance === "string" ? balance : 0,
      amount: typeof balance === "number" || typeof balance === "string" ? balance : 0,
      decimals: 9,
      symbol: token_type.split("::").slice(-1)[0] || token_type,
      token_type,
    }));
  }

  return [];
}

function normalizeAccount(address: string, value: unknown) {
  const record = asRecord(value);
  const account = asRecord(record.account);
  const source = Object.keys(account).length > 0 ? account : record;
  const normalizedNonce = source.nonce ?? record.nonce ?? 0;
  return {
    ...record,
    ...source,
    nonce: normalizedNonce,
    balances: record.balances ?? source.balances ?? [],
    owned_objects: asObjectArray(source.owned_objects ?? record.owned_objects),
    owned_object_count: source.owned_object_count ?? record.owned_object_count ?? asObjectArray(source.owned_objects ?? record.owned_objects).length,
    address: String(source.owner ?? source.address ?? record.owner ?? record.address ?? address),
  };
}

function asObjectArray(value: unknown): unknown[] {
  if (Array.isArray(value)) return value.map(normalizeObjectInfo);
  const record = asRecord(value);
  if (Array.isArray(record.objects)) return record.objects.map(normalizeObjectInfo);
  if (Array.isArray(record.owned_objects)) return record.owned_objects.map(normalizeObjectInfo);
  const account = asRecord(record.account);
  if (Array.isArray(account.owned_objects)) return account.owned_objects.map(normalizeObjectInfo);
  return [];
}

function normalizeObjectInfo(value: unknown) {
  const record = asRecord(value);
  const id = String(record.id ?? record.object_id ?? record.objectId ?? "");
  const type = String(record.type_ ?? record.type ?? record.object_type ?? record.objectType ?? "");
  return {
    ...record,
    id: id || record.id,
    object_id: id || record.object_id,
    type_: type || record.type_,
    object_type: type || record.object_type,
    data: Array.isArray(record.data) ? record.data : [],
  };
}

function normalizeTransaction(value: unknown) {
  const record = asRecord(value);
  const status = String(record.status ?? "");
  const normalizedStatus = status.toLowerCase();
  const normalizedNonce = record.nonce ?? null;
  const success =
    typeof record.success === "boolean"
      ? record.success
      : ["success", "executed", "committed", "simulated_pending", "pending"].includes(normalizedStatus);
  const previewed = typeof record.previewed === "boolean" ? record.previewed : normalizedStatus.includes("preview");
  const submitted =
    typeof record.submitted === "boolean"
      ? record.submitted
      : previewed || normalizedStatus === "pending" || normalizedStatus === "submitted" || normalizedStatus === "executed";
  const committed =
    typeof record.committed === "boolean"
      ? record.committed
      : normalizedStatus === "committed" || normalizedStatus === "success";
  return {
    ...record,
    nonce: normalizedNonce,
    success,
    previewed,
    submitted,
    committed,
    checkpoint_height: record.block_height ?? record.checkpoint_height ?? null,
    block_height: record.block_height ?? record.checkpoint_height ?? null,
    sender_address: record.sender_address ?? record.sender ?? null,
    object_inputs: Array.isArray(record.object_inputs) ? record.object_inputs : [],
    gas_payment: record.gas_payment ?? null,
    effects: record.effects ?? null,
  };
}

function normalizeBlock(value: unknown) {
  const record = asRecord(value);
  return {
    ...record,
    checkpoint_height: record.height ?? record.block_height ?? null,
    transaction_effects: Array.isArray(record.transaction_effects) ? record.transaction_effects : [],
    object_changes: Array.isArray(record.object_changes) ? record.object_changes : [],
    object_graph_edges: Array.isArray(record.object_graph_edges) ? record.object_graph_edges : [],
  };
}

function dedupeTransactions<T>(transactions: T[]): T[] {
  const seen = new Set<string>();
  return transactions.filter((transaction) => {
    const hash = readField(transaction, "hash");
    if (typeof hash !== "string") return true;

    const normalized = hash.toLowerCase();
    if (seen.has(normalized)) return false;
    seen.add(normalized);
    return true;
  });
}

// ดึงประวัติธุรกรรมทั้งหมด (รองรับ Limit และการกรองด้วย Account)
export async function getAllTransactions(limit: number = 50, account?: string) {
  const params: { limit: number; owner?: string } = { limit };
  if (account) params.owner = account;
  const response = await callRpc(RPC_METHODS.GET_ALL_TRANSACTIONS, params);
  if (Array.isArray(response)) return dedupeTransactions(response.map(normalizeTransaction));

  const result = readField(response, "result");
  if (Array.isArray(result)) return dedupeTransactions(result.map(normalizeTransaction));

  const transactions = readField(response, "transactions");
  if (Array.isArray(transactions)) return dedupeTransactions(transactions.map(normalizeTransaction));

  const data = readField(response, "data");
  if (Array.isArray(data)) return dedupeTransactions(data.map(normalizeTransaction));

  return normalizeTransaction(response);
}

// ค้นหาธุรกรรมแบบเจาะจงด้วย Hash
export async function getTransaction(hash: string) {
  return normalizeTransaction(await callRpc(RPC_METHODS.GET_TRANSACTION, { hash }));
}

export async function submitTransaction(transaction: SignedTransactionPayload) {
  return callRpc(RPC_METHODS.SUBMIT_TRANSACTION, withCanonicalNonce(transaction));
}

export async function publishModule(payload: PublishModulePayload) {
  return callRpc(RPC_METHODS.PUBLISH_MODULE, withCanonicalNonce(payload));
}

export async function getModule(address: string, name: string) {
  return callRpc(RPC_METHODS.GET_MODULE, { address, name });
}

export async function listModules() {
  return callRpc(RPC_METHODS.LIST_MODULES, []);
}

export async function verifyModule(module_bytes: RpcBytes) {
  return callRpc(RPC_METHODS.VERIFY_MODULE, { module_bytes });
}

export async function callFunction(payload: CallFunctionPayload) {
  return callRpc(RPC_METHODS.CALL_FUNCTION, {
    ...withCanonicalNonce(payload),
    args: payload.args ?? [],
    type_args: payload.type_args ?? [],
  });
}

export async function viewFunction(payload: FunctionCallPayload) {
  return callRpc(RPC_METHODS.VIEW_FUNCTION, [
    {
      ...payload,
      args: payload.args ?? [],
      type_args: payload.type_args ?? [],
    },
  ]);
}

export async function getObject(object_id: string) {
  return callRpc(RPC_METHODS.GET_OBJECT, { object_id });
}

export async function getObjectByRef(object_ref: ObjectRefPayload) {
  return callRpc(RPC_METHODS.GET_OBJECT_BY_REF, { object_ref });
}

export async function getObjects(request: GetObjectsRequest = {}) {
  const response = await callRpc(RPC_METHODS.GET_OBJECTS, {
    owner: request.owner ?? null,
    owner_kind: request.owner_kind ?? null,
    object_type: request.object_type ?? null,
    min_version: request.min_version ?? null,
    max_version: request.max_version ?? null,
  });
  return asObjectArray(response);
}

export async function getOwnedObjects(owner: string, options: GetOwnedObjectsOptions = {}) {
  const response = await callRpc(RPC_METHODS.GET_OWNED_OBJECTS, {
    owner,
    object_type: options.object_type ?? null,
  });
  return asObjectArray(response);
}

export async function getOwnedNfts(address: string) {
  return callRpc(RPC_METHODS.GET_OWNED_NFTS, address);
}

export async function getCollections() {
  return callRpc(RPC_METHODS.LIST_COLLECTIONS, []);
}

export async function getNftsByCollection(collectionId: string) {
  return callRpc(RPC_METHODS.GET_NFTS_BY_COLLECTION, collectionId);
}

function withCanonicalNonce<T extends NoncePayload>(payload: T): T {
  const nonce = payload.nonce;
  if (typeof nonce !== "number") {
    return payload;
  }

  return {
    ...payload,
    nonce,
  };
}
