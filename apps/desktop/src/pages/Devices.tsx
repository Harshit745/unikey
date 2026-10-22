import React, { useCallback, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { DeviceInfo, PeerInfo, TrustedDevice } from '@/api/types';
import { GlassCard, SectionHeader } from '@/components/ui/primitives';
import { DeviceCard, type DeviceStatus } from '@/components/ui/DeviceCard';
import { Table, Th, Td, Tr } from '@/components/ui/table';
import { Badge } from '@/components/ui/Badge';
import { EmptyState } from '@/components/ui/controls';
import { Monitor, Smartphone, Laptop, Apple, Wifi, Plus } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';

/** Render a wall-clock epoch-millis timestamp as a short relative age. */
function formatLastSeen(ms: number): string {
  const diff = Math.max(0, Date.now() - ms);
  if (diff < 1_000) return 'just now';
  const secs = Math.floor(diff / 1_000);
  if (secs < 60) return `${secs}s ago`;
  return `${Math.floor(secs / 60)}m ago`;
}

/** Platform string → lucide icon + tint for the DeviceCard tile. */
function platformMeta(platform: string): { icon: React.ReactNode; tint: string } {
  const p = platform.toLowerCase();
  if (p.includes('mac') || p.includes('darwin') || p.includes('ios')) {
    return {
      icon: <Apple className="h-5 w-5" />,
      tint: 'bg-neutral-500/15 text-neutral-600 dark:bg-neutral-400/20 dark:text-neutral-300',
    };
  }
  if (p.includes('android')) {
    return {
      icon: <Smartphone className="h-5 w-5" />,
      tint: 'bg-teal-500/15 text-teal-600 dark:bg-teal-400/20 dark:text-teal-300',
    };
  }
  if (p.includes('window')) {
    return {
      icon: <Laptop className="h-5 w-5" />,
      tint: 'bg-sky-500/15 text-sky-600 dark:bg-sky-400/20 dark:text-sky-300',
    };
  }
  if (p.includes('linux')) {
    return {
      icon: <Laptop className="h-5 w-5" />,
      tint: 'bg-amber-500/15 text-amber-600 dark:bg-amber-400/20 dark:text-amber-300',
    };
  }
  return {
    icon: <Monitor className="h-5 w-5" />,
    tint: 'bg-indigo-500/15 text-indigo-600 dark:bg-indigo-400/20 dark:text-indigo-300',
  };
}

/** Trusted device `last_seen` ISO string → DeviceStatus. */
function trustedStatus(lastSeen: string): DeviceStatus {
  const ms = new Date(lastSeen).getTime();
  if (Number.isNaN(ms)) return 'away';
  const diff = Date.now() - ms;
  if (diff < 5 * 60_000) return 'connected';
  if (diff < 30 * 60_000) return 'idle';
  return 'away';
}

const Devices: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const navigate = useNavigate();
  // Poll the daemon's own identity so the Host row is always current.
  const { data: device } = usePoll<DeviceInfo>(api.device, 3000);
  // Poll the live peer registry — peers appear as they're discovered and
  // disappear automatically once the daemon prunes them offline.
  const { data: peers } = usePoll<PeerInfo[]>(api.peers, 2500);
  // Trusted (previously paired) devices — removal revokes all access.
  const { data: trusted, refetch: refetchTrusted } = usePoll<TrustedDevice[]>(api.trustedDevices, 5000);

  const [forgetting, setForgetting] = useState<string | null>(null);

  const handleForget = useCallback(
    async (id: string, name: string) => {
      if (forgetting) return;
      if (!window.confirm(`Remove "${name}" from trusted devices?\nIt will need to pair again to connect.`)) return;
      setForgetting(id);
      try {
        await api.removeDevice(id);
        refetchTrusted();
      } catch (e: any) {
        window.alert(`Failed to remove device: ${e?.message}`);
      } finally {
        setForgetting(null);
      }
    },
    [forgetting, refetchTrusted],
  );

  // Online peers float to the top, same as before.
  const sortedPeers = useMemo(
    () => [...(peers ?? [])].sort((a, b) => (b.online ? 1 : 0) - (a.online ? 1 : 0)),
    [peers],
  );

  return (
    <>
      <GlassCard className="p-6">
        <SectionHeader title="This device" description="The machine you're using UniKey on" />
        {!device ? (
          <p className="text-[13px] tracking-[-0.01em] text-black/45 dark:text-white/45">
            {daemonReachable ? 'Loading…' : 'Daemon offline.'}
          </p>
        ) : (
          <Table>
            <tbody>
              <Tr>
                <Th className="w-44">Device ID</Th>
                <Td className="font-mono text-[12px] text-black/55 dark:text-white/55">{device.id}</Td>
              </Tr>
              <Tr>
                <Th>Name</Th>
                <Td className="font-medium text-neutral-900 dark:text-white">{device.name}</Td>
              </Tr>
              <Tr>
                <Th>Platform</Th>
                <Td className="text-black/55 dark:text-white/55">{device.platform}</Td>
              </Tr>
              <Tr>
                <Th>Type</Th>
                <Td className="text-black/55 dark:text-white/55">{device.device_type}</Td>
              </Tr>
              <Tr>
                <Th>Protocol</Th>
                <Td className="font-mono text-[12px] text-black/55 dark:text-white/55">{device.protocol_version}</Td>
              </Tr>
              <Tr>
                <Th>Created</Th>
                <Td className="text-[12px] text-black/45 dark:text-white/45">
                  {new Date(device.created_at).toLocaleString()}
                </Td>
              </Tr>
            </tbody>
          </Table>
        )}
      </GlassCard>

      <GlassCard className="p-6">
        <SectionHeader title="Trusted devices" description="Machines you've paired with" />
        {!trusted ? (
          <p className="text-[13px] tracking-[-0.01em] text-black/45 dark:text-white/45">
            {daemonReachable ? 'Loading…' : 'Daemon offline.'}
          </p>
        ) : trusted.length === 0 ? (
          <EmptyState
            icon={<Monitor className="h-5 w-5" />}
            title="No trusted devices yet"
            description="Use the Pairing page to pair your first device."
          />
        ) : (
          <motion.div layout className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
            <AnimatePresence>
              {trusted.map(d => {
                const { icon, tint } = platformMeta(d.platform);
                return (
                  <motion.div
                    key={d.id}
                    layout
                    initial={{ opacity: 0, scale: 0.96 }}
                    animate={{ opacity: 1, scale: 1 }}
                    exit={{ opacity: 0, scale: 0.95 }}
                    transition={{ type: 'spring', bounce: 0, duration: 0.4 }}
                  >
                    <DeviceCard
                      icon={icon}
                      tint={tint}
                      name={d.name}
                      os={d.platform}
                      status={trustedStatus(d.last_seen)}
                      latency="—"
                      role={device && d.id === device.id ? 'Host' : 'Peer'}
                      onForget={() => handleForget(d.id, d.name)}
                    />
                  </motion.div>
                );
              })}
            </AnimatePresence>
          </motion.div>
        )}
      </GlassCard>

      <GlassCard className="p-6">
        <SectionHeader title="Nearby peers" description="Machines discovered on the local network" />
        {!peers ? (
          <p className="text-[13px] tracking-[-0.01em] text-black/45 dark:text-white/45">
            {daemonReachable ? 'Loading…' : 'Daemon offline.'}
          </p>
        ) : peers.length === 0 ? (
          <EmptyState
            icon={<Wifi className="h-5 w-5" />}
            title="No UniKey devices on this network yet"
            description="Start another UniKey instance on the same Wi-Fi or LAN — each device appears here automatically with its name, platform, IP and latency."
          />
        ) : (
          <Table>
            <thead>
              <Tr>
                <Th>Peer</Th>
                <Th>Platform</Th>
                <Th>IP</Th>
                <Th>Latency</Th>
                <Th>Status</Th>
                <Th>Last seen</Th>
              </Tr>
            </thead>
            <tbody>
              {sortedPeers.map(p => (
                <Tr key={p.id}>
                  <Td className="font-semibold text-neutral-900 dark:text-white">{p.name}</Td>
                  <Td className="text-[12px] text-black/55 dark:text-white/55">{p.platform}</Td>
                  <Td className="font-mono text-[12px] text-black/55 dark:text-white/55">{p.ip}</Td>
                  <Td className="font-mono text-[12px] text-black/55 dark:text-white/55">
                    {p.latency_ms !== null && p.latency_ms !== undefined ? `${p.latency_ms} ms` : '—'}
                  </Td>
                  <Td>
                    <Badge tone={p.online ? 'success' : 'neutral'}>
                      {p.online ? 'Online' : 'Offline'}
                    </Badge>
                  </Td>
                  <Td className="text-[12px] text-black/45 dark:text-white/45">
                    {formatLastSeen(p.last_seen)}
                  </Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        )}
      </GlassCard>

      <button
        type="button"
        onClick={() => navigate('/pairing')}
        className="grid w-full place-items-center gap-2 rounded-2xl border border-dashed border-black/15 px-6 py-8 text-center transition-colors duration-150 hover:border-indigo-400/60 hover:bg-indigo-500/[0.04] dark:border-white/15 dark:hover:border-indigo-300/50"
      >
        <Plus className="mx-auto h-5 w-5 text-indigo-500 dark:text-indigo-300" />
        <p className="text-[13.5px] font-semibold leading-tight tracking-[-0.01em] text-neutral-900 dark:text-white">
          Pair a new device
        </p>
        <p className="text-[12px] text-black/45 dark:text-white/45">Enter or generate a pairing code</p>
      </button>
    </>
  );
};

export default Devices;
