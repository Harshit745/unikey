import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { PeerInfo } from '@/api/types';
import { Placeholder } from '@/components/Placeholder';
import { GlassCard, SectionHeader } from '@/components/ui/primitives';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Field, TextInput, Select } from '@/components/ui/controls';
import { Table, Th, Td, Tr } from '@/components/ui/table';
import { KeyRound, RefreshCw, CheckCircle2, Loader2 } from 'lucide-react';

type GenState =
  | { phase: 'idle' }
  | { phase: 'pending'; sessionId: string; code: string; expiresIn: number }
  | { phase: 'done'; deviceName: string }
  | { phase: 'error'; message: string };

const Pairing: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const { data: peers } = usePoll<PeerInfo[]>(api.peers, 3000);
  const [gen, setGen] = useState<GenState>({ phase: 'idle' });
  const [pairCode, setPairCode] = useState('');
  const [targetPeer, setTargetPeer] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [enterNotice, setEnterNotice] = useState<{ ok: boolean; text: string } | null>(null);
  const [history, setHistory] = useState<import('@/api/types').PairingEvent[]>([]);
  const pollRef = useRef<number | null>(null);

  const generate = useCallback(async () => {
    setGen({ phase: 'idle' });
    try {
      const r = await api.pairingGenerate();
      setGen({ phase: 'pending', sessionId: r.session_id, code: r.pair_code, expiresIn: r.expires_in });
    } catch (e: any) {
      setGen({ phase: 'error', message: e?.message ?? 'Failed to generate code' });
    }
  }, []);

  // While a code is displayed, poll its status until it resolves or expires.
  useEffect(() => {
    if (gen.phase !== 'pending') return;
    const sessionId = gen.sessionId;
    const started = Date.now();
    pollRef.current = window.setInterval(async () => {
      try {
        const s = await api.pairingStatus(sessionId);
        if (s.state === 'Completed' || s.state === 'Accepted') {
          window.clearInterval(pollRef.current!);
          setGen({ phase: 'done', deviceName: s.remote_device?.name ?? 'device' });
        } else if (Date.now() - started > gen.expiresIn * 1000) {
          window.clearInterval(pollRef.current!);
          setGen({ phase: 'error', message: 'Pairing code expired — generate a new one.' });
        }
      } catch {
        // transient poll error — keep polling
      }
    }, 1500);
    return () => { if (pollRef.current) window.clearInterval(pollRef.current); };
  }, [gen]);

  useEffect(() => {
    api.pairingHistory().then(h => setHistory(h)).catch(() => {});
  }, [gen.phase]);

  const initiate = useCallback(async () => {
    if (!pairCode || !targetPeer) return;
    setSubmitting(true);
    setEnterNotice(null);
    try {
      const r = await api.pairingInitiate(pairCode, targetPeer);
      if (r.success) {
        setEnterNotice({ ok: true, text: `Paired with ${r.device_name ?? 'device'}` });
        setPairCode('');
      } else {
        setEnterNotice({ ok: false, text: r.error ?? 'Pairing was rejected' });
      }
    } catch (e: any) {
      setEnterNotice({ ok: false, text: e?.message ?? 'Pairing failed' });
    } finally {
      setSubmitting(false);
    }
  }, [pairCode, targetPeer]);

  return (
    <>
      {!daemonReachable ? (
        <Placeholder
          icon="🔐"
          title="Daemon offline"
          description="Start the daemon to pair devices."
        />
      ) : (
        <>
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            {/* Generate code */}
            <GlassCard className="p-6">
              <SectionHeader
                title="Generate pairing code"
                description="Display a short numeric code on this device. The other device enters it to complete pairing."
              />

              {gen.phase === 'idle' && (
                <Button onClick={generate}>
                  <KeyRound className="h-4 w-4" />
                  Generate code
                </Button>
              )}
              {gen.phase === 'pending' && (
                <div className="flex flex-col items-center py-2 text-center">
                  <p className="select-all font-mono text-[36px] font-bold leading-none tracking-[0.2em] text-indigo-600 dark:text-indigo-300">
                    {gen.code.replace(/(\d{3})(?=\d)/g, '$1 ')}
                  </p>
                  <p className="mt-3 flex items-center gap-1.5 text-[12px] text-black/45 dark:text-white/45">
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    Waiting for the other device to enter this code…
                  </p>
                  <Button variant="secondary" size="sm" onClick={generate} className="mt-4">
                    <RefreshCw className="h-3.5 w-3.5" />
                    Regenerate
                  </Button>
                </div>
              )}
              {gen.phase === 'done' && (
                <div className="flex flex-col items-center py-2 text-center">
                  <CheckCircle2 className="mb-3 h-9 w-9 text-emerald-500 dark:text-emerald-300" />
                  <p className="text-[14px] font-semibold text-neutral-900 dark:text-white">
                    Paired with {gen.deviceName}!
                  </p>
                  <Button variant="secondary" size="sm" onClick={() => setGen({ phase: 'idle' })} className="mt-4">
                    Pair another device
                  </Button>
                </div>
              )}
              {gen.phase === 'error' && (
                <div>
                  <p className="mb-3 text-[13px] leading-snug text-rose-600 dark:text-rose-300">{gen.message}</p>
                  <Button onClick={generate}>Try again</Button>
                </div>
              )}
            </GlassCard>

            {/* Enter code */}
            <GlassCard className="p-6">
              <SectionHeader
                title="Enter pairing code"
                description="Type the numeric code shown on the other device."
              />
              <div className="flex flex-col gap-4">
                <Field label="Nearby device" hint="Only devices discovered on your local network appear here.">
                  <Select value={targetPeer} onChange={e => setTargetPeer(e.target.value)}>
                    <option value="">— select a discovered device —</option>
                    {(peers ?? []).map(p => (
                      <option key={p.id} value={p.id}>{p.name} ({p.ip})</option>
                    ))}
                  </Select>
                </Field>
                <Field label="Pairing code" hint="A 6-digit code, no spaces.">
                  <TextInput
                    type="text"
                    placeholder="e.g. 849261"
                    value={pairCode}
                    onChange={e => setPairCode(e.target.value.replace(/[^0-9]/g, '').slice(0, 6))}
                  />
                </Field>
                <Button
                  onClick={initiate}
                  disabled={!daemonReachable || pairCode.length < 6 || !targetPeer || submitting}
                >
                  {submitting ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Pairing…
                    </>
                  ) : (
                    'Submit code'
                  )}
                </Button>
                {enterNotice && (
                  <p
                    className={`text-[13px] leading-snug ${
                      enterNotice.ok ? 'text-emerald-600 dark:text-emerald-300' : 'text-rose-600 dark:text-rose-300'
                    }`}
                  >
                    {enterNotice.text}
                  </p>
                )}
              </div>
            </GlassCard>
          </div>

          {/* Pairing history */}
          <GlassCard className="p-0">
            <h3 className="px-5 pb-2 pt-5 text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
              Pairing history
            </h3>
            {history.length === 0 ? (
              <p className="px-5 pb-5 text-[13px] text-black/45 dark:text-white/45">No pairing activity yet.</p>
            ) : (
              <Table>
                <thead>
                  <Tr>
                    <Th>Event</Th>
                    <Th>Device</Th>
                    <Th>When</Th>
                  </Tr>
                </thead>
                <tbody>
                  {history.slice(0, 20).map((e, i) => (
                    <Tr key={i}>
                      <Td>
                        <Badge
                          tone={e.kind === 'Paired' ? 'success' : e.kind === 'Removed' ? 'error' : 'neutral'}
                        >
                          {e.kind}
                        </Badge>
                      </Td>
                      <Td className="font-medium text-neutral-900 dark:text-white">{e.peer_name}</Td>
                      <Td className="whitespace-nowrap font-mono text-[12px] text-black/45 dark:text-white/45">
                        {new Date(e.timestamp).toLocaleString()}
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

export default Pairing;
