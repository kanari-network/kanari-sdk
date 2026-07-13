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

function shortRoot(value: string | null, head = 10, tail = 8) {
  if (!value) return "not available";
  return value.length > head + tail + 3 ? `${value.slice(0, head)}...${value.slice(-tail)}` : value;
}

type TreeNodeKind = "default" | "proof" | "stored" | "updated";

const TREE_X = [
  [450],
  [225, 675],
  [112, 337, 562, 787],
  [56, 169, 281, 394, 506, 619, 731, 844],
];
const TREE_Y = [38, 108, 182, 262];

function nodeKind(status: SmtStatus | null, depth: number, index: number): TreeNodeKind {
  if (depth === 0) return "updated";
  if (status?.consistent === false && (depth === 2 || index === 6)) return "proof";
  if (status?.overlay_deletes && (index === 1 || index === 6)) return "proof";
  if (status?.overlay_updates && (index === 0 || index === 2 || (depth === 3 && index === 5))) return "updated";
  if (status?.persisted_root && (index === 0 || index === 3 || index === 5)) return "stored";
  return "default";
}

function SparseTreeDiagram({ status }: { status: SmtStatus | null }) {
  const effectiveRoot = shortRoot(status?.effective_root ?? null);
  const auditLabel = status?.consistent === false ? "audit mismatch" : status?.overlay_entries ? "pending overlay" : "canonical state";

  return (
    <div className="smt-tree" role="img" aria-label={`Sparse Merkle tree projection for ${auditLabel}`}>
      <svg viewBox="0 0 900 332" preserveAspectRatio="xMidYMid meet" aria-hidden="true">
        <g className="smt-tree__edges">
          {TREE_X.slice(0, -1).flatMap((level, depth) =>
            level.flatMap((x, index) => [0, 1].map((branch) => (
              <line
                key={`${depth}-${index}-${branch}`}
                x1={x}
                y1={TREE_Y[depth] + 10}
                x2={TREE_X[depth + 1][index * 2 + branch]}
                y2={TREE_Y[depth + 1] - 10}
              />
            ))),
          )}
        </g>

        <g className="smt-tree__axis" aria-hidden="true">
          <line x1="878" y1="30" x2="878" y2="274" />
          {[0, 1, 2, 3].map((height) => (
            <g key={height}>
              <line x1="872" y1={TREE_Y[3 - height]} x2="884" y2={TREE_Y[3 - height]} />
              <text x="890" y={TREE_Y[3 - height] + 4}>{height}</text>
            </g>
          ))}
          <text x="862" y="17">HEIGHT</text>
        </g>

        {TREE_X.map((level, depth) =>
          level.map((x, index) => {
            const kind = nodeKind(status, depth, index);
            const label = depth === 0
              ? `Root · ${effectiveRoot}`
              : depth === 3
                ? kind === "default" ? "Default leaf" : kind === "stored" ? "Stored leaf" : kind === "updated" ? "Overlay update" : "Proof / delete"
                : `H${3 - depth} · ${kind === "updated" ? "changed" : kind === "proof" ? "audit" : "hash"}`;
            return (
              <g className={`smt-tree__node smt-tree__node--${kind}`} key={`${depth}-${index}`}>
                <circle cx={x} cy={TREE_Y[depth]} r={depth === 0 ? 10 : 9} />
                <text x={x} y={depth === 0 ? TREE_Y[depth] - 18 : TREE_Y[depth] - 15} textAnchor="middle">
                  {label}
                </text>
              </g>
            );
          }),
        )}
      </svg>
      <div className="smt-tree__legend" aria-label="Sparse Merkle tree legend">
        <span><i className="smt-tree__dot smt-tree__dot--stored" />Persisted key/value leaf</span>
        <span><i className="smt-tree__dot smt-tree__dot--proof" />Delete, proof, or audit mismatch</span>
        <span><i className="smt-tree__dot smt-tree__dot--default" />Default empty hash</span>
        <span><i className="smt-tree__dot smt-tree__dot--updated" />Root or overlay update</span>
      </div>
      <p className="smt-tree__note">
        Structural projection: the node only exposes aggregate SMT diagnostics, not raw state keys or all 256 tree levels.
      </p>
    </div>
  );
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

      <section className="smt-page__tree-panel">
        <Panel
          title="Live Sparse Merkle Tree"
          subtitle="Root-to-leaf projection. Colors reflect the current node's persisted state, pending overlay, and audit result."
          action={auditPill(status, loading)}
        >
          <SparseTreeDiagram status={status} />
        </Panel>
      </section>

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
