import React, { useEffect, useState } from 'react';
import { motion } from 'motion/react';
import { useNavigate } from 'react-router-dom';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api, subscribeEvents } from '@/api/client';
import type { ServicesResponse, SessionInfo, TransferInfo, Envelope } from '@/api/types';
import { StatCard } from '@/components/ui/StatCard';
import { GlassCard, IconTile } from '@/components/ui/primitives';
import { Table, Th, Td, Tr } from '@/components/ui/table';
import { Badge } from '@/components/ui/Badge';
import { EmptyState } from '@/components/ui/controls';
import { useAppearance } from '@/context/AppearanceContext';
import { Button } from '@/components/ui/Button';
import {
  Activity,
  Wrench,
  Monitor,
  FolderUp,
  Link,
  ClipboardList,
  ArrowUpRight,
  CheckCircle,
  XCircle,
  Square,
  KeyRound,
  PlusCircle,
  MinusCircle,
  Settings,
  Radio,
} from 'lucide-react';

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ['KB', 'MB', 'GB'];
  let v = n;
  let u = -1;
  do { v /= 1024; u++; } while (v >= 1024 && u < units.length - 1);
  return `${v.toFixed(v >= 10 ? 0 : 1)} ${units[u]}`;
}

/** ISO timestamp → "HH:MM:SS" for the activity feed. */
function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

/** A single row of the live activity feed. */
function describeEvent(env: Envelope): { icon: React.ReactNode; tint: string; text: string } {
  const str = (v: unknown) => (typeof v === 'string' ? v : '');
  switch (env.type) {
    case 'session_connected':
      return {
        icon: <Link className="h-4 w-4" />,
        tint: 'bg-indigo-500/15 text-indigo-600 dark:bg-indigo-400/20 dark:text-indigo-300',
        text: `Connected to ${str(env.peer_name) || 'a device'}`,
      };
    case 'session_disconnected':
      return {
        icon: <Link className="h-4 w-4" />,
        tint: 'bg-slate-500/15 text-slate-600 dark:bg-slate-400/20 dark:text-slate-300',
        text: `Disconnected from ${str(env.peer_name) || 'a device'}`,
      };
    case 'clipboard_synced':
      return {
        icon: <ClipboardList className="h-4 w-4" />,
        tint: 'bg-teal-500/15 text-teal-600 dark:bg-teal-400/20 dark:text-teal-300',
        text: `Clipboard ${env.direction === 'in' ? 'received' : 'synced'} (${str(env.content_type) || 'text'})`,
      };
    case 'transfer_started':
      return {
        icon: <ArrowUpRight className="h-4 w-4" />,
        tint: 'bg-sky-500/15 text-sky-600 dark:bg-sky-400/20 dark:text-sky-300',
        text: `Transfer started: ${str(env.filename)}`,
      };
    case 'transfer_completed':
      return {
        icon: <CheckCircle className="h-4 w-4" />,
        tint: 'bg-emerald-500/15 text-emerald-600 dark:bg-emerald-400/20 dark:text-emerald-300',
        text: `Transfer complete: ${str(env.filename)}`,
      };
    case 'transfer_failed':
      return {
        icon: <XCircle className="h-4 w-4" />,
        tint: 'bg-rose-500/15 text-rose-600 dark:bg-rose-400/20 dark:text-rose-300',
        text: `Transfer failed: ${str(env.error).slice(0, 60)}`,
      };
    case 'transfer_cancelled':
      return {
        icon: <Square className="h-4 w-4" />,
        tint: 'bg-amber-500/15 text-amber-600 dark:bg-amber-400/20 dark:text-amber-300',
        text: 'Transfer cancelled',
      };
    case 'pairing_completed':
      return {
        icon: <KeyRound className="h-4 w-4" />,
        tint: 'bg-indigo-500/15 text-indigo-600 dark:bg-indigo-400/20 dark:text-indigo-300',
        text: 'Pairing completed',
      };
    case 'device_added':
      return {
        icon: <PlusCircle className="h-4 w-4" />,
        tint: 'bg-slate-500/15 text-slate-600 dark:bg-slate-400/20 dark:text-slate-300',
        text: 'Device added',
      };
    case 'device_removed':
      return {
        icon: <MinusCircle className="h-4 w-4" />,
        tint: 'bg-slate-500/15 text-slate-600 dark:bg-slate-400/20 dark:text-slate-300',
        text: 'Device removed',
      };
    case 'service_status':
      return {
        icon: <Settings className="h-4 w-4" />,
        tint: 'bg-slate-500/15 text-slate-600 dark:bg-slate-400/20 dark:text-slate-300',
        text: `Service ${str(env.status)}: ${str(env.name)}`,
      };
    default:
      return {
        icon: <Radio className="h-4 w-4" />,
        tint: 'bg-slate-500/15 text-slate-600 dark:bg-slate-400/20 dark:text-slate-300',
        text: `Event: ${env.type}`,
      };
  }
}

