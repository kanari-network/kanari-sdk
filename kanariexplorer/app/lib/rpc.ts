// app/lib/rpc.ts

// ตั้งค่า URL ให้ชี้ไปที่ RPC Server ของ Kanari (ค่าเริ่มต้น 127.0.0.1 พอร์ต 19001)
const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || "http://192.168.1.103:19001";

export async function callRpc(method: string, params: any = []) {
  const body = {
    jsonrpc: "2.0",
    method,
    params,
    id: Date.now(),
  };

  try {
    const res = await fetch(RPC_URL, {
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
export async function getBalance(address: string) {
  return callRpc("kanari_getBalance", address);
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
  return callRpc("kanari_listTokens", []);
}

// 🚨 FIX 2: ดักจับ Error ของ getBlockHeight แบบเงียบๆ ป้องกันหน้า Home พังหากฝั่ง Rust ยังไม่ได้ทำ API นี้ไว้
export async function getBlockHeight() {
  try {
    return await callRpc("kanari_getBlockHeight", []);
  } catch (e) {
    console.warn("getBlockHeight not available yet, returning null.");
    return null;
  }
}

// ดึงประวัติธุรกรรมทั้งหมด (รองรับ Limit และการกรองด้วย Account)
export async function getAllTransactions(limit: number = 50, account?: string) {
  const params: any = { limit };
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