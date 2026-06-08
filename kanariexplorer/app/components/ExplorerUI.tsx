import Link from "next/link";
import type { MouseEvent, ReactNode } from "react";

export type DataRecord = Record<string, unknown>;

export function asRecord(value: unknown): DataRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as DataRecord) : {};
}

export function asArray(value: unknown): unknown[] {
  if (Array.isArray(value)) return value;
  const record = asRecord(value);
  if (Array.isArray(record.result)) return record.result;
  if (Array.isArray(record.balances)) return record.balances;
  return [];
}

export function readString(value: unknown, key: string, fallback = "-") {
  const item = asRecord(value)[key];
  if (typeof item === "string") return item;
  if (typeof item === "number" || typeof item === "bigint") return String(item);
  return fallback;
}

export function readAddress(value: unknown, primaryKey: string, fallbackKey?: string, fallback = "-") {
  const primary = readString(value, primaryKey, "");
  if (primary) return primary;
  return fallbackKey ? readString(value, fallbackKey, fallback) : fallback;
}

export function readNumber(value: unknown, key: string) {
  const item = asRecord(value)[key];
  if (typeof item === "number") return item;
  if (typeof item === "string" && item.trim()) {
    const parsed = Number(item);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

export function shortHash(value: unknown, head = 10, tail = 8) {
  const text = String(value ?? "");
  if (text.length <= head + tail + 4) return text || "-";
  return `${text.slice(0, head)}...${text.slice(-tail)}`;
}

export function formatNumber(value: unknown) {
  const number = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(number)) return "-";
  return number.toLocaleString();
}

export function formatBalance(value: unknown, decimalsValue: unknown = 9) {
  if (value === null || value === undefined || value === "") return "-";
  const decimals = Number(decimalsValue);
  try {
    const raw = BigInt(String(value));
    const scale = BigInt(10) ** BigInt(Number.isFinite(decimals) ? decimals : 9);
    const whole = String(raw / scale).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    const fraction = raw % scale;
    if (fraction === BigInt(0)) return whole;
    return `${whole}.${String(fraction).padStart(Number(decimals), "0").replace(/0+$/, "")}`;
  } catch {
    return String(value);
  }
}

export function PageHeader({
  eyebrow,
  title,
  accent,
  description,
  children,
}: {
  eyebrow: string;
  title: string;
  accent?: string;
  description: string;
  children?: ReactNode;
}) {
  return (
    <section className="subpage-hero explorer-page-hero">
      <p className="section-kicker">{eyebrow}</p>
      <h1>
        {title}
        {accent ? (
          <>
            <br />
            <span>{accent}</span>
          </>
        ) : null}
      </h1>
      <p className="subpage-hero__description">{description}</p>
      {children ? <div className="hero-actions explorer-page-actions">{children}</div> : null}
    </section>
  );
}

export function SearchForm({
  value,
  placeholder,
  buttonLabel = "Search",
  onChange,
  onSubmit,
}: {
  value: string;
  placeholder: string;
  buttonLabel?: string;
  onChange: (value: string) => void;
  onSubmit?: () => void;
}) {
  return (
    <form
      className="search-box"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit?.();
      }}
    >
      <span className="search-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" width="18" height="18">
          <path d="m21 21-4.35-4.35m1.35-5.65a7 7 0 1 1-14 0 7 7 0 0 1 14 0Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
      </span>
      <input value={value} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} />
      <button className="button" type="submit">
        {buttonLabel}
      </button>
    </form>
  );
}

export function StatCard({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <article className="stat-card">
      <strong>{value}</strong>
      <span>{label}</span>
      <p className="stat-card__detail">{detail}</p>
    </article>
  );
}

export function Panel({
  title,
  subtitle,
  action,
  children,
}: {
  title: string;
  subtitle?: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="panel">
      <div className="panel-head">
        <div>
          <h2 className="panel-title">{title}</h2>
          {subtitle ? <p className="panel-subtitle">{subtitle}</p> : null}
        </div>
        {action}
      </div>
      {children}
    </section>
  );
}

export function StatusPill({ label, state = "ok" }: { label: string; state?: "ok" | "warn" | "down" }) {
  return (
    <span className="status-pill">
      <span className={`dot ${state === "warn" ? "dot--warn" : state === "down" ? "dot--down" : ""}`} />
      <span className="status-pill__label">{label}</span>
    </span>
  );
}

export function EmptyState({ loading, label }: { loading?: boolean; label: string }) {
  return (
    <div className="empty-state">
      {loading ? <div className="spinner" /> : null}
      {label}
    </div>
  );
}

export function CopyButton({ value, label = "Copy" }: { value: string; label?: string }) {
  async function copyValue(event: MouseEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();

    try {
      await navigator.clipboard.writeText(value);
    } catch {
      // Clipboard permissions can be blocked in non-secure contexts.
    }
  }

  return (
    <button className="copy-button" type="button" onClick={copyValue} aria-label={label} title={label}>
      <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path d="M8 8h10v12H8z" />
        <path d="M6 16H4V4h12v2" />
      </svg>
    </button>
  );
}

export function RawDetails({ label, value }: { label: string; value: unknown }) {
  return (
    <details className="raw-details">
      <summary>{label}</summary>
      <pre className="raw-box custom-scrollbar">{JSON.stringify(value, null, 2)}</pre>
    </details>
  );
}

export function RouteList() {
  const routes = [
    ["Transactions", "/tx"],
    ["Token Registry", "/coins"],
    ["System Modules", "/modules"],
    ["Account Lookup", "/account"],
    ["NFT Collections", "/nft"],
  ];

  return (
    <div className="route-list">
      {routes.map(([label, href]) => (
        <Link className="route-card" href={href} key={href}>
          {label}
          <span aria-hidden="true">-&gt;</span>
        </Link>
      ))}
    </div>
  );
}
