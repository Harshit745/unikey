import React, { useCallback, useState } from 'react';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { TransferInfo, PeerInfo } from '@/api/types';
import { Placeholder } from '@/components/Placeholder';
import { GlassCard } from '@/components/ui/primitives';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Field, TextInput, Select, EmptyState } from '@/components/ui/controls';
import { Table, Th, Td, Tr } from '@/components/ui/table';
import { FolderUp, Send, RefreshCw, X } from 'lucide-react';

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let v = n;
  let u = -1;
  do { v /= 1024; u++; } while (v >= 1024 && u < units.length - 1);
  return `${v.toFixed(v >= 10 ? 0 : 1)} ${units[u]}`;
}

const statusTone = (status: string) =>
  status === 'Completed' ? 'success'
    : status === 'Failed' || status === 'Cancelled' ? 'error'
    : status === 'Queued' || status === 'InProgress' ? 'accent'
    : 'neutral';

const Files: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const { data: transfers, refetch: refetchTransfers } = usePoll<TransferInfo[]>(api.transfers, 2000);
  const { data: peers } = usePoll<PeerInfo[]>(api.peers, 4000);
  const [peerId, setPeerId] = useState('');
  const [path, setPath] = useState('');
  const [sending, setSending] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  const doSend = useCallback(async () => {
    if (!peerId || !path) return;
    setSending(true);
    setNotice(null);
    try {
      const id = await api.sendTransfer(peerId, path);
      setNotice({ ok: true, text: `Transfer queued (${id.slice(0, 8)}…)` });
      setPath('');
      refetchTransfers();
    } catch (e: any) {
      setNotice({ ok: false, text: e?.message ?? 'Send failed' });
    } finally {
      setSending(false);
    }
  }, [peerId, path, refetchTransfers]);

  const doCancel = useCallback(async (transferId: string) => {
    try {
      await api.cancelTransfer(transferId);
      refetchTransfers();
    } catch (e: any) {
      setNotice({ ok: false, text: e?.message ?? 'Cancel failed' });
    }
  }, [refetchTransfers]);

  // Sessions are the only valid send targets; fall back to online peers.
  const { data: sessions } = usePoll<import('@/api/types').SessionInfo[]>(api.sessions, 4000);
  const targetPeers =
    sessions && sessions.length > 0
      ? sessions.map(s => ({ id: s.peer_id, name: s.peer_name, platform: s.platform }))
      : (peers ?? []).filter(p => p.online).map(p => ({ id: p.id, name: p.name, platform: p.platform }));

  return (
    <>
      {!daemonReachable ? (
        <Placeholder
          icon="📂"
          title="Daemon offline"
          description="Start the daemon to transfer files."
        />
      ) : (
        <>
          <GlassCard className="p-6">
            <h3 className="text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
              Send a file
            </h3>
            <div className="mt-4 grid grid-cols-1 items-end gap-4 sm:grid-cols-2 lg:grid-cols-[1fr_1fr_auto]">
              <Field label="Target device">
                <Select value={peerId} onChange={e => setPeerId(e.target.value)}>
                  <option value="">— select —</option>
                  {targetPeers.map(p => (
                    <option key={p.id} value={p.id}>{p.name} ({p.platform})</option>
                  ))}
                </Select>
              </Field>
              <Field label="File path">
                <TextInput
                  type="text"
                  placeholder="/path/to/file"
                  value={path}
                  onChange={e => setPath(e.target.value)}
                />
              </Field>
              <Button onClick={doSend} disabled={sending || !peerId || !path} className="h-9">
                {sending ? (
                  <>
                    <span className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/40 border-t-white" />
                    Sending…
                  </>
                ) : (
                  <>
                    <Send className="h-4 w-4" />
                    Send
                  </>
                )}
              </Button>
            </div>
            {targetPeers.length === 0 && (
              <p className="mt-3 text-[12px] text-black/45 dark:text-white/45">
                No active session or online peer — pair and connect a device first.
              </p>
            )}
            {notice && (
              <p
                className={`mt-3 text-[13px] leading-snug ${
                  notice.ok ? 'text-emerald-600 dark:text-emerald-300' : 'text-rose-600 dark:text-rose-300'
                }`}
              >
                {notice.text}
              </p>
            )}
          </GlassCard>

          <GlassCard className="p-0">
            <div className="flex items-center justify-between gap-4 px-5 pt-5">
              <h3 className="text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
                Transfers
              </h3>
              <Button variant="secondary" size="sm" onClick={refetchTransfers}>
                <RefreshCw className="h-3.5 w-3.5" />
                Refresh
              </Button>
            </div>
            {!transfers ? (
              <div className="px-5 py-10 text-center text-[13px] text-black/45 dark:text-white/45">
                Loading transfers…
              </div>
            ) : transfers.length === 0 ? (
              <EmptyState
                icon={<FolderUp className="h-5 w-5" />}
                title="No transfers yet"
                description="Transfers will appear here once initiated from any device."
              />
            ) : (
              <Table>
                <thead>
                  <Tr>
                    <Th>File</Th>
                    <Th>Direction</Th>
                    <Th>Size</Th>
                    <Th>Progress</Th>
                    <Th>Status</Th>
                    <Th>{''}</Th>
                  </Tr>
                </thead>
                <tbody>
                  {transfers.map(t => (
                    <Tr key={t.transfer_id}>
                      <Td className="max-w-[220px] truncate font-medium text-neutral-900 dark:text-white">
                        {t.filename}
                      </Td>
                      <Td className="text-black/55 dark:text-white/55">
                        {t.direction === 'send' ? '→ Send' : '← Receive'}
                      </Td>
                      <Td className="whitespace-nowrap font-mono text-[12px] text-black/45 dark:text-white/45">
                        {formatBytes(t.transferred)} / {formatBytes(t.total_size)}
                      </Td>
                      <Td className="min-w-[140px]">
                        <div className="h-1.5 overflow-hidden rounded-full bg-black/[0.06] dark:bg-white/[0.08]">
                          <div
                            className="h-full rounded-full bg-indigo-500 transition-[width] duration-300"
                            style={{ width: `${Math.min(t.progress_percent, 100)}%` }}
                          />
                        </div>
                      </Td>
                      <Td>
                        <Badge tone={statusTone(t.status) as 'success' | 'error' | 'accent' | 'neutral'}>
                          {t.status}
                        </Badge>
                      </Td>
                      <Td>
                        {(t.status === 'Queued' || t.status === 'InProgress') && (
                          <Button variant="secondary" size="sm" onClick={() => doCancel(t.transfer_id)}>
                            <X className="h-3.5 w-3.5" />
                            Cancel
                          </Button>
                        )}
                      </Td>
                    </Tr>
                  ))}
                </tbody>
              </Table>
            )}
          </GlassCard>
        </>
      )}
    </>
  );
};

export default Files;
