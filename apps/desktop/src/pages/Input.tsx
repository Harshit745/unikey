import React, { useEffect, useMemo, useState } from 'react';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import { Placeholder } from '@/components/Placeholder';
import { GlassCard, SectionHeader } from '@/components/ui/primitives';
import { Button } from '@/components/ui/Button';
import { Field, Select } from '@/components/ui/controls';
import { MousePointer2, ArrowLeftRight, ShieldCheck, OctagonX } from 'lucide-react';

const labels = {
  local: 'Controlling this computer', checking_peer: 'Checking the other device…',
  armed: 'Ready at the shared edge', entering: 'Waiting for cursor handoff…',
  remote: 'Controlling the other computer', receiving: 'A paired device is controlling this cursor',
};

const Input: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const { data: status, error, refetch } = usePoll(api.cursorStatus, 1000);
  const { data: sessions } = usePoll(api.sessions, 4000);
  const { data: trusted } = usePoll(api.trustedDevices, 4000);
  const [peer, setPeer] = useState('');
  const [edge, setEdge] = useState<'left' | 'right'>('right');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const targets = useMemo(() => (sessions ?? []).filter(s => trusted?.some(t => t.id === s.peer_id)), [sessions, trusted]);
  useEffect(() => {
    if (!peer && targets.length === 1) setPeer(targets[0].peer_id);
    else if (peer && !targets.some(t => t.peer_id === peer)) setPeer('');
  }, [peer, targets]);
  const act = async (action: () => Promise<unknown>) => {
    setBusy(true); setNotice(null);
    try { await action(); await refetch(); }
    catch (e) { setNotice(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };
  if (!daemonReachable) return <Placeholder icon="🖱️" title="Daemon offline" description="Start the daemon to share your cursor." />;
  const sharing = !!status && ['checking_peer', 'armed', 'entering', 'remote'].includes(status.phase);
  return <>
    <GlassCard className="p-6">
      <div className="flex items-start gap-3">
        <MousePointer2 className="mt-1 h-5 w-5 shrink-0 text-[var(--accent)]" />
        <div>
          <h2 className="text-[17px] font-semibold tracking-tight text-[var(--ink)]">One cursor, two computers</h2>
          <p className="mt-1 max-w-xl text-[13px] leading-relaxed text-[var(--ink-2)]">
            Move to the shared screen edge to control your paired computer. Move back across the same edge to return.
            Movement, clicks, dragging within the remote computer, and scrolling follow your mouse.
          </p>
        </div>
      </div>
      <div role="status" aria-live="polite" className="mt-5 flex items-center gap-2 border-t border-[var(--hairline)] pt-4 text-[13px] text-[var(--ink)]">
        <span className={`h-2 w-2 rounded-full ${status && !error ? 'bg-emerald-500' : 'bg-amber-500'}`} />
        {error ? (error === 'HTTP 404'
          ? 'The running daemon has no shared cursor API. Fully quit UniKey and its old daemon, then reopen the updated app.'
          : `Cannot read shared cursor status (${error}). Check the daemon connection.`)
          : status ? labels[status.phase] : 'Reading cursor status…'}
      </div>
      {(notice || status?.error) && <p role="alert" className="mt-3 text-[13px] text-rose-600 dark:text-rose-300">{notice ?? status?.error}</p>}
    </GlassCard>

    <GlassCard className="p-6">
      <SectionHeader title="Receive control" description="Enable this on the computer you want to control. Only paired devices with an active encrypted connection can take ownership." />
      <div className="mt-4 flex flex-wrap items-center justify-between gap-4">
        <p className="flex items-center gap-2 text-[13px] text-[var(--ink-2)]"><ShieldCheck className="h-4 w-4" />
          {status?.receiving_enabled ? 'Receiving is allowed for this app session' : 'Receiving is off'}
        </p>
        <Button variant="secondary" disabled={busy || !status?.supported || !!error} onClick={() => act(() => api.cursorReceiving(!status?.receiving_enabled))}>
          {status?.receiving_enabled ? 'Disable receiving' : 'Allow paired cursor'}
        </Button>
      </div>
    </GlassCard>

    <GlassCard className="p-6">
      <SectionHeader title="Share this mouse" description="First allow receiving on the other computer, then choose where it sits beside this display." />
      <div className="mt-4 grid items-end gap-4 sm:grid-cols-[1fr_150px_auto]">
        <Field label="Paired computer"><Select value={peer} onChange={e => setPeer(e.target.value)} disabled={sharing || busy}>
          <option value="">Select a connected device</option>
          {targets.map(t => <option key={t.peer_id} value={t.peer_id}>{t.peer_name}</option>)}
        </Select></Field>
        <Field label="Located to your"><Select value={edge} onChange={e => setEdge(e.target.value as 'left' | 'right')} disabled={sharing || busy}>
          <option value="left">Left</option><option value="right">Right</option>
        </Select></Field>
        <Button disabled={busy || !status?.supported || !!error || (!sharing && !peer)} variant={sharing ? 'secondary' : 'primary'}
          onClick={() => act(() => sharing ? api.cursorStop() : api.cursorStart(peer, edge))}>
          <ArrowLeftRight className="h-4 w-4" />{sharing ? 'Stop sharing' : 'Start sharing'}
        </Button>
      </div>
      {targets.length === 0 && <p className="mt-3 text-[12px] text-[var(--ink-2)]">Pair a device and keep it connected to choose it here.</p>}
      <div className="mt-5 flex items-start gap-2 border-t border-[var(--hairline)] pt-4 text-[12px] leading-relaxed text-[var(--ink-2)]">
        <OctagonX className="mt-0.5 h-4 w-4 shrink-0" />
        <p>Press Escape on the source keyboard at any time to recover control. A lost connection returns control automatically.
          On macOS, allow UniKey under Privacy &amp; Security → Accessibility (Device Control and Data Access).
          The first version uses each computer’s main display. Keyboard sharing and dragging files between apps on different computers are separate features.</p>
      </div>
    </GlassCard>
  </>;
};
export default Input;
