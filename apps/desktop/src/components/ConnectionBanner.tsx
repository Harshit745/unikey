import React from 'react';
import { useDaemon } from '@/context/DaemonContext';
import { Button } from '@/components/ui/Button';

/**
 * Slim banner between the top bar and content, visible only when the
 * daemon is unreachable or we're mid-reconnect. The always-on status
 * lives in the TopBar chip; this is the actionable notice.
 */
export const ConnectionBanner: React.FC = () => {
  const { daemonReachable, health, retryConnect } = useDaemon();

  if (daemonReachable) return null;

  const connecting = health.status === 'down' && navigator.onLine;

  return (
    <div
      className={[
        'flex items-center justify-center gap-3 border-b px-6 py-2.5 text-[12.5px] font-medium tracking-[-0.01em]',
        connecting
          ? 'border-[var(--hairline)] bg-[var(--warn-soft)] text-[var(--warn)]'
          : 'border-[var(--hairline)] bg-[var(--danger-soft)] text-[var(--danger)]',
      ].join(' ')}
    >
      <span className="relative flex h-2 w-2">
        {connecting && (
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-amber-500 opacity-60" />
        )}
        <span
          className={[
            'relative inline-flex h-2 w-2 rounded-full',
            connecting ? 'bg-amber-500' : 'bg-rose-500',
          ].join(' ')}
        />
      </span>
      <span>
        {connecting
          ? 'Reconnecting to daemon…'
          : 'Daemon is offline — UI will reconnect when it starts.'}
      </span>
      {!connecting && (
        <Button variant="secondary" size="sm" onClick={retryConnect}>
          Retry
        </Button>
      )}
    </div>
  );
};
