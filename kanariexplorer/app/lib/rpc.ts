export async function callRpc(method: string, params: any) {
  const url = (process.env.NEXT_PUBLIC_RPC_URL as string) || "http://127.0.0.1:19001";
  const body = {
    jsonrpc: "2.0",
    method,
    params,
    id: 1,
  };

  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const txt = await res.text();
    throw new Error(`RPC request failed: ${res.status} ${txt}`);
  }

  return res.json();
}

export async function getBalance(address: string) {
  const resp = await callRpc("kanari_getBalance", address);
  return resp?.result ?? resp;
}

export async function getTokenBalance(address: string, token_type: string) {
  const resp = await callRpc("kanari_getTokenBalance", { address, token_type });
  return resp?.result ?? resp;
}

export async function getAllBalances(address: string) {
  const resp = await callRpc("kanari_getAllBalances", { address });
  // server returns { address, balances: [...] }
  return resp?.result?.balances ?? resp?.result ?? resp;
}

export async function getAccount(address: string) {
  const resp = await callRpc("kanari_getAccount", address);
  return resp?.result ?? resp;
}

export async function getTokens() {
  const resp = await callRpc("kanari_listTokens", []);
  return resp?.result ?? resp;
}

