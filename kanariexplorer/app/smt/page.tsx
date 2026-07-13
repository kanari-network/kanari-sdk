"use client";

import { useEffect, useState } from "react";
import {
  CopyButton,
  EmptyState,
  PageHeader,
  Panel,
  RawDetails,
  StatCard,
  StatusPill,
  formatNumber,
} from "../components/ExplorerUI";
import { getSmtStatus, type SmtStatus } from "../lib/rpc";

function rootValue(value: string | null) {
  return value || "Not available";
}

function auditPill(status: SmtStatus | null, loading: boolean) {
  if (loading) return <StatusPill label="Loading" state="warn" />;
  if (!status) return <StatusPill label="Unavailable" state="down" />;
  if (!status.enabled) return <StatusPill label="SMT disabled" state="warn" />;
  if (!status.audit_performed) return <StatusPill label="Status only" state="warn" />;
  return <StatusPill label={status.consistent ? "Audit passed" : "Audit failed"} state={status.consistent ? "ok" : "down"} />;
}

export default function SmtStatusPage() {
  const [status, setStatus] = useState<SmtStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [lastUpdated, setLastUpdated] = useState("");

  async function loadStatus(audit = false) {
    setLoading(true);
    setError("");
    try {
      const nextStatus = await getSmtStatus(audit);
      setStatus(nextStatus);
      setLastUpdated(new Date().toLocaleTimeString());
    } catch (err) {
      setStatus(null);
      setError(err instanceof Error ? err.message : "Failed to read SMT status.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadStatus();
  }, []);

  const membershipState = status?.canonical_membership_changed ? "warn" : "ok";
  const schemaMatches =
    status !== null &&
    status.runtime_schema_version === status.expected_runtime_schema_version &&
    status.wallet_supply_index_version === status.expected_wallet_supply_index_version;

  return (
    <div className="explorer-wrap smt-page">
      <PageHeader
        eyebrow="State Integrity"
        title="Sparse Merkle"
        accent="Tree."
        description="Inspect the current incremental state tree without mutating node state. A full audit compares persisted SMT leaves against canonical state and can take longer on a large database."
      >
        <div className="smt-page__actions">
          <button className="button button--ghost" type="button" onClick={() => void loadStatus(false)} disabled={loading}>
            Refresh status
          </button>
          <button className="button button--light" type="button" onClick={() => void loadStatus(true)} disabled={loading}>
            Run full audit
          </button>
        </div>
      </PageHeader>

      {error ? <EmptyState label={`${error} Rebuild and restart the node if kanari_getSmtStatus is not available yet.`} /> : null}

      <section className="stat-grid explorer-stat-grid smt-page__stats">
        <StatCard label="SMT" value={status?.enabled ? "Enabled" : status ? "Disabled" : "-"} detail="Persistent sparse Merkle tree" />
        <StatCard label="Overlay" value={formatNumber(status?.overlay_entries)} detail={`${formatNumber(status?.overlay_updates)} updates / ${formatNumber(status?.overlay_deletes)} deletes`} />
        <StatCard label="Leaves" value={status?.persisted_leaf_count === null || status?.persisted_leaf_count === undefined ? "Audit required" : formatNumber(status.persisted_leaf_count)} detail="Persisted canonical leaves" />
        <StatCard label="Height" value={formatNumber(status?.height)} detail={lastUpdated ? `Updated ${lastUpdated}` : "Waiting for RPC"} />
      </section>

      <section className="content-grid smt-page__grid">
        <Panel
          title="SMT Audit"
          subtitle="Status is cheap. Full audit is explicitly requested and never repairs or rebuilds state."
          action={auditPill(status, loading)}
        >
          {loading && !status ? <EmptyState loading label="Loading sparse Merkle tree status..." /> : null}
          {!loading && status ? (
            <div className="block-summary">
              <div className="block-summary__top">
                <div>
                  <p className="tiny-label">Canonical membership</p>
                  <StatusPill label={status.canonical_membership_changed ? "Changing" : "Stable"} state={membershipState} />
                </div>
                <div>
                  <p className="tiny-label">Consistency</p>
                  <strong className="mono">{status.consistent === null ? "Not audited" : status.consistent ? "Consistent" : "Mismatch"}</strong>
                </div>
              </div>
              <div className="block-summary__top">
                <div>
                  <p className="tiny-label">Runtime schema</p>
                  <strong className="mono">{status.runtime_schema_version ?? "-"} / {status.expected_runtime_schema_version}</strong>
                </div>
                <div>
                  <p className="tiny-label">Wallet index schema</p>
                  <strong className="mono">{status.wallet_supply_index_version ?? "-"} / {status.expected_wallet_supply_index_version}</strong>
                </div>
              </div>
              <div>
                <p className="tiny-label">Schema state</p>
                <StatusPill label={schemaMatches ? "Current" : "Needs attention"} state={schemaMatches ? "ok" : "warn"} />
              </div>
              {status.consistency_error ? (
                <div>
                  <p className="tiny-label">Audit error</p>
                  <p className="mono muted-text break-anywhere">{status.consistency_error}</p>
                </div>
              ) : null}
            </div>
          ) : null}
        </Panel>

        <Panel
          title="State Roots"
          subtitle="Compare persisted and effective roots. A non-empty overlay can make the effective root differ before commit."
          action={<StatusPill label={status?.overlay_entries ? "Overlay pending" : "Committed view"} state={status?.overlay_entries ? "warn" : "ok"} />}
        >
          {status ? (
            <div className="smt-root-list">
              {[
                ["Persisted SMT root", status.persisted_root],
                ["Effective state root", status.effective_root],
                ["Checkpoint state root", status.checkpoint_state_root],
              ].map(([label, value]) => {
                const root = rootValue(value);
                return (
                  <div className="smt-root-row" key={label}>
                    <p className="tiny-label">{label}</p>
                    <span className="copy-row copy-row--wrap">
                      <span className="mono">{root}</span>
                      {value ? <CopyButton value={value} label={`Copy ${label}`} /> : null}
                    </span>
                  </div>
                );
              })}
            </div>
          ) : null}
        </Panel>
      </section>

      {status ? <RawDetails label="Raw SMT RPC response" value={status} /> : null}
    </div>
  );
}
