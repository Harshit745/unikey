import React from 'react';
import { Button } from '@/components/ui/Button';

interface Props {
  children: React.ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Top-level error boundary. A render error anywhere in the tree shows this
 * message with a working retry instead of a blank window (which previously
 * read as an app crash).
 */
export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('[unikey] render error:', error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="grid h-full place-items-center p-8">
          <div className="surface-card w-full max-w-md p-6 text-center">
            <p className="text-[17px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
              Something went wrong
            </p>
            <p className="mt-1 text-[13px] leading-snug tracking-[-0.01em] text-black/50 dark:text-white/50">
              The UI hit an unexpected error while rendering.
            </p>
            <pre className="mt-4 max-h-40 overflow-auto rounded-xl bg-black/[0.04] p-3 text-left text-[11.5px] leading-relaxed text-black/55 dark:bg-white/[0.06] dark:text-white/55">
              {this.state.error.message}
            </pre>
            <Button className="mt-5" onClick={() => this.setState({ error: null })}>
              Try again
            </Button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
