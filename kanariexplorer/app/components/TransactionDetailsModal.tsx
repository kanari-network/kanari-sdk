import { EmptyState, RawDetails, readString, shortHash } from "./ExplorerUI";

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

function DetailItem({ label, value, mono = false, wide = false }: { label: string; value: string; mono?: boolean; wide?: boolean }) {
  if (!value || value === "-") return null;

  return (
    <div className={wide ? "tx-detail-item tx-detail-item--wide" : "tx-detail-item"}>
      <p className="tiny-label">{label}</p>
      <span className={mono ? "mono break-anywhere" : "break-anywhere"}>{value}</span>
    </div>
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
  const status = readFirstString(transaction, ["status"], "unknown");
  const moduleFunctions = readOptionalArray(transaction, "module_functions");

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
                  <span className="tx-summary-label">Status</span>
                  <strong className="tx-summary-value">{status}</strong>
                </article>
              </div>

              <section className="tx-detail-grid" aria-label="Transaction fields">
                <DetailItem label="Type" value={readFirstString(transaction, ["tx_type", "type"], "operation").replace(/_/g, " ")} />
                <DetailItem label="Block Height" value={readFirstString(transaction, ["block_height", "height"])} mono />
                <DetailItem label="Sender" value={readFirstString(transaction, ["sender", "from"])} mono wide />
                <DetailItem label="Recipient / Target" value={readFirstString(transaction, ["recipient", "to", "module"])} mono wide />
                <DetailItem label="Function" value={readFirstString(transaction, ["function"])} mono />
                <DetailItem label="Sequence" value={readFirstString(transaction, ["sequence_number", "sequence", "nonce"])} mono />
                <DetailItem label="Gas Limit" value={readFirstString(transaction, ["gas_limit", "gas"])} mono />
                <DetailItem label="Gas Price" value={readFirstString(transaction, ["gas_price"])} mono />
                <DetailItem label="Gas Used" value={readFirstString(transaction, ["gas_used"])} mono />
                <DetailItem label="Action" value={readFirstString(transaction, ["action"])} />
                {moduleFunctions.length > 0 ? (
                  <DetailItem label="Module Functions" value={moduleFunctions.join(", ")} mono wide />
                ) : null}
              </section>

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
