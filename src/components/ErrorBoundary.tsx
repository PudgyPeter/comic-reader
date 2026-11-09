import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Unhandled UI error", error, info.componentStack);
  }

  private reset = () => {
    this.setState({ error: null });
  };

  private reload = () => {
    window.location.hash = "#/";
    window.location.reload();
  };

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="grid h-full place-items-center px-6">
        <div className="w-full max-w-lg rounded-2xl border border-[var(--line)] bg-[var(--panel)] p-6">
          <h1 className="font-display text-lg font-semibold tracking-tight">
            Something went wrong
          </h1>
          <p className="mt-1.5 text-[13px] leading-relaxed text-muted">
            The screen failed to render. Your library and reading progress are stored on disk
            and are unaffected.
          </p>
          <pre className="mt-3 max-h-40 overflow-auto rounded-lg bg-[var(--surface-2)] p-3 text-[11px] leading-relaxed text-[var(--text)]">
            {error.message || String(error)}
          </pre>
          <div className="mt-4 flex items-center gap-2">
            <button className="btn btn-sm btn-primary" onClick={this.reload}>
              Back to library
            </button>
            <button className="btn btn-sm" onClick={this.reset}>
              Try again
            </button>
          </div>
        </div>
      </div>
    );
  }
}
