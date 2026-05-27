"use client";

import { useState } from "react";

type TransactionDetailsModalProps = {
  open: boolean;
  loading: boolean;
  transaction: TxRecord | null;
  onClose: () => void;
};

type TxRecord = Record<string, unknown>;

function valueOrDash(value: unknown) {
  if (value === null || value === undefined || value === "") return "-";
  return String(value);
}

function titleCase(value: unknown) {
  return valueOrDash(value)
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function shortValue(value: unknown, head = 12, tail = 10) {
  const text = valueOrDash(value);
  if (text === "-" || text.length <= head + tail + 3) return text;
  return `${text.slice(0, head)}...${text.slice(-tail)}`;
}

function statusClass(status: unknown) {
  const normalized = String(status).toLowerCase();
  if (normalized === "pending") {
    return "border-amber-400/20 bg-amber-400/10 text-amber-300";
  }
  if (normalized === "failed" || normalized === "error") {
    return "border-rose-400/20 bg-rose-400/10 text-rose-300";
  }
  return "border-emerald-400/20 bg-emerald-400/10 text-emerald-300";
}

function fieldLabel(label: string) {
  return (
    <div className="text-[9px] font-black uppercase tracking-[0.14em] text-zinc-500 sm:text-[10px] sm:tracking-[0.18em]">
      {label}
    </div>
  );
}

function CopyButton({ value }: { value: unknown }) {
  const [copied, setCopied] = useState(false);
  const text = valueOrDash(value);

  async function copy() {
    if (text === "-") return;
    await navigator.clipboard?.writeText(text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  }

  return (
    <button
      onClick={copy}
      className="rounded-lg border border-white/10 bg-white/5 px-2 py-1 text-[9px] font-black uppercase tracking-widest text-zinc-400 transition hover:border-cyan-400/30 hover:text-cyan-300 sm:px-2.5 sm:text-[10px]"
      type="button"
    >
      {copied ? "Copied" : "Copy"}
    </button>
  );
}

function InfoCard({
  label,
  value,
  tone = "default",
  copy = false,
}: {
  label: string;
  value: unknown;
  tone?: "default" | "cyan" | "emerald" | "amber";
  copy?: boolean;
}) {
  const color =
    tone === "cyan"
      ? "text-cyan-300"
      : tone === "emerald"
        ? "text-emerald-300"
        : tone === "amber"
          ? "text-amber-300"
          : "text-zinc-100";

  return (
    <div className="rounded-2xl border border-white/10 bg-white/[0.03] p-3 sm:p-4">
      <div className="mb-2 flex items-center justify-between gap-3">
        {fieldLabel(label)}
        {copy && <CopyButton value={value} />}
      </div>
      <div className={`break-all font-mono text-xs leading-relaxed sm:text-sm ${color}`}>
        {valueOrDash(value)}
      </div>
    </div>
  );
}

function DetailRow({
  label,
  value,
  copy = false,
}: {
  label: string;
  value: unknown;
  copy?: boolean;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-x-3 gap-y-2 border-b border-white/5 py-3 last:border-b-0 md:grid-cols-[150px_minmax(0,1fr)_auto] md:items-center">
      <div className="col-span-2 md:col-span-1">{fieldLabel(label)}</div>
      <div className="min-w-0 break-all font-mono text-xs leading-relaxed text-zinc-200 sm:text-sm">
        {valueOrDash(value)}
      </div>
      {copy && <CopyButton value={value} />}
    </div>
  );
}

function ModuleFunctions({ functions }: { functions: unknown }) {
  if (!Array.isArray(functions) || functions.length === 0) return null;

  return (
    <div className="rounded-2xl border border-white/10 bg-white/[0.03] p-3 sm:p-4">
      {fieldLabel("Module Functions")}
      <div className="mt-3 flex flex-wrap gap-2">
        {functions.map((fnName, index) => (
          <span
            key={`${fnName}-${index}`}
            className="rounded-lg border border-emerald-400/15 bg-emerald-400/10 px-2.5 py-1 font-mono text-[11px] text-emerald-300 sm:text-xs"
          >
            {fnName}
          </span>
        ))}
      </div>
    </div>
  );
}

export default function TransactionDetailsModal({
  open,
  loading,
  transaction,
  onClose,
}: TransactionDetailsModalProps) {
  if (!open) return null;

  const tx: TxRecord = transaction ?? {};
  const status = valueOrDash(tx.status);
  const action =
    tx.module && tx.function
      ? `${tx.module}::${tx.function}`
      : tx.function || tx.module || titleCase(tx.tx_type);
  const target = tx.recipient ?? tx.to ?? tx.receiver ?? tx.module ?? "-";
  const rawJson = transaction ? JSON.stringify(transaction, null, 2) : "";

  return (
    <div className="fixed inset-0 z-[100] flex items-end justify-center bg-black/80 p-0 backdrop-blur-xl sm:items-center sm:p-4">
      <div className="flex h-[100dvh] max-h-[100dvh] w-full flex-col overflow-hidden rounded-none border-white/10 bg-[#111113] shadow-[0_0_100px_rgba(16,185,129,0.12)] sm:h-auto sm:max-h-[92dvh] sm:max-w-5xl sm:rounded-[28px] sm:border">
        <div className="shrink-0 border-b border-white/5 p-4 sm:p-5 md:p-6">
          <div className="flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-3 sm:gap-4">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl border border-emerald-500/20 bg-emerald-500/10 sm:h-11 sm:w-11">
                <svg
                  className="h-5 w-5 text-emerald-400"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                  />
                </svg>
              </div>
              <div className="min-w-0">
                <h3 className="text-base font-black tracking-tight text-white sm:text-lg md:text-xl">
                  Transaction Details
                </h3>
                {!loading && transaction && (
                  <div className="mt-1 truncate font-mono text-[11px] text-zinc-500 sm:text-xs">
                    {shortValue(tx.hash, 12, 10)}
                  </div>
                )}
              </div>
            </div>
            <button
              onClick={onClose}
              className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/10 bg-white/5 text-white transition hover:border-emerald-400/40 hover:bg-emerald-500/20 sm:h-11 sm:w-11"
              type="button"
            >
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-4 custom-scrollbar sm:p-5 md:p-6">
          {loading ? (
            <div className="py-24 text-center font-mono text-sm text-zinc-600">
              Syncing transaction data...
            </div>
          ) : transaction ? (
            <div className="space-y-4 sm:space-y-5">
              <div className="grid gap-3 sm:gap-4 md:grid-cols-[1.4fr_0.8fr_0.8fr]">
                <InfoCard label="Action" value={action} tone="cyan" />
                <div className="rounded-2xl border border-white/10 bg-white/[0.03] p-3 sm:p-4">
                  {fieldLabel("Status")}
                  <div
                    className={`mt-3 inline-flex rounded-full border px-3 py-1.5 text-[11px] font-black uppercase tracking-widest sm:text-xs ${statusClass(status)}`}
                  >
                    {status}
                  </div>
                </div>
                <InfoCard
                  label="Block"
                  value={tx.block_height != null ? tx.block_height : "Mempool"}
                  tone={tx.block_height != null ? "emerald" : "amber"}
                />
              </div>

              <InfoCard label="Transaction Hash" value={tx.hash} tone="cyan" copy />

              <div className="grid gap-4 lg:grid-cols-2">
                <section className="rounded-2xl border border-white/10 bg-black/20 p-4 sm:p-5">
                  <h4 className="mb-2 text-sm font-black text-white">Participants</h4>
                  <DetailRow label="Sender" value={tx.sender} copy />
                  <DetailRow label="Target" value={target} copy={target !== "-"} />
                  <DetailRow label="Sequence" value={tx.sequence_number} />
                </section>

                <section className="rounded-2xl border border-white/10 bg-black/20 p-4 sm:p-5">
                  <h4 className="mb-2 text-sm font-black text-white">Execution</h4>
                  <DetailRow label="Type" value={titleCase(tx.tx_type)} />
                  <DetailRow label="Module" value={tx.module} copy={Boolean(tx.module)} />
                  <DetailRow label="Function" value={tx.function} />
                </section>
              </div>

              <div className="grid gap-4 lg:grid-cols-2">
                <section className="rounded-2xl border border-white/10 bg-black/20 p-4 sm:p-5">
                  <h4 className="mb-2 text-sm font-black text-white">Gas And Value</h4>
                  <DetailRow label="Amount" value={tx.amount} />
                  <DetailRow label="Gas Limit" value={tx.gas_limit} />
                  <DetailRow label="Gas Price" value={tx.gas_price} />
                  <DetailRow label="Gas Used" value={tx.gas_used} />
                </section>

                <ModuleFunctions functions={tx.module_functions} />
              </div>

              <details className="group rounded-2xl border border-white/10 bg-[#09090b]">
                <summary className="flex cursor-pointer list-none items-center justify-between gap-3 p-4 text-[10px] font-black uppercase tracking-[0.18em] text-zinc-500 transition hover:text-emerald-300 sm:tracking-[0.22em]">
                  Raw JSON
                  <span className="text-zinc-700 transition group-open:rotate-90">&gt;</span>
                </summary>
                <div className="border-t border-white/5 p-3 sm:p-4">
                  <pre className="max-h-72 overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed text-emerald-400/70 custom-scrollbar sm:max-h-80 sm:text-xs">
                    {rawJson}
                  </pre>
                </div>
              </details>
            </div>
          ) : (
            <div className="py-24 text-center font-mono text-sm text-zinc-600">
              No data found.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
