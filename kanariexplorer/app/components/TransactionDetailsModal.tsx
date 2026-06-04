import { EmptyState, RawDetails, readString, shortHash } from "./ExplorerUI";

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

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" aria-label="Transaction details">
      <section className="modal-card">
        <div className="panel-head">
          <div>
            <h2 className="panel-title">Transaction Details</h2>
            <p className="panel-subtitle mono">{transaction ? shortHash(readString(transaction, "hash")) : "Loading transaction"}</p>
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
              <div className="grid-cards">
                <article className="panel stat-card">
                  <span className="stat-card__label">Hash</span>
                  <strong className="stat-card__value mono stat-card__value--hash">
                    {readString(transaction, "hash")}
                  </strong>
                </article>
                <article className="panel stat-card">
                  <span className="stat-card__label">Status</span>
                  <strong className="stat-card__value">{readString(transaction, "status", "unknown")}</strong>
                </article>
              </div>
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
