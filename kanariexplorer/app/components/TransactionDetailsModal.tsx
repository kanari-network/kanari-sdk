import { describeTransactionLifecycle, EmptyState, RawDetails, readAddress, readString, shortHash, StatusPill } from "./ExplorerUI";
import ObjectGraphView from "./ObjectGraphView";

function readFirstString(value: unknown, keys: string[], fallback = "-") {
  for (const key of keys) {
    const item = readString(value, key, "");
    if (item) return item;
  }

  return fallback;
}

function readOptionalArray(value: unknown, key: string) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return [];
  const item = (value as Record<string, unknown>)[key];
  return Array.isArray(item) ? item.map((entry) => String(entry)) : [];
}

function readArrayLength(value: unknown, key: string) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return 0;
  const item = (value as Record<string, unknown>)[key];
  return Array.isArray(item) ? item.length : 0;
}

function readObject(value: unknown, key: string) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const item = (value as Record<string, unknown>)[key];
  return item && typeof item === "object" && !Array.isArray(item) ? (item as Record<string, unknown>) : null;
}

function readArray(value: unknown, key: string) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return [];
  const item = (value as Record<string, unknown>)[key];
  return Array.isArray(item) ? item : [];
}

function readBooleanField(value: unknown, key: string) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return "false";
  const item = (value as Record<string, unknown>)[key];
  return item === true ? "true" : item === false ? "false" : String(item ?? "false");
}

function DetailItem({ label, value, mono = false, wide = false }: { label: string; value: string; mono?: boolean; wide?: boolean }) {
  if (!value || value === "-") return null;

  return (
    <div className={wide ? "tx-detail-item tx-detail-item--wide" : "tx-detail-item"}>
      <p className="tiny-label">{label}</p>
      <span className={mono ? "mono break-anywhere" : "break-anywhere"}>{value}</span>
    </div>
  );
}

function splitPublishedFunction(value: string) {
  const parts = value.split("::");
  if (parts.length < 3) {
    return { full: value, functionName: value, modulePath: "" };
  }

  return {
    full: value,
    functionName: parts[parts.length - 1] ?? value,
    modulePath: parts.slice(0, -1).join("::"),
  };
}

function PublishedFunctionsList({ functions }: { functions: string[] }) {
  if (functions.length === 0) return null;

  return (
    <details className="object-graph-details tx-published-functions" aria-label="Published functions">
      <summary>Published Functions</summary>
      <div className="tx-published-functions__meta">
        <p className="panel-subtitle">Functions declared in the published module bytecode.</p>
        <StatusPill label={`${functions.length} functions`} state="ok" />
      </div>
      <div className="tx-published-functions__list">
        {functions.map((item) => (
          <div className="tx-published-functions__item" key={item}>
            <p className="tiny-label">Function</p>
            <strong className="tx-published-functions__name mono">{splitPublishedFunction(item).functionName}</strong>
            {splitPublishedFunction(item).modulePath ? (
              <span className="tx-published-functions__path mono break-anywhere">
                {splitPublishedFunction(item).modulePath}
              </span>
            ) : null}
          </div>
        ))}
      </div>
    </details>
  );
}

