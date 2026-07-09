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

type EdgeGroup = {
  items: unknown[];
  label: string;
  summary: string;
};

function readObjectRef(value: unknown) {
  const record = asRecord(value);
  const nested = record.object_ref;
  const source =
    nested && typeof nested === "object" && !Array.isArray(nested) ? asRecord(nested) : record;

  return {
    digest: readString(source, "digest", ""),
    objectId: readString(source, "object_id", readString(source, "id", "-")),
    version: readString(source, "version", ""),
  };
}

function readOwnerLabel(value: unknown) {
  const record = asRecord(value);
  const directOwner = record.owner;
  if (typeof directOwner === "string" && directOwner) {
    return `owned by ${shortHash(directOwner, 10, 6)}`;
  }

  const ownerKind = record.owner_kind;
  if (typeof ownerKind === "string" && ownerKind) {
    return ownerKind.replace(/_/g, " ");
  }

  if (directOwner && typeof directOwner === "object" && !Array.isArray(directOwner)) {
    const ownerRecord = asRecord(directOwner);
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

function humanizeEnumLabel(value: string) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/_/g, " ")
    .trim();
}

function readRelationKey(value: unknown) {
  return readString(value, "relation", "unknown");
}

function readRelationLabel(value: unknown) {
  return humanizeEnumLabel(readRelationKey(value));
}

function readBooleanTag(value: unknown, key: string, label: string) {
  const item = asRecord(value)[key];
  return item === true ? label : "";
}

function classifyEdge(value: unknown) {
  const relation = readRelationKey(value);

  if (relation.startsWith("SharedInput")) {
    return {
      label: "Shared Input Dependencies",
      summary: "Shared object inputs that drove downstream create/mutate/delete/transfer effects.",
    };
  }

  if (relation.startsWith("ImmutableInput")) {
    return {
      label: "Immutable Input Dependencies",
      summary: "Immutable object reads that still influenced resulting object state transitions.",
    };
  }

  if (relation.startsWith("Input")) {
    return {
      label: "Owned Input Dependencies",
      summary: "Owned object inputs connected to the exact changes they caused.",
    };
  }

  if (relation.startsWith("Gas")) {
    return {
      label: "Gas Dependencies",
      summary: "Gas payment objects and the objects they funded or mutated during execution.",
    };
  }

  if (relation === "OwnershipTransfer") {
    return {
      label: "Ownership Transfers",
      summary: "Explicit ownership moves between previous and next object states.",
    };
  }

  if (relation === "VersionSuccessor" || relation === "Delete") {
    return {
      label: "Version Lineage",
      summary: "Version successors and deletions across the object lifecycle.",
    };
  }

  if (relation === "CallContextCreate") {
    return {
      label: "Call Context Creates",
      summary: "Objects created directly from the execution call context.",
    };
  }

  return {
    label: "Other Dependencies",
    summary: "Additional causal edges reported by transaction effects.",
  };
}

function buildEdgeGroups(edges: unknown[]): EdgeGroup[] {
  const groups = new Map<string, EdgeGroup>();

  edges.forEach((entry) => {
    const group = classifyEdge(entry);
    const existing = groups.get(group.label);
    if (existing) {
      existing.items.push(entry);
      return;
    }

    groups.set(group.label, {
      items: [entry],
      label: group.label,
      summary: group.summary,
    });
  });

  return [...groups.values()];
}

function GraphRefCard({ value }: { value: unknown }) {
  const objectRef = readObjectRef(value);
  const owner = readOwnerLabel(value);
  const mutableTag = readBooleanTag(value, "mutable", "mutable");
  const tags = [owner, mutableTag].filter(Boolean);

  return (
    <article className="graph-ref-card">
      <strong className="mono break-anywhere">{objectRef.objectId}</strong>
      <span className="graph-ref-card__meta">
        v{objectRef.version || "-"}
        {objectRef.digest ? ` | ${shortHash(objectRef.digest, 8, 6)}` : ""}
      </span>
      {tags.length > 0 ? <span className="graph-ref-card__tag">{tags.join(" | ")}</span> : null}
    </article>
  );
}

function GraphChangeCard({ value }: { value: unknown }) {
  const record = asRecord(value);
  const nextRef = readObjectRef(record.object_ref ?? record);
  const prevRef = record.previous_object_ref ? readObjectRef(record.previous_object_ref) : null;
  const owner = readOwnerLabel(record);
  const previousOwner = readOwnerLabel({ owner: record.previous_owner });
  const typeName = readString(record, "type_", readString(record, "type", ""));
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
          prev v{prevRef.version || "-"}
          {prevRef.digest ? ` | ${shortHash(prevRef.digest, 8, 6)}` : ""}
        </span>
      ) : null}
      {nextRef.digest ? <span className="graph-change-card__meta">digest {shortHash(nextRef.digest, 8, 6)}</span> : null}
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
  const relation = readRelationLabel(record);

  return (
    <div className="graph-edge-row">
      <div className="graph-edge-row__node">
        <p className="tiny-label">Source</p>
        <strong className="mono break-anywhere">{source.objectId}</strong>
        <span className="graph-edge-row__meta">
          v{source.version || "-"}
          {source.digest ? ` | ${shortHash(source.digest, 8, 6)}` : ""}
        </span>
      </div>
      <div className="graph-edge-row__relation">
        <span>{relation}</span>
      </div>
      <div className="graph-edge-row__node">
        <p className="tiny-label">Target</p>
        <strong className="mono break-anywhere">{target.objectId}</strong>
        <span className="graph-edge-row__meta">
          v{target.version || "-"}
          {target.digest ? ` | ${shortHash(target.digest, 8, 6)}` : ""}
        </span>
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
    {
      items: changes.filter((entry) => readChangeType(entry) === "created"),
      label: "Created",
      summary: "Fresh objects emitted by this execution path.",
    },
    {
      items: changes.filter((entry) => readChangeType(entry) === "mutated"),
      label: "Mutated",
      summary: "Existing objects whose version advanced in place.",
    },
    {
      items: changes.filter((entry) => readChangeType(entry) === "transferred"),
      label: "Transferred",
      summary: "Objects whose ownership moved to a new owner or owner kind.",
    },
    {
      items: changes.filter((entry) => readChangeType(entry) === "deleted"),
      label: "Deleted",
      summary: "Objects removed from the live object set.",
    },
  ].filter((group) => group.items.length > 0);

  const edges = asArray(graphEdges);
  const edgeGroups = buildEdgeGroups(edges);

  if (lanes.length === 0 && groupedChanges.length === 0 && edgeGroups.length === 0) {
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
              <p className="panel-subtitle">{group.summary}</p>
              <div className="graph-change-group__items">
                {group.items.map((entry, index) => (
                  <GraphChangeCard key={`${group.label}-${index}-${readObjectRef(entry).objectId}`} value={entry} />
                ))}
              </div>
            </section>
          ))}
        </div>
      ) : null}

      {edgeGroups.length > 0 ? (
        <div className="graph-change-groups">
          {edgeGroups.map((group) => (
            <section className="graph-change-group" key={group.label}>
              <div className="graph-lane__head">
                <span className="graph-lane__title">{group.label}</span>
                <span className="graph-lane__count mono">{group.items.length}</span>
              </div>
              <p className="panel-subtitle">{group.summary}</p>
              <div className="graph-edge-list__items">
                {group.items.map((entry, index) => (
                  <GraphEdgeRow
                    key={`edge-${group.label}-${index}-${readObjectRef(asRecord(entry).source_object_ref).objectId}`}
                    value={entry}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      ) : null}
    </section>
  );
}
