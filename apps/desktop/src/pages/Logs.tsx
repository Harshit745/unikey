import React, { useCallback, useState } from 'react';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { LogsResponse } from '@/api/types';
import { useDaemon } from '@/context/DaemonContext';
import { useAppearance } from '@/context/AppearanceContext';
import { GlassCard, Segmented } from '@/components/ui/primitives';

const Logs: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const { reduceMotion } = useAppearance();
  const [tail, setTail] = useState(100);
  const fetcher = useCallback(() => api.logs(tail), [tail]);
  const { data: logs } = usePoll<LogsResponse>(fetcher, 3000);

  const classify = (line: string): string => {
    if (/\berror\b/i.test(line))   return 'err';
    if (/\bwarn(ing)?\b/i.test(line)) return 'warn';
    if (/\binfo\b/i.test(line))    return 'info';
    return '';
  };

  return (
    <GlassCard className="p-0">
      <div className="flex flex-wrap items-center justify-between gap-4 px-5 pt-5">
        <h3 className="text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
          Daemon log output
        </h3>
        <Segmented
          value={String(tail)}
          options={[50, 100, 200, 500].map(n => ({ value: String(n), label: String(n) }))}
          onChange={v => setTail(Number(v))}
          reduceMotion={reduceMotion}
          ariaLabel="Tail lines"
        />
      </div>

      {!daemonReachable ? (
        <p className="px-5 py-10 text-center text-[13px] text-black/45 dark:text-white/45">
          Daemon offline — logs unavailable.
        </p>
      ) : !logs ? (
        <div className="flex flex-col items-center gap-2 py-10 text-center">
          <div className="spinner" />
          <p className="text-[13px] text-black/45 dark:text-white/45">Loading logs…</p>
        </div>
      ) : (
        <div className="px-5 pb-5 pt-3">
          <p className="mb-2 text-[11px] tracking-[-0.01em] text-black/40 dark:text-white/40">
            {logs.file} {logs.truncated && '— (truncated)'}
          </p>
          <div className="log-viewer">
            {logs.lines.map((line, i) => (
              <div key={i} className={`log-line ${classify(line)}`}>{line}</div>
            ))}
          </div>
        </div>
      )}
    </GlassCard>
  );
};

export default Logs;
