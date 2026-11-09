import type { SmartRule, SmartRuleField, SmartRuleOp, SmartRules } from "../lib/types";
import { Icon } from "./Icon";

export const SMART_FIELDS: { value: SmartRuleField; label: string; numeric?: boolean }[] = [
  { value: "title", label: "Title" },
  { value: "series", label: "Series" },
  { value: "number", label: "Issue number" },
  { value: "publisher", label: "Publisher" },
  { value: "universe", label: "Universe" },
  { value: "writer", label: "Writer" },
  { value: "artist", label: "Artist" },
  { value: "coverArtist", label: "Cover artist" },
  { value: "edition", label: "Edition" },
  { value: "releaseDate", label: "Release date" },
  { value: "year", label: "Year", numeric: true },
  { value: "status", label: "Read status" },
  { value: "format", label: "File format" },
  { value: "pages", label: "Page count", numeric: true },
  { value: "lastPage", label: "Last read page", numeric: true },
];

export const SMART_OPS: { value: SmartRuleOp; label: string; needsValue: boolean }[] = [
  { value: "contains", label: "contains", needsValue: true },
  { value: "eq", label: "is", needsValue: true },
  { value: "ne", label: "is not", needsValue: true },
  { value: "startsWith", label: "starts with", needsValue: true },
  { value: "gt", label: "greater than", needsValue: true },
  { value: "gte", label: "at least", needsValue: true },
  { value: "lt", label: "less than", needsValue: true },
  { value: "lte", label: "at most", needsValue: true },
  { value: "isSet", label: "is set", needsValue: false },
  { value: "isEmpty", label: "is empty", needsValue: false },
];

export const EMPTY_RULES: SmartRules = { all: [], any: [] };

const STATUS_VALUES = ["unread", "in-progress", "completed"];
const FORMAT_VALUES = ["cbz", "cbr", "pdf", "folder"];

function isNumericField(field: SmartRuleField) {
  return SMART_FIELDS.find((entry) => entry.value === field)?.numeric ?? false;
}

function RuleRow({
  rule,
  onChange,
  onRemove,
}: {
  rule: SmartRule;
  onChange: (rule: SmartRule) => void;
  onRemove: () => void;
}) {
  const op = SMART_OPS.find((entry) => entry.value === rule.op) ?? SMART_OPS[0];
  const numeric = isNumericField(rule.field);
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <select
        className="input h-8 w-auto text-xs"
        value={rule.field}
        aria-label="Rule field"
        onChange={(event) => onChange({ ...rule, field: event.currentTarget.value as SmartRuleField })}
      >
        {SMART_FIELDS.map((field) => (
          <option key={field.value} value={field.value}>
            {field.label}
          </option>
        ))}
      </select>
      <select
        className="input h-8 w-auto text-xs"
        value={rule.op}
        aria-label="Rule operator"
        onChange={(event) => onChange({ ...rule, op: event.currentTarget.value as SmartRuleOp })}
      >
        {SMART_OPS.filter((entry) => numeric || !["gt", "gte", "lt", "lte"].includes(entry.value)).map(
          (entry) => (
            <option key={entry.value} value={entry.value}>
              {entry.label}
            </option>
          ),
        )}
      </select>
      {op.needsValue &&
        (rule.field === "status" || rule.field === "format" ? (
          <select
            className="input h-8 w-auto text-xs"
            value={rule.value}
            aria-label="Rule value"
            onChange={(event) => onChange({ ...rule, value: event.currentTarget.value })}
          >
            <option value="">Choose…</option>
            {(rule.field === "status" ? STATUS_VALUES : FORMAT_VALUES).map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        ) : (
          <input
            className="input h-8 w-36 text-xs"
            value={rule.value}
            inputMode={numeric ? "numeric" : "text"}
            placeholder={numeric ? "Number" : "Value"}
            aria-label="Rule value"
            onChange={(event) => onChange({ ...rule, value: event.currentTarget.value })}
          />
        ))}
      <button
        type="button"
        className="rounded p-1 text-muted hover:text-rose-300"
        aria-label="Remove rule"
        onClick={onRemove}
      >
        <Icon name="x" className="size-3.5" />
      </button>
    </div>
  );
}

export function SmartRulesEditor({
  rules,
  onChange,
}: {
  rules: SmartRules;
  onChange: (rules: SmartRules) => void;
}) {
  const group = (key: "all" | "any") => {
    const list = rules[key];
    const update = (next: SmartRule[]) => onChange({ ...rules, [key]: next });
    return (
      <div>
        <div className="mb-1.5 flex items-center justify-between">
          <span className="label mb-0">
            {key === "all" ? "Match all of these" : "Match any of these"}
          </span>
          <button
            type="button"
            className="btn btn-sm"
            onClick={() =>
              update([...list, { field: key === "all" ? "series" : "status", op: "eq", value: "" }])
            }
          >
            <Icon name="plus" className="size-3" /> Add rule
          </button>
        </div>
        {list.length === 0 ? (
          <p className="text-[10px] text-muted">
            {key === "all" ? "No filters — this group adds no restriction." : "Optional."}
          </p>
        ) : (
          <div className="space-y-1.5">
            {list.map((rule, index) => (
              <RuleRow
                key={index}
                rule={rule}
                onChange={(next) => update(list.map((entry, i) => (i === index ? next : entry)))}
                onRemove={() => update(list.filter((_, i) => i !== index))}
              />
            ))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="space-y-3">
      {group("all")}
      <div className="border-t border-[var(--line)] pt-3">{group("any")}</div>
      <p className="text-[10px] text-muted">
        Rules are typed and compiled locally: only these fields and operators are allowed, so
        collection rules can never run arbitrary queries. Both groups are combined with AND.
      </p>
    </div>
  );
}
