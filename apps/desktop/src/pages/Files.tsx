import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { useNavigate } from 'react-router-dom';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { TransferInfo, SessionInfo, TrustedDevice } from '@/api/types';
import { Placeholder } from '@/components/Placeholder';
import { GlassCard } from '@/components/ui/primitives';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Field, TextInput, Select, EmptyState } from '@/components/ui/controls';
import { Table, Th, Td, Tr } from '@/components/ui/table';
import { FolderUp, Send, RefreshCw, X, MoveRight, ShieldCheck } from 'lucide-react';

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let v = n;
  let u = -1;
  do { v /= 1024; u++; } while (v >= 1024 && u < units.length - 1);
  return `${v.toFixed(v >= 10 ? 0 : 1)} ${units[u]}`;
}

const statusTone = (status: string) =>
  status === 'Completed' || status === 'Verified' ? 'success'
    : status === 'Failed' || status === 'Cancelled' ? 'error'
    : status === 'Queued' || status === 'InProgress' || status === 'AwaitingConfirmation' ? 'accent'
    : 'neutral';

const Files: React.FC = () => {
  const navigate = useNavigate();
  const { daemonReachable } = useDaemon();
  const { data: transfers, refetch: refetchTransfers } = usePoll<TransferInfo[]>(api.transfers, 2000);
  const { data: sessions } = usePoll<SessionInfo[]>(api.sessions, 4000);
  const { data: trustedDevices } = usePoll<TrustedDevice[]>(api.trustedDevices, 4000);
  const [peerId, setPeerId] = useState('');
  const [path, setPath] = useState('');
  const [sending, setSending] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [edge, setEdge] = useState<'left' | 'right'>('right');
  const [edgeCaptureEnabled, setEdgeCaptureEnabled] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  const trustedIds = useMemo(
    () => new Set((trustedDevices ?? []).map(device => device.id)),
    [trustedDevices],
  );

  // A transport session alone is not authorization. Only paired/trusted
  // devices may be offered as file destinations.
  const targetPeers = useMemo(() => {
    return (sessions ?? [])
      .filter(session => trustedIds.has(session.peer_id))
      .map(session => ({ id: session.peer_id, name: session.peer_name, platform: session.platform }));
  }, [sessions, trustedIds]);

  useEffect(() => {
    if (!peerId && targetPeers.length === 1) setPeerId(targetPeers[0].id);
    if (peerId && !targetPeers.some(peer => peer.id === peerId)) setPeerId('');
  }, [peerId, targetPeers]);

  const sendPaths = useCallback(async (paths: string[]) => {
    if (!peerId || paths.length === 0) return;
    setSending(true);
    setNotice(null);
    try {
      for (const filePath of paths) await api.sendTransfer(peerId, filePath);
      setNotice({
        ok: true,
        text: `${paths.length} file${paths.length === 1 ? '' : 's'} sent; waiting for receiver verification.`,
      });
      setPath('');
      refetchTransfers();
    } catch (e: any) {
      setNotice({ ok: false, text: e?.message ?? 'Send failed' });
    } finally {
      setSending(false);
      setDragActive(false);
    }
  }, [peerId, refetchTransfers]);

  const doSend = useCallback(async () => {
    if (!peerId || !path) return;
    setSending(true);
    setNotice(null);
    try {
      await sendPaths([path]);
    } catch (e: any) {
      setNotice({ ok: false, text: e?.message ?? 'Send failed' });
    } finally {
      setSending(false);
    }
  }, [peerId, path, sendPaths]);

  useEffect(() => {
    if (!daemonReachable) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWindow().onDragDropEvent(event => {
      if (disposed) return;
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        setDragActive(true);
      } else if (event.payload.type === 'drop') {
        void sendPaths(event.payload.paths);
      } else {
        setDragActive(false);
      }
    }).then(stop => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch(error => {
      setNotice({ ok: false, text: `Native file drop is unavailable: ${String(error)}` });
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [daemonReachable, sendPaths]);

  const doCancel = useCallback(async (transferId: string) => {
    try {
      await api.cancelTransfer(transferId);
      refetchTransfers();
    } catch (e: any) {
      setNotice({ ok: false, text: e?.message ?? 'Cancel failed' });
    }
  }, [refetchTransfers]);

  const toggleEdgeCapture = useCallback(async () => {
    setNotice(null);
    try {
      if (edgeCaptureEnabled) {
        await invoke('edge_capture_stop');
        setEdgeCaptureEnabled(false);
        return;
      }
      if (!peerId) throw new Error('Select a paired, connected target first');
      await invoke('edge_capture_start', { peerId, edge });
      setEdgeCaptureEnabled(true);
      setNotice({
        ok: true,
        text: `Screen-edge handoff enabled on the ${edge}. Drag files to that screen edge to send them.`,
      });
    } catch (error) {
      setNotice({ ok: false, text: error instanceof Error ? error.message : String(error) });
    }
  }, [edge, edgeCaptureEnabled, peerId]);

  useEffect(() => () => {
    if (edgeCaptureEnabled) void invoke('edge_capture_stop');
  }, [edgeCaptureEnabled]);

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
              <div className="mt-4 flex items-center justify-between gap-4 rounded-xl border border-amber-500/20 bg-amber-500/[0.06] p-4">
                <div>
                  <p className="text-[13px] font-medium text-[var(--ink)]">Pair before transferring</p>
                  <p className="mt-1 text-[12px] text-[var(--ink-2)]">
                    A network connection alone cannot receive files. Pair the device to authorize encrypted transfers.
                  </p>
                </div>
                <Button variant="secondary" size="sm" onClick={() => navigate('/pairing')}>
                  <ShieldCheck className="h-3.5 w-3.5" />
                  Pair device
                </Button>
              </div>
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

          <GlassCard
            className={`relative overflow-hidden border-2 border-dashed p-8 text-center transition-colors ${
              dragActive
                ? 'border-blue-500 bg-blue-500/[0.08]'
                : 'border-black/10 bg-transparent dark:border-white/12'
            }`}
          >
            <div className="mx-auto flex h-11 w-11 items-center justify-center rounded-xl bg-blue-500/10 text-blue-600 dark:text-blue-300">
              {dragActive ? <MoveRight className="h-5 w-5" /> : <FolderUp className="h-5 w-5" />}
            </div>
            <h3 className="mt-3 text-[14px] font-semibold text-[var(--ink)]">
              {dragActive ? 'Release to send' : 'Drag files here'}
            </h3>
            <p className="mx-auto mt-1 max-w-md text-[12.5px] leading-relaxed text-[var(--ink-2)]">
              {peerId
                ? 'Drop files from Finder or Explorer. UniKey will report success only after the other device saves and verifies them.'
                : 'Select a paired, connected target device before dropping files.'}
            </p>
            {sending && <p className="mt-3 text-[12px] font-medium text-blue-600 dark:text-blue-300">Preparing transfer…</p>}
          </GlassCard>

          <GlassCard className="p-6">
            <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
              <div className="max-w-xl">
                <h3 className="text-[15px] font-semibold leading-tight tracking-[-0.02em] text-[var(--ink)]">
                  Screen-edge handoff
                </h3>
                <p className="mt-1 text-[12.5px] leading-relaxed text-[var(--ink-2)]">
                  Adds a narrow native handoff target to one screen edge, so reaching the edge with files from Finder or Explorer starts the transfer without opening UniKey first.
                </p>
              </div>
              <div className="flex items-end gap-2">
                <Field label="Shared edge">
                  <Select
                    value={edge}
                    onChange={event => setEdge(event.target.value as 'left' | 'right')}
                    disabled={edgeCaptureEnabled}
                  >
                    <option value="left">Left</option>
                    <option value="right">Right</option>
                  </Select>
                </Field>
                <Button
                  variant={edgeCaptureEnabled ? 'secondary' : 'primary'}
                  onClick={toggleEdgeCapture}
                  disabled={!edgeCaptureEnabled && !peerId}
                >
                  <MoveRight className="h-4 w-4" />
                  {edgeCaptureEnabled ? 'Disable edge' : 'Enable edge'}
                </Button>
              </div>
            </div>
            <p className="mt-3 text-[11.5px] leading-relaxed text-[var(--ink-2)]">
              Current milestone: reaching the edge transfers into the receiving device's configured UniKey folder. Continuing the held drag over arbitrary remote applications still requires native Windows and macOS bridges and is not represented as complete.
            </p>
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
                        {t.direction === 'Send' ? '→ Send' : '← Receive'}
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
                        {(t.status === 'Queued' || t.status === 'InProgress' || t.status === 'AwaitingConfirmation') && (
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
