"use client";

import Link from "next/link";
import { Suspense, useEffect, useState } from "react";
import {
  EmptyState,
  PageHeader,
  StatusPill,
  formatBalance,
  readString,
} from "../components/ExplorerUI";
import { RPC_METHODS, callRpc, getTokenBalance } from "../lib/rpc";

export const USD_COIN_TYPE = "0x2::usd::USD";

type FaucetStatus = {
  enabled: boolean;
  usd_type: string;
  usd_decimals: number;
  default_drip_usd: string;
  max_per_request_usd: string;
  cooldown_secs: number;
};

type ClaimResult = {
  hash: string;
  status: string;
  leg_hashes?: string[];
  amount_usd: string;
  recipient: string;
};

function isValidAddress(value: string): boolean {
  const trimmed = value.trim().toLowerCase();
  return (
    trimmed.startsWith("0x") &&
    trimmed.length === 66 &&
    /^[0-9a-f]+$/.test(trimmed.slice(2))
  );
}

function formatCooldown(totalSecs: number): string {
  if (!Number.isFinite(totalSecs) || totalSecs < 0) return "";
  if (totalSecs % 3600 === 0) return `${totalSecs / 3600}h`;
  if (totalSecs % 60 === 0) return `${totalSecs / 60}m`;
  return `${totalSecs}s`;
}

function FaucetContent() {
  const [address, setAddress] = useState("");
  const [service, setService] = useState<FaucetStatus | null>(null);
  const [serviceError, setServiceError] = useState("");
  const [claiming, setClaiming] = useState(false);
  const [result, setResult] = useState<ClaimResult | null>(null);
  const [claimError, setClaimError] = useState("");
  const [balance, setBalance] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function loadStatus() {
      try {
        const data = (await callRpc(
          RPC_METHODS.GET_USD_FAUCET_STATUS,
          {},
        )) as FaucetStatus;
        if (!cancelled) {
          setService(data);
          if (!data.enabled) {
            setServiceError(
              "USD faucet is disabled on this node. The operator must set FAUCET_SPONSOR_KEY (a gas-paying key, funded once with KANARI) and restart the node.",
            );
          } else {
            setServiceError("");
          }
        }
      } catch {
        if (!cancelled) {
          setService(null);
          setServiceError(
            "Faucet status unavailable. Is the node RPC reachable?",
          );
        }
      }
    }
    void loadStatus();
    return () => {
      cancelled = true;
    };
  }, []);

  async function refreshBalance(target: string) {
    try {
      const response = await getTokenBalance(target.trim(), USD_COIN_TYPE);
      const value =
        readString(response, "balance", readString(response, "amount", "")) ||
        (typeof response === "number" || typeof response === "string"
          ? String(response)
          : "");
      setBalance(value || null);
    } catch {
      setBalance(null);
    }
  }

  async function handleClaim(event: React.FormEvent) {
    event.preventDefault();
    setClaimError("");
    setResult(null);
    const target = address.trim();
    if (!isValidAddress(target)) {
      setClaimError("Enter a 0x-prefixed 64-hex-character address.");
      return;
    }
    setClaiming(true);
    try {
      const data = (await callRpc(RPC_METHODS.REQUEST_USD_FAUCET, {
        address: target,
      })) as ClaimResult;
      setResult(data);
      await refreshBalance(target);
    } catch (error) {
      setClaimError(
        error instanceof Error ? error.message : "Faucet request failed.",
      );
    } finally {
      setClaiming(false);
    }
  }

  const online = service?.enabled === true;
  const dripLabel = service?.default_drip_usd
    ? `${service.default_drip_usd} USD`
    : "USD";
  const cooldownLabel = service ? formatCooldown(service.cooldown_secs) : "";

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="Devnet Faucet"
        title="USD"
        accent="Faucet."
        description={`Enter a wallet address to receive ${dripLabel} as two USD coin objects — no key, no password. Each claim mints fresh coins straight to the address.`}
      />

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">Claim {dripLabel}</h2>
            <p className="panel-subtitle">
              {service?.enabled
                ? `Fixed ${dripLabel} in 2 coin objects · cooldown ${cooldownLabel} per address & IP`
                : "Checking node faucet..."}
            </p>
          </div>
          <StatusPill
            label={service ? (online ? "Online" : "Disabled") : "Offline"}
            state={service ? (online ? "ok" : "warn") : "down"}
          />
        </div>
        {serviceError ? (
          <EmptyState label={serviceError} />
        ) : (
          <form onSubmit={handleClaim} className="data-list">
            <div className="data-row data-row--account">
              <div>
                <p className="tiny-label">Wallet address</p>
                <div className="search-box">
                  <input
                    value={address}
                    onChange={(event) => setAddress(event.target.value)}
                    placeholder="0x..."
                    spellCheck={false}
                    autoComplete="off"
                  />
                </div>
              </div>
              <div>
                <p className="tiny-label">You receive</p>
                <span className="mono">{dripLabel} in 2 coin objects</span>
              </div>
            </div>
            <div className="data-row data-row--account">
              <div>
                <button
                  className="button button--dark"
                  type="submit"
                  disabled={claiming || !online}
                >
                  {claiming ? "Claiming..." : "Claim USD"}
                </button>
              </div>
              <div>
                <p className="tiny-label">USD balance</p>
                <span className="mono">
                  {balance === null
                    ? "-"
                    : `${formatBalance(balance, "6")} USD`}
                </span>
              </div>
            </div>
          </form>
        )}
        {claimError ? (
          <div className="data-row">
            <div>
              <p className="tiny-label">Error</p>
              <StatusPill label={claimError} state="down" />
            </div>
          </div>
        ) : null}
        {result ? (
          <div className="data-row data-row--tokens">
            <div className="primary-text">
              <div>
                <p className="tiny-label">Claimed</p>
                <strong className="mono">
                  {result.amount_usd} USD →{" "}
                  <Link
                    className="text-link"
                    href={`/account?address=${encodeURIComponent(result.recipient)}`}
                  >
                    {result.recipient.slice(0, 10)}...
                    {result.recipient.slice(-8)}
                  </Link>
                </strong>
              </div>
              <div className="muted-text mono">
                {(result.leg_hashes ?? [result.hash]).map((hash) => (
                  <span key={hash}>
                    <Link
                      className="text-link"
                      href={`/tx?hash=${encodeURIComponent(hash)}`}
                    >
                      tx {hash.slice(0, 16)}...
                    </Link>{" "}
                  </span>
                ))}
                · {result.status}
              </div>
            </div>
            <div>
              <p className="tiny-label">Status</p>
              <StatusPill label="Claimed" state="ok" />
            </div>
            <div>
              <p className="tiny-label">Next step</p>
              <Link className="text-link" href="/coins">
                View tokens
              </Link>
            </div>
          </div>
        ) : null}
      </section>
    </div>
  );
}

export default function FaucetPage() {
  return (
    <Suspense fallback={<EmptyState loading label="Loading faucet..." />}>
      <FaucetContent />
    </Suspense>
  );
}
