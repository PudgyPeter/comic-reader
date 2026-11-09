import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, ProgressBar, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { cx, formatRelative, progressRatio } from "../lib/utils";

export default function HistoryPage() {
  const query = useQuery({
    queryKey: ["history"],
    queryFn: () => api.readingHistory(120),
  });

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
        <EmptyState title="History unavailable" body="Reading history could not be loaded." />
      </div>
    );
  }

  const { sessions, interrupted, recent } = query.data;

  return (
    <div className="mx-auto w-full max-w-[1100px] px-6 py-6">
      <h1 className="font-display text-2xl font-semibold tracking-tight">Reading history</h1>
      <p className="mt-1 text-[13px] text-muted">
        Sessions and recently opened issues. Recorded locally and never uploaded.
      </p>

      {interrupted.length > 0 && (
        <section className="mt-6 rounded-xl border border-[var(--accent)]/30 bg-[var(--surface-1)] p-4">
          <div className="flex items-center gap-2">
            <Icon name="clock" className="size-4 text-[var(--accent)]" />
            <h2 className="font-display text-sm font-semibold tracking-tight">
              Resume interrupted sessions
            </h2>
          </div>
          <p className="mt-1 text-[11px] text-muted">
            These sessions never closed cleanly, so the app stopped mid-read. Pick up where
            you left off.
          </p>
          <div className="mt-3 space-y-2">
            {interrupted.map((session) => (
              <div
                key={session.id}
                className="flex items-center gap-3 rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2"
              >
                <Cover
                  fileId={session.fileId}
                  title={session.title}
                  className="h-11 w-8 shrink-0"
                  width={80}
                  rounded="rounded"
                />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-[12px] font-medium">{session.title}</p>
                  <p className="truncate text-[10px] text-muted">
                    {session.seriesTitle ? `${session.seriesTitle} · ` : ""}
                    {formatRelative(session.endedAt)} · page {session.lastPage + 1}
                  </p>
                </div>
                <Link
                  to={`/reader/${session.issueId}?session=${session.id}&page=${session.lastPage + 1}`}
                  className="btn btn-sm btn-primary"
                >
                  Resume
                </Link>
              </div>
            ))}
          </div>
        </section>
      )}

      <section className="mt-6">
        <h2 className="mb-3 font-display text-sm font-semibold tracking-tight">Recently opened</h2>
        {recent.length === 0 ? (
          <EmptyState
            title="Nothing read yet"
            body="Open an issue and your progress will show up here."
          />
        ) : (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            {recent.map((issue) => (
              <Link
                key={issue.id}
                to={`/reader/${issue.id}`}
                className="group overflow-hidden rounded-xl border border-[var(--line)] bg-[var(--surface-1)] transition-colors hover:border-[var(--line-strong)]"
              >
                <div className="relative">
                  <Cover
                    fileId={issue.fileId}
                    title={issue.title}
                    className="aspect-[2/3] w-full transition-transform duration-300 group-hover:scale-[1.03]"
                  />
                  <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent p-2">
                    <ProgressBar value={progressRatio(issue)} />
                  </div>
                </div>
                <div className="p-2.5">
                  <p className="truncate text-[12px] font-medium">{issue.title}</p>
                  <p className="mt-0.5 truncate text-[10px] text-muted">
                    {issue.seriesTitle ? `${issue.seriesTitle} · ` : ""}
                    {formatRelative(issue.lastReadAt)}
                  </p>
                </div>
              </Link>
            ))}
          </div>
        )}
      </section>

      <section className="mt-8">
        <h2 className="mb-3 font-display text-sm font-semibold tracking-tight">Sessions</h2>
        {sessions.length === 0 ? (
          <EmptyState
            title="No sessions recorded"
            body="Reading sessions appear after you spend time with an issue."
          />
        ) : (
          <div className="overflow-hidden rounded-xl border border-[var(--line)]">
            {sessions.map((session, index) => (
              <Link
                key={session.id}
                to={`/reader/${session.issueId}`}
                className={cx(
                  "flex items-center gap-3 px-4 py-3 transition-colors hover:bg-[var(--surface-2)]",
                  index > 0 && "border-t border-[var(--line)]",
                )}
              >
                <div className="grid size-8 shrink-0 place-items-center rounded-lg bg-[var(--surface-2)] text-muted">
                  <Icon name="clock" className="size-4" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-[13px] font-medium">{session.title}</p>
                  <p className="truncate text-[10px] text-muted">
                    {session.seriesTitle ? `${session.seriesTitle} · ` : ""}
                    {formatRelative(session.endedAt)}
                  </p>
                </div>
                <div className="shrink-0 text-right">
                  <p className="text-[11px] tabular-nums text-muted">
                    {session.pages} page{session.pages === 1 ? "" : "s"}
                  </p>
                  <p className="text-[10px] tabular-nums text-muted">
                    to page {session.lastPage + 1}
                  </p>
                </div>

              </Link>
            ))}
          </div>
        )}
      </section>

      <section className="mt-8">
        <h2 className="mb-3 font-display text-sm font-semibold tracking-tight">Status</h2>
        <div className="flex flex-wrap gap-2">
          {(["unread", "in-progress", "completed"] as const).map((status) => {
            const count = recent.filter((issue) => issue.status === status).length;
            return (
              <span key={status} className="flex items-center gap-2 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-3 py-2">
                <StatusBadge status={status} />
                <span className="text-[11px] tabular-nums text-muted">{count}</span>
              </span>
            );
          })}
        </div>
      </section>
    </div>
  );
}
