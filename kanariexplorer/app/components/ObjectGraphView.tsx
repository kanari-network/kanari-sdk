import { asArray, asRecord, readString, shortHash } from "./ExplorerUI";

type ObjectGraphViewProps = {
  title: string;
  subtitle?: string;
  objectInputs?: unknown;
  sharedInputs?: unknown;
  immutableInputs?: unknown;
  gasObjects?: unknown;
  objectChanges?: unknown;
  graphEdges?: unknown;
};

function readObjectRef(value: unknown) {
  const record = asRecord(value);
  const source = record.object_ref && typeof record.object_ref === "object" && !Array.isArray(record.object_ref) ? asRecord(record.object_ref) : record;
  return {
    digest: readString(source, "digest", ""),
    objectId: readString(source, "object_id", "-"),
    version: readString(source, "version", ""),
  };
}

function readObjectOwner(value: unknown) {
  const record = asRecord(value);
  const owner = record.owner;
  if (typeof owner === "string") return owner;
  if (owner && typeof owner === "object" && !Array.isArray(owner)) {
    const ownerRecord = asRecord(owner);
    const addressOwner = ownerRecord.AddressOwner;
    if (typeof addressOwner === "string") return `owned by ${shortHash(addressOwner, 10, 6)}`;
    if ("Shared" in ownerRecord) return "shared";
    if ("Immutable" in ownerRecord) return "immutable";
  }
  return "";
}

function readChangeType(value: unknown) {
  return readString(value, "change_type", "unknown").toLowerCase();
}

function readRelation(value: unknown) {
  return readString(value, "relation", "unknown").replace(/_/g, " ");
}

function readBooleanTag(value: unknown, key: string, label: string) {
  const item = asRecord(value)[key];
  return item === true ? label : "";
}

function GraphRefCard({ value }: { value: unknown }) {
  const objectRef = readObjectRef(value);
  const owner = readObjectOwner(value);
  const mutableTag = readBooleanTag(value, "mutable", "mutable");
  const tags = [owner, mutableTag].filter(Boolean);

  return (
    <article className="graph-ref-card">
      <strong className="mono break-anywhere">{objectRef.objectId}</strong>
      <span className="graph-ref-card__meta">
        v{objectRef.version || "-"}
        {objectRef.digest ? ` • ${shortHash(objectRef.digest, 8, 6)}` : ""}
      </span>
      {tags.length > 0 ? <span className="graph-ref-card__tag">{tags.join(" • ")}</span> : null}
    </article>
  );
}

function GraphChangeCard({ value }: { value: unknown }) {
  const record = asRecord(value);
  const nextRef = readObjectRef(record.object_ref ?? record);
  const prevRef = record.previous_object_ref ? readObjectRef(record.previous_object_ref) : null;
  const owner = readObjectOwner(record);
  const previousOwner = (() => {
    const previous = record.previous_owner;
    if (typeof previous === "string") return previous;
    if (previous && typeof previous === "object" && !Array.isArray(previous)) {
      const previousRecord = asRecord(previous);
      const addressOwner = previousRecord.AddressOwner;
      if (typeof addressOwner === "string") return `owned by ${shortHash(addressOwner, 10, 6)}`;
      if ("Shared" in previousRecord) return "shared";
      if ("Immutable" in previousRecord) return "immutable";
    }
    return "";
  })();
  const typeName = readString(record, "type_", "");
  const previousVersion = readString(record, "previous_version", "");
  const changeType = readChangeType(record);

  return (
    <article className={`graph-change-card graph-change-card--${changeType}`}>
      <div className="graph-change-card__top">
        <span className="graph-change-card__label">{changeType}</span>
        <span className="mono graph-change-card__version">
          {previousVersion ? `v${previousVersion} -> ` : ""}v{nextRef.version || "-"}
        </span>
      </div>
      <strong className="mono break-anywhere">{nextRef.objectId}</strong>
      {typeName ? <span className="graph-change-card__type break-anywhere">{typeName}</span> : null}
      {prevRef ? (
        <span className="graph-change-card__meta">
          prev {prevRef.version || "-"}
          {prevRef.digest ? ` • ${shortHash(prevRef.digest, 8, 6)}` : ""}
        </span>
      ) : null}
      {owner || previousOwner ? (
        <span className="graph-change-card__owner">
          {previousOwner ? `${previousOwner} -> ` : ""}
          {owner || "unknown owner"}
        </span>
      ) : null}
    </article>
  );
}