export default function TransactionDetailsModal({
  open,
  loading,
  transaction,
  onClose,
}: {
  open: boolean;
  loading: boolean;
  transaction: unknown;
  onClose: () => void;
}) {
  if (!open) return null;

  const hash = readFirstString(transaction, ["hash", "tx_hash"]);
  const status = describeTransactionLifecycle(transaction);
  const senderAddress = readAddress(transaction, "sender_address", "sender");
  const transactionType = readFirstString(transaction, ["tx_type", "type"], "operation").replace(/_/g, " ");
  const publishedModule = readFirstString(transaction, ["module"]);
  const moduleFunctions = readOptionalArray(transaction, "module_functions");
  const effects = readObject(transaction, "effects");
  const objectInputs = readArrayLength(transaction, "object_inputs");
  const effectObjectChanges = readArrayLength(effects, "object_changes");
  const createdCount = readArrayLength(effects, "created");
  const mutatedCount = readArrayLength(effects, "mutated");
  const deletedCount = readArrayLength(effects, "deleted");
  const transferredCount = readArrayLength(effects, "transferred");
  const graphEdgeCount = readArrayLength(effects, "causal_edges");
  const gasPaymentObjectCount = readArrayLength(readObject(transaction, "gas_payment"), "payment_objects");
  const effectInputObjects = readArray(effects, "input_objects");
  const effectSharedInputs = readArray(effects, "shared_inputs");
  const effectImmutableInputs = readArray(effects, "immutable_inputs");
  const effectGasObjects = readArray(effects, "gas_object_refs");
  const effectObjectChangesList = readArray(effects, "object_changes");
  const effectGraphEdgesList = readArray(effects, "causal_edges");
  const declaredObjectInputs = readArray(transaction, "object_inputs");
  const gasPaymentObjects = readArray(readObject(transaction, "gas_payment"), "payment_objects");

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" aria-label="Transaction details">
      <section className="modal-card">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">Transaction Details</h2>
            <p className="panel-subtitle mono">{transaction ? shortHash(hash) : "Loading transaction"}</p>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label="Close transaction details">
            <svg viewBox="0 0 24 24" fill="none">
              <path d="m6 6 12 12M18 6 6 18" />
            </svg>
          </button>
        </div>
        <div className="modal-body custom-scrollbar">
          {loading ? (
            <EmptyState loading label="Syncing transaction data..." />
          ) : transaction ? (
            <>
              <div className="tx-summary-grid">
                <article className="tx-summary-card">
                  <span className="tx-summary-label">Hash</span>
                  <strong className="tx-summary-value mono">
                    {hash}
                  </strong>
                </article>
                <article className="tx-summary-card">
                  <span className="tx-summary-label">Lifecycle</span>
                  <strong className="tx-summary-value">
                    <StatusPill label={status.label} state={status.state} />
                  </strong>
                </article>
              </div>

              <section className="tx-detail-grid" aria-label="Transaction fields">
                <DetailItem label="Type" value={transactionType} />
                <DetailItem label="Status Detail" value={status.detail || readFirstString(transaction, ["status"], "unknown")} />
                <DetailItem label="Checkpoint Height" value={readFirstString(transaction, ["checkpoint_height", "block_height", "height"])} mono />
                <DetailItem label="Sender" value={senderAddress} mono wide />
                <DetailItem label="Recipient / Target" value={shortHash(readFirstString(transaction, ["recipient", "to", "module"]))} mono wide />
                <DetailItem label="Function" value={readFirstString(transaction, ["function"])} mono />
                <DetailItem label="Published Module" value={publishedModule} mono wide />
                <DetailItem label="Nonce" value={readFirstString(transaction, ["nonce"])} mono />
                <DetailItem label="Gas Limit" value={readFirstString(transaction, ["gas_limit", "gas"])} mono />
                <DetailItem label="Gas Price" value={readFirstString(transaction, ["gas_price"])} mono />
                <DetailItem label="Gas Used" value={readFirstString(transaction, ["gas_used"])} mono />
                <DetailItem label="Object Inputs" value={objectInputs > 0 ? String(objectInputs) : "-"} mono />
                <DetailItem label="Gas Objects" value={gasPaymentObjectCount > 0 ? String(gasPaymentObjectCount) : "-"} mono />
                <DetailItem label="Object Changes" value={effectObjectChanges > 0 ? String(effectObjectChanges) : "-"} mono />
                <DetailItem label="Graph Edges" value={graphEdgeCount > 0 ? String(graphEdgeCount) : "-"} mono />
                <DetailItem label="Previewed" value={readBooleanField(transaction, "previewed")} mono />
                <DetailItem label="Submitted" value={readBooleanField(transaction, "submitted")} mono />
                <DetailItem label="Committed" value={readBooleanField(transaction, "committed")} mono />
                <DetailItem label="Success" value={readBooleanField(transaction, "success")} mono />
                <DetailItem
                  label="Effects Summary"
                  value={
                    effectObjectChanges > 0
                      ? `created ${createdCount}, mutated ${mutatedCount}, deleted ${deletedCount}, transferred ${transferredCount}`
                      : "-"
                  }
                  mono
                  wide
                />
                <DetailItem label="Action" value={readFirstString(transaction, ["action"])} />
              </section>

              <PublishedFunctionsList functions={moduleFunctions} />

              <ObjectGraphView
                title="Object Graph Timeline"
                subtitle="Execution access sets, object mutations, and causal dependencies for this transaction."
                objectInputs={effectInputObjects.length > 0 ? effectInputObjects : declaredObjectInputs}
                sharedInputs={effectSharedInputs}
                immutableInputs={effectImmutableInputs}
                gasObjects={effectGasObjects.length > 0 ? effectGasObjects : gasPaymentObjects}
                objectChanges={effectObjectChangesList}
                graphEdges={effectGraphEdgesList}
              />

              <RawDetails label="Raw transaction JSON" value={transaction} />
            </>
          ) : (
            <EmptyState label="No transaction data found." />
          )}
        </div>
      </section>
    </div>
  );
}
