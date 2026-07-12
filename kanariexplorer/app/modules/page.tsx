"use client";

import { useEffect, useState } from "react";
import { asArray, EmptyState, PageHeader, Panel, RawDetails, readString, SearchForm, shortHash, StatusPill } from "../components/ExplorerUI";
import { listModules } from "../lib/rpc";

function moduleSearchText(module: unknown) {
  return [
    readString(module, "name", ""),
    readString(module, "address", ""),
    readString(module, "bytecode_hash", ""),
    readString(module, "size", ""),
    readString(module, "function_count", ""),
    readString(module, "functions", ""),
  ]
    .join(" ")
    .toLowerCase();
}

function readStringArray(value: unknown, key: string) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return [];
  const item = (value as Record<string, unknown>)[key];
  return Array.isArray(item) ? item.map((entry) => String(entry)) : [];
}

export default function ModulesPage() {
  const [search, setSearch] = useState("");
  const [modules, setModules] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(true);

  const normalizedSearch = search.trim().toLowerCase();
  const filteredModules = normalizedSearch
    ? modules.filter((module) => moduleSearchText(module).includes(normalizedSearch))
    : modules;

  useEffect(() => {
    async function loadModules() {
      setLoading(true);
      try {
        setModules(asArray(await listModules()));
      } catch {
        setModules([]);
      } finally {
        setLoading(false);
      }
    }

    loadModules();
  }, []);

  return (
    <div className="explorer-wrap">
      <PageHeader
        eyebrow="System Modules"
        title="Module"
        accent="Registry."
        description="Published Move modules reported by the Kanari RPC system module registry."
      >
        <SearchForm value={search} onChange={setSearch} placeholder="Search module name, address, or hash" buttonLabel="Filter" />
      </PageHeader>

      <Panel
        title="System Module List"
        subtitle={normalizedSearch ? `Showing ${filteredModules.length} of ${modules.length} modules` : "Published modules from the network"}
        action={<StatusPill label={loading ? "Syncing" : `${filteredModules.length} modules`} state={loading ? "warn" : "ok"} />}
      >
        {loading ? <EmptyState loading label="Loading system modules..." /> : null}
        {!loading && modules.length === 0 ? <EmptyState label="No modules found." /> : null}
        {!loading && modules.length > 0 && filteredModules.length === 0 ? <EmptyState label="No modules match this search." /> : null}
        {filteredModules.length > 0 ? (
          <div className="module-list">
            {filteredModules.map((module, index) => (
              <div className="module-row" key={`${readString(module, "address", "module")}-${readString(module, "name", String(index))}`}>
                <div className="primary-text">
                  <strong>{readString(module, "name", `Module ${index + 1}`)}</strong>
                  <div className="muted-text mono">{shortHash(readString(module, "address"))}</div>
                </div>
                <div>
                  <p className="tiny-label">Size</p>
                  <span className="mono">{readString(module, "size")}</span>
                </div>
                <div>
                  <p className="tiny-label">Functions</p>
                  <span className="mono">{readString(module, "function_count", "0")}</span>
                </div>
                <div>
                  <p className="tiny-label">Bytecode Hash</p>
                  <span className="mono muted-text break-anywhere">{readString(module, "bytecode_hash")}</span>
                </div>
                {readStringArray(module, "functions").length > 0 ? (
                  <details className="object-json-details module-functions-details">
                    <summary>Functions</summary>
                    <div className="module-functions-list">
                      {readStringArray(module, "functions").map((fnName) => (
                        <div className="module-functions-list__item" key={fnName}>
                          <span className="mono break-anywhere">{fnName}</span>
                        </div>
                      ))}
                    </div>
                  </details>
                ) : null}
              </div>
            ))}
          </div>
        ) : null}
      </Panel>

      <RawDetails label="Developer: modules JSON" value={filteredModules} />
    </div>
  );
}
