import React from 'react';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { DeviceInfo } from '@/api/types';
import { GlassCard } from '@/components/ui/primitives';
import { Table, Td, Tr } from '@/components/ui/table';

const About: React.FC = () => {
  const { daemonReachable, version } = useDaemon();
  const { data: device } = usePoll<DeviceInfo>(api.device, 5000);

  return (
    <>
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        {/* Shell */}
        <GlassCard className="p-0">
          <h3 className="px-5 pb-2 pt-5 text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
            Desktop Shell
          </h3>
          <Table>
            <tbody>
              <Row label="Version" value="0.1.0" />
              <Row label="Build system" value="Tauri v2 + React 18" />
              <Row label="Runtime" value={navigator.userAgent.includes('Mac') ? 'macOS' : 'Desktop'} />
            </tbody>
          </Table>
        </GlassCard>

        {/* Daemon */}
        <GlassCard className="p-0">
          <h3 className="px-5 pb-2 pt-5 text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
            Daemon
          </h3>
          {!version ? (
            <p className="px-5 py-4 text-[13px] text-black/45 dark:text-white/45">
              {daemonReachable ? 'Loading…' : 'Offline'}
            </p>
          ) : (
            <Table>
              <tbody>
                <Row label="Version" value={version.version} />
                <Row label="Build" value={version.build} />
                <Row label="Commit" value={version.commit.slice(0, 8)} />
                <Row label="Platform" value={version.platform} />
                <Row label="Rust" value={version.rustc} />
              </tbody>
            </Table>
          )}
        </GlassCard>

        {/* Device */}
        <GlassCard className="p-0 lg:col-span-2">
          <h3 className="px-5 pb-2 pt-5 text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
            Local device
          </h3>
          {!device ? (
            <p className="px-5 py-4 text-[13px] text-black/45 dark:text-white/45">
              {daemonReachable ? 'Loading…' : 'Offline'}
            </p>
          ) : (
            <Table>
              <tbody>
                <Row
                  label="Device ID"
                  value={<span className="font-mono text-[11px]">{device.id}</span>}
                />
                <Row label="Name" value={device.name} />
                <Row label="Platform" value={device.platform} />
                <Row label="Protocol" value={device.protocol_version} />
                <Row label="Created" value={new Date(device.created_at).toLocaleString()} />
              </tbody>
            </Table>
          )}
        </GlassCard>
      </div>

      <p className="pb-1 text-center text-[12px] tracking-[-0.01em] text-black/40 dark:text-white/40">
        UniKey — one keyboard, one clipboard, one workspace across every device.
      </p>
    </>
  );
};

const Row = ({ label, value }: { label: string; value: React.ReactNode }) => (
  <Tr>
    <Td className="w-40 font-medium text-black/55 dark:text-white/55">{label}</Td>
    <Td className="text-neutral-900 dark:text-white">{value}</Td>
  </Tr>
);

export default About;
