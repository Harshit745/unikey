import React, { useCallback } from 'react';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { ClipboardEntry } from '@/api/types';
import { Placeholder } from '@/components/Placeholder';
import { GlassCard } from '@/components/ui/primitives';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { EmptyState } from '@/components/ui/controls';
import { Table, Th, Td, Tr } from '@/components/ui/table';
import { ClipboardList, RefreshCw } from 'lucide-react';

function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

const Clipboard: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const fetcher = useCallback(() => api.clipboardHistory(), []);
  const { data: entries, refetch } = usePoll<ClipboardEntry[]>(fetcher, 2500);

  return (
    <>
      {!daemonReachable ? (
        <Placeholder
          icon="📋"
          title="Daemon offline"
          description="Start the daemon to view clipboard entries."
        />
      ) : !entries ? (
        <GlassCard className="flex flex-col items-center justify-center gap-2 py-12 text-center">
          <div className="spinner" />
          <p className="text-[13px] text-black/45 dark:text-white/45">Loading clipboard history…</p>
        </GlassCard>
      ) : entries.length === 0 ? (
        <GlassCard className="p-0">
          <EmptyState
            icon={<ClipboardList className="h-5 w-5" />}
            title="No clipboard entries yet"
            description="Copy something on a paired device — it will appear here."
          />
        </GlassCard>
      ) : (
        <GlassCard className="p-0">
          <div className="flex items-center justify-between gap-4 px-5 pt-5">
            <h3 className="text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
              Clipboard history
            </h3>
            <Button variant="secondary" size="sm" onClick={refetch}>
              <RefreshCw className="h-3.5 w-3.5" />
              Refresh
            </Button>
          </div>
          <Table>
            <thead>
              <Tr>
                <Th>Time</Th>
                <Th>Direction</Th>
                <Th>Type</Th>
                <Th>Content</Th>
              </Tr>
            </thead>
            <tbody>
              {entries.map((e, i) => (
                <Tr key={i}>
                  <Td className="whitespace-nowrap font-mono text-[12px] text-black/45 dark:text-white/45">
                    {formatTime(e.timestamp)}
                  </Td>
                  <Td>
                    <Badge tone={e.direction === 'in' ? 'neutral' : 'success'}>
                      {e.direction === 'in' ? 'Received' : 'Sent'}
                    </Badge>
                  </Td>
                  <Td className="font-mono text-[12px] text-black/55 dark:text-white/55">{e.content_type}</Td>
                  <Td className="max-w-[420px] truncate text-black/55 dark:text-white/55">
                    {e.preview || <span className="text-black/35 dark:text-white/35">(empty)</span>}
                  </Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        </GlassCard>
      )}
    </>
  );
};

export default Clipboard;