function GraphEdgeRow({ value }: { value: unknown }) {
  const record = asRecord(value);
  const source = readObjectRef(record.source_object_ref);
  const target = readObjectRef(record.target_object_ref);
  const relation = readRelation(record);

  return (
    <div className="graph-edge-row">
      <div className="graph-edge-row__node">
        <p className="tiny-label">Source</p>
        <strong className="mono break-anywhere">{source.objectId}</strong>
        <span className="graph-edge-row__meta">v{source.version || "-"}</span>
      </div>
      <div className="graph-edge-row__relation">
        <span>{relation}</span>
      </div>
      <div className="graph-edge-row__node">
        <p className="tiny-label">Target</p>
        <strong className="mono break-anywhere">{target.objectId}</strong>
        <span className="graph-edge-row__meta">v{target.version || "-"}</span>
      </div>
    </div>
  );
}

export default function ObjectGraphView({
  title,
  subtitle,
  objectInputs,
  sharedInputs,
  immutableInputs,
  gasObjects,
  objectChanges,
  graphEdges,
}: ObjectGraphViewProps) {
  const lanes = [
    { items: asArray(objectInputs), label: "Owned Inputs" },
    { items: asArray(sharedInputs), label: "Shared Inputs" },
    { items: asArray(immutableInputs), label: "Immutable Inputs" },
    { items: asArray(gasObjects), label: "Gas Objects" },
  ].filter((lane) => lane.items.length > 0);

  const changes = asArray(objectChanges);
  const groupedChanges = [
    { items: changes.filter((entry) => readChangeType(entry) === "created"), label: "Created" },
    { items: changes.filter((entry) => readChangeType(entry) === "mutated"), label: "Mutated" },
    { items: changes.filter((entry) => readChangeType(entry) === "transferred"), label: "Transferred" },
    { items: changes.filter((entry) => readChangeType(entry) === "deleted"), label: "Deleted" },
  ].filter((group) => group.items.length > 0);
  const edges = asArray(graphEdges);

  if (lanes.length === 0 && groupedChanges.length === 0 && edges.length === 0) {
    return null;
  }

  return (
    <section className="object-graph-panel">
      <div className="panel-head object-graph-panel__head">
        <div>
          <h3 className="panel-title">{title}</h3>
          {subtitle ? <p className="panel-subtitle">{subtitle}</p> : null}
        </div>
      </div>

      {lanes.length > 0 ? (
        <div className="graph-lane-list">
          {lanes.map((lane) => (
            <div className="graph-lane" key={lane.label}>
              <div className="graph-lane__head">
                <span className="graph-lane__title">{lane.label}</span>
                <span className="graph-lane__count mono">{lane.items.length}</span>
              </div>
              <div className="graph-lane__items">
                {lane.items.map((entry, index) => (
                  <GraphRefCard key={`${lane.label}-${index}-${readObjectRef(entry).objectId}`} value={entry} />
                ))}
              </div>
            </div>
          ))}
        </div>
      ) : null}

      {groupedChanges.length > 0 ? (
        <div className="graph-change-groups">
          {groupedChanges.map((group) => (
            <section className="graph-change-group" key={group.label}>
              <div className="graph-lane__head">
                <span className="graph-lane__title">{group.label}</span>
                <span className="graph-lane__count mono">{group.items.length}</span>
              </div>
              <div className="graph-change-group__items">
                {group.items.map((entry, index) => (
                  <GraphChangeCard key={`${group.label}-${index}-${readObjectRef(entry).objectId}`} value={entry} />
                ))}
              </div>
            </section>
          ))}
        </div>
      ) : null}

      {edges.length > 0 ? (
        <div className="graph-edge-list">
          <div className="graph-lane__head">
            <span className="graph-lane__title">Dependency Edges</span>
            <span className="graph-lane__count mono">{edges.length}</span>
          </div>
          <div className="graph-edge-list__items">
            {edges.map((entry, index) => (
              <GraphEdgeRow key={`edge-${index}-${readObjectRef(asRecord(entry).source_object_ref).objectId}`} value={entry} />
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}
