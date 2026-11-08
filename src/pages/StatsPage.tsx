import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { EmptyState, ProgressBar, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { formatDuration, pluralize } from "../lib/utils";

export default function StatsPage() {
  const query = useQuery({ queryKey: ["stats"], queryFn: api.stats });

  if (query.isLoading) {
    return (
      <div className="grid h-64 place-items-center">
        <Spinner />
      </div>
    );
  }
  if (query.isError || !query.data) {
    return (
      <div className="p-6">
        <EmptyState title="Statistics unavailable" body="Reading history could not be loaded." />
      </div>
    );
  }

  const stats = query.data;
  const completion = stats.totalIssues ? stats.completed / stats.totalIssues : 0;
  const maxSeries = Math.max(1, ...stats.topSeries.map((entry) => entry.total));

  const cards = [
    { label: "Issues read", value: stats.completed, hint: `of ${stats.totalIssues}`, icon: "book" },
    { label: "Pages read", value: stats.totalPagesRead, hint: "recorded page depth", icon: "layers" },
    { label: "Reading time", value: formatDuration(stats.readingSeconds), hint: "tracked sessions", icon: "clock" },
    { label: "Streak", value: `${stats.streakDays}d`, hint: "consecutive days", icon: "star" },
  ];

  return (
    <div className="mx-auto w-full max-w-[1100px] px-6 py-6">
      <h1 className="font-display text-2xl font-semibold tracking-tight">Statistics</h1>
      <p className="mt-1 text-[13px] text-muted">
        Calculated from locally stored reading history. Nothing leaves this machine.
      </p>

      <div className="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
        {cards.map((card) => (
          <div key={card.label} className="rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-4">
            <div className="flex items-center justify-between text-muted">
              <span className="text-[11px] uppercase tracking-wider">{card.label}</span>
              <Icon name={card.icon} className="size-4" />
            </div>
            <p className="mt-2 font-display text-2xl font-semibold tabular-nums">{card.value}</p>
            <p className="text-[10px] text-muted">{card.hint}</p>
          </div>
        ))}
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
        <section className="rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-4">
          <h2 className="text-sm font-semibold">Overall completion</h2>
          <p className="mt-1 text-[11px] text-muted">
            {stats.completed} completed, {stats.inProgress} in progress
          </p>
          <div className="mt-3">
            <ProgressBar value={completion} className="h-2" />
          </div>
          <p className="mt-2 font-display text-3xl font-semibold tabular-nums">
            {Math.round(completion * 100)}%
          </p>
          <p className="mt-3 text-[11px] text-muted">
            {stats.completedThisMonth} {pluralize(stats.completedThisMonth, "issue")} completed this month
          </p>
        </section>

        <section className="rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-4">
          <h2 className="text-sm font-semibold">Most-read series</h2>
          {stats.topSeries.length === 0 ? (
            <p className="mt-3 text-xs text-muted">No reading history yet.</p>
          ) : (
            <div className="mt-3 space-y-2.5">
              {stats.topSeries.map((entry) => (
                <div key={entry.name}>
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="truncate">{entry.name}</span>
                    <span className="tabular-nums text-muted">
                      {entry.read}/{entry.total}
                    </span>
                  </div>
                  <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-[var(--surface-3)]">
                    <div
                      className="h-full rounded-full bg-[var(--accent)]"
                      style={{ width: `${(entry.total / maxSeries) * 100}%` }}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>

      <section className="mt-4 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-4">
        <h2 className="text-sm font-semibold">By status</h2>
        <div className="mt-3 flex gap-6">
          {stats.byStatus.map((bucket) => (
            <div key={bucket.label}>
              <p className="font-display text-xl font-semibold tabular-nums">{bucket.count}</p>
              <p className="text-[10px] uppercase tracking-wider text-muted">{bucket.label}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
