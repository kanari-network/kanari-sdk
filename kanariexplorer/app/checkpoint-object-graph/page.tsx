"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import ObjectGraphView from "../components/ObjectGraphView";
import {
  asArray,
  EmptyState,
  PageHeader,
  Panel,
  describeTransactionLifecycle,
  formatNumber,
  readString,
  SearchForm,
  StatusPill,
} from "../components/ExplorerUI";
import { ArrowIcon } from "../components/SiteChrome";
import { getBlockHeight, getFullBlock } from "../lib/rpc";

function readArrayLength(source: unknown, key: string) {
  if (typeof source !== "object" || source === null || Array.isArray(source)) return 0;
  const value = (source as Record<string, unknown>)[key];
  return Array.isArray(value) ? value.length : 0;
}

export default function CheckpointObjectGraphPage() {
  const [heightInput, setHeightInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [blockHeight, setBlockHeight] = useState<number | null>(null);
  const [checkpoint, setCheckpoint] = useState<unknown>(null);

  async function loadCheckpoint(targetHeight?: number | null) {
    setLoading(true);
    setError("");

    try {
      const resolvedHeight =
        typeof targetHeight === "number" && Number.isFinite(targetHeight)
          ? targetHeight
          : Number(await getBlockHeight());

      if (!Number.isFinite(resolvedHeight)) {
        throw new Error("Checkpoint height is not available yet.");
      }

      const block = await getFullBlock(resolvedHeight);
      setBlockHeight(resolvedHeight);
      setCheckpoint(block);
      setHeightInput(String(resolvedHeight));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load checkpoint object graph.");
      setCheckpoint(null);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      void loadCheckpoint();
    }, 0);
    return () => window.clearTimeout(timeout);
  }, []);

  const effectCount = readArrayLength(checkpoint, "transaction_effects");
  const objectChangeCount = readArrayLength(checkpoint, "object_changes");
  const graphEdgeCount = readArrayLength(checkpoint, "object_graph_edges");
  const effects = asArray(
    typeof checkpoint === "object" && checkpoint !== null && !Array.isArray(checkpoint)
      ? (checkpoint as Record<string, unknown>).transaction_effects
      : [],
  );
  const objectChanges =
    typeof checkpoint === "object" && checkpoint !== null && !Array.isArray(checkpoint)
      ? (checkpoint as Record<string, unknown>).object_changes
      : [];
  const graphEdges =
    typeof checkpoint === "object" && checkpoint !== null && !Array.isArray(checkpoint)
      ? (checkpoint as Record<string, unknown>).object_graph_edges
      : [];

  return (
    <div className="explorer-wrap checkpoint-graph-page">
      <PageHeader
        eyebrow="Checkpoint Object Graph"
        title="Checkpoint"
        accent="Timeline."
        description="Inspect transaction effects, object mutations, and causal graph edges for one checkpoint without crowding the live overview dashboard."
      >
        <SearchForm
          value={heightInput}
          onChange={setHeightInput}
          onSubmit={() => {
            const parsed = Number(heightInput);
            if (Number.isFinite(parsed)) {
              void loadCheckpoint(parsed);
            }
          }}
          placeholder="Enter checkpoint height"
          buttonLabel="Load"
        />
      </PageHeader>

      <section className="content-grid checkpoint-graph-page__grid">
        <Panel
          title="Checkpoint Summary"
          subtitle={blockHeight === null ? "Waiting for checkpoint height" : `Height ${formatNumber(blockHeight)}`}
          action={<StatusPill label={checkpoint ? "Loaded" : loading ? "Loading" : "Pending"} state={checkpoint ? "ok" : loading ? "warn" : "down"} />}
        >
          {loading ? <EmptyState loading label="Loading checkpoint object graph..." /> : null}
          {!loading && error ? <EmptyState label={error} /> : null}
          {!loading && checkpoint ? (
            <div className="block-summary">
              <div className="block-summary__top block-summary__top--triple">
                <div>
                  <p className="tiny-label">Height</p>
                  <strong className="mono block-summary__height">{readString(checkpoint, "height", formatNumber(blockHeight))}</strong>
                </div>
                <div>
                  <p className="tiny-label">Transactions</p>
                  <strong className="mono">
                    {readString(checkpoint, "tx_count", readString(checkpoint, "transaction_count", readString(checkpoint, "transactions_len", "-")))}
                  </strong>
                </div>
                <div>
                  <p className="tiny-label">Effects</p>
                  <strong className="mono">{formatNumber(effectCount)}</strong>
                </div>
              </div>
              <div className="block-summary__top block-summary__top--triple">
                <div>
                  <p className="tiny-label">Object Changes</p>
                  <strong className="mono">{formatNumber(objectChangeCount)}</strong>
                </div>
                <div>
                  <p className="tiny-label">Graph Edges</p>
                  <strong className="mono">{formatNumber(graphEdgeCount)}</strong>
                </div>
                <div>
                  <p className="tiny-label">State Root</p>
                  <strong className="mono checkpoint-graph-page__root">
                    {readString(checkpoint, "state_root")}
                  </strong>
                </div>
              </div>
              <div>
                <p className="tiny-label">Hash</p>
                <p className="mono muted-text block-summary__hash">{readString(checkpoint, "hash")}</p>
              </div>
            </div>
          ) : null}
        </Panel>

        <Panel
          title="Graph Focus"
          subtitle="Use this page when you want the full checkpoint graph without the live node dashboard competing for space."
          action={
            <Link className="button button--ghost checkpoint-graph-page__back" href="/">
              Overview <ArrowIcon />
            </Link>
          }
        >
          <div className="checkpoint-graph-callout">
            <p>
              Each transaction effect below is grouped with its lifecycle signal, then rolled into one aggregated object graph for the selected checkpoint.
            </p>
          </div>
        </Panel>
      </section>

      {!loading && !error && effects.length > 0 ? (
        <section className="panel checkpoint-graph-page__effects">
          <div className="panel-head">
            <div>
              <h2 className="panel-title">Transaction Effects</h2>
              <p className="panel-subtitle">Lifecycle and footprint for each effect included in this checkpoint.</p>
            </div>
          </div>
          <div className="checkpoint-effect-strip">
            {effects.map((effect, index) => {
              const lifecycle = describeTransactionLifecycle(effect);
              return (
                <article className="checkpoint-effect-card" key={`effect-${index}`}>
                  <div className="checkpoint-effect-card__head">
                    <span className="checkpoint-effect-card__title">Tx Effect {index + 1}</span>
                    <StatusPill label={lifecycle.label} state={lifecycle.state} />
                  </div>
                  <div className="checkpoint-effect-card__grid">
                    <div>
                      <p className="tiny-label">Gas Used</p>
                      <strong className="mono">{readString(effect, "gas_used", "-")}</strong>
                    </div>
                    <div>
                      <p className="tiny-label">Inputs</p>
                      <strong className="mono">{readArrayLength(effect, "input_objects")}</strong>
                    </div>
                    <div>
                      <p className="tiny-label">Changes</p>
                      <strong className="mono">{readArrayLength(effect, "object_changes")}</strong>
                    </div>
                    <div>
                      <p className="tiny-label">Edges</p>
                      <strong className="mono">{readArrayLength(effect, "causal_edges")}</strong>
                    </div>
                  </div>
                </article>
              );
            })}
          </div>
        </section>
      ) : null}

      {!loading && !error ? (
        <ObjectGraphView
          title="Checkpoint Object Graph"
          subtitle="Aggregated object graph for the selected checkpoint across all executed transaction effects."
          objectChanges={objectChanges}
          graphEdges={graphEdges}
        />
      ) : null}
    </div>
  );
}