/** Transfer status → Badge tone. */
const transferTone = (status: string) =>
  status === 'Completed'
    ? 'success'
    : status === 'Failed' || status === 'Cancelled'
      ? 'error'
      : status === 'InProgress' || status === 'Queued'
        ? 'accent'
        : 'neutral';

/** Service status → Badge tone. */
const serviceTone = (status: string) =>
  status === 'running' ? 'success' : status === 'registered' ? 'neutral' : 'error';

const Dashboard: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const navigate = useNavigate();
  const { reduceMotion } = useAppearance();
  const { data: services } = usePoll<ServicesResponse>(api.services, 3000);
  const { data: sessions } = usePoll<SessionInfo[]>(api.sessions, 2500);
  const { data: transfers } = usePoll<TransferInfo[]>(api.transfers, 2500);
  const [feed, setFeed] = useState<Envelope[]>([]);

  // Subscribe to the live SSE stream once; keep the most recent 50 events.
  useEffect(() => {
    if (!daemonReachable) return;
    const stop = subscribeEvents(
      env => setFeed(prev => [env, ...prev].slice(0, 50)),
      () => {},
    );
    return stop;
  }, [daemonReachable]);

  const running = services?.services.filter(s => s.status === 'running').length ?? 0;
  const total = services?.services.length ?? 0;
  const sessionCount = sessions?.length ?? 0;
  const activeTransfers = transfers?.filter(t => t.status === 'InProgress' || t.status === 'Queued').length ?? 0;

  const cardTitle = 'px-4 pt-4 pb-3 text-[14px] font-semibold leading-tight tracking-[-0.015em] text-[var(--ink)]';
  const muted = 'px-4 pb-4 text-[12.5px] text-[var(--ink-2)]';

  return (
    <>
      {/* Stat cards */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-4">
        <StatCard
          icon={<Activity />}
          tint="bg-indigo-500/15 text-indigo-600 dark:bg-indigo-400/20 dark:text-indigo-300"
          valueNode={daemonReachable ? 'Online' : 'Offline'}
          label="Daemon status"
          detail={daemonReachable ? 'Responding normally' : 'Connection unavailable'}
          tone={daemonReachable ? 'success' : 'danger'}
          reduceMotion={reduceMotion}
        />
        <StatCard
          icon={<Wrench />}
          tint="bg-sky-500/15 text-sky-600 dark:bg-sky-400/20 dark:text-sky-300"
          value={running}
          label="Services running"
          detail={services ? `${running} of ${total} available` : 'Loading status'}
          tone={total > 0 && running === total ? 'success' : total > 0 ? 'warning' : 'neutral'}
          reduceMotion={reduceMotion}
        />
        <StatCard
          icon={<Monitor />}
          tint="bg-teal-500/15 text-teal-600 dark:bg-teal-400/20 dark:text-teal-300"
          value={sessionCount}
          label="Active sessions"
          detail={sessions ? (sessionCount ? 'Secure connections' : 'No connected peers') : 'Loading sessions'}
          reduceMotion={reduceMotion}
        />
        <StatCard
          icon={<FolderUp />}
          tint="bg-violet-500/15 text-violet-600 dark:bg-violet-400/20 dark:text-violet-300"
          value={activeTransfers}
          label="Active transfers"
          detail={transfers ? (activeTransfers ? 'In progress or queued' : 'Transfer queue clear') : 'Loading transfers'}
          reduceMotion={reduceMotion}
        />
      </div>

      {/* Sessions + live activity */}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <GlassCard className="p-0">
          <h3 className={cardTitle}>Sessions</h3>
          {!daemonReachable ? (
            <p className={muted}>Daemon offline</p>
          ) : !sessions ? (
            <p className={muted}>Loading…</p>
          ) : sessions.length === 0 ? (
            <EmptyState
              icon={<Monitor className="h-5 w-5" />}
              title="No active sessions"
              description="Pair a trusted device, then connect it on the same network."
              action={<Button size="sm" onClick={() => navigate('/pairing')}>Open pairing</Button>}
            />
          ) : (
            <Table>
              <Tr><Th>Peer</Th><Th>Platform</Th><Th>RTT</Th></Tr>
              {sessions.map(s => (
                <Tr key={s.peer_id}>
                  <Td className="font-semibold">{s.peer_name}</Td>
                  <Td className="text-[12px]">{s.platform}</Td>
                  <Td className="font-mono text-[12px]">{s.rtt_ms != null ? `${s.rtt_ms}ms` : '—'}</Td>
                </Tr>
              ))}
            </Table>
          )}
        </GlassCard>

        <GlassCard className="p-0">
          <h3 className={cardTitle}>Live activity</h3>
          {feed.length === 0 ? (
            <EmptyState
              icon={<Radio className="h-5 w-5" />}
              title={daemonReachable ? 'Waiting for activity' : 'Daemon offline'}
              description={daemonReachable ? 'Session, clipboard, and file events will appear here as they happen.' : 'Activity resumes automatically after the daemon reconnects.'}
            />
          ) : (
            <div className="flex flex-col gap-0.5 p-2">
              {feed.map((env, i) => {
                const { icon, tint, text } = describeEvent(env);
                return (
                  <motion.div
                    key={`${env.timestamp}-${i}`}
                    whileTap={{ scale: 0.995 }}
                    className="flex items-center gap-3 rounded-xl px-2.5 py-2 transition-colors duration-150 hover:bg-black/[0.03] dark:hover:bg-white/[0.04]"
                  >
                    <IconTile icon={icon} tint={tint} size="sm" />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-[13px] font-medium leading-tight tracking-[-0.01em] text-neutral-900 dark:text-white">
                        {text}
                      </p>
                    </div>
                    <span className="shrink-0 text-[11.5px] font-medium tabular-nums tracking-[-0.01em] text-black/40 dark:text-white/40">
                      {formatTime(env.timestamp)}
                    </span>
                  </motion.div>
                );
              })}
            </div>
          )}
        </GlassCard>
      </div>

      {/* Recent transfers */}
      <GlassCard className="p-4">
        <h3 className={cardTitle}>Recent transfers</h3>
        {!transfers || transfers.length === 0 ? (
          <EmptyState
            icon={<FolderUp className="h-5 w-5" />}
            title="No transfers yet"
            description="Transfers will appear here as you send and receive files."
            action={<Button size="sm" variant="secondary" onClick={() => navigate('/files')}>Open files</Button>}
          />
        ) : (
          <Table>
            <Tr><Th>File</Th><Th>Status</Th><Th>Size</Th></Tr>
            {transfers.slice(0, 5).map(t => (
              <Tr key={t.transfer_id}>
                <Td className="max-w-[260px] truncate font-semibold">{t.filename}</Td>
                <Td><Badge tone={transferTone(t.status)}>{t.status}</Badge></Td>
                <Td className="font-mono text-[12px]">{formatBytes(t.transferred)} / {formatBytes(t.total_size)}</Td>
              </Tr>
            ))}
          </Table>
        )}
      </GlassCard>

      {/* Services */}
      <GlassCard className="p-4">
        <h3 className={cardTitle}>Services</h3>
        {!services ? (
          <EmptyState
            icon={<Wrench className="h-5 w-5" />}
            title={daemonReachable ? 'Loading services…' : 'Daemon offline'}
            description={daemonReachable ? undefined : 'Connect to view service status.'}
          />
        ) : (
          <Table>
            <Tr><Th>Service</Th><Th>Phase</Th><Th>Status</Th></Tr>
            {services.services.map(s => (
              <Tr key={s.name}>
                <Td className="font-semibold">{s.name}</Td>
                <Td className="text-[12px]">{s.phase}</Td>
                <Td><Badge tone={serviceTone(s.status)}>{s.status}</Badge></Td>
              </Tr>
            ))}
          </Table>
        )}
      </GlassCard>
    </>
  );
};

export default Dashboard;
