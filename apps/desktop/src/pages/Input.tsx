import React, { useCallback, useState } from 'react';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { ConfigResponse } from '@/api/types';
import { Placeholder } from '@/components/Placeholder';
import { GlassCard, SpringSwitch, SectionHeader } from '@/components/ui/primitives';
import { StatCard } from '@/components/ui/StatCard';
import { useAppearance } from '@/context/AppearanceContext';
import { Keyboard, Mouse, Activity } from 'lucide-react';

const Input: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const { reduceMotion } = useAppearance();
  const fetcher = useCallback(() => api.config(), []);
  const { data: config, refetch } = usePoll<ConfigResponse>(fetcher, 5000);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  const setToggle = useCallback(async (key: 'keyboard_forwarding' | 'mouse_forwarding' | 'edge_switching', value: boolean) => {
    if (!config) return;
    setSaving(true);
    setNotice(null);
    try {
      const next: ConfigResponse = {
        ...config,
        input: { ...config.input, [key]: value },
      };
      await api.updateConfig(next);
      setNotice({ ok: true, text: 'Settings saved' });
      refetch();
    } catch (e: any) {
      setNotice({ ok: false, text: e?.message ?? 'Save failed' });
    } finally {
      setSaving(false);
    }
  }, [config, refetch]);

  return (
    <>
      {!daemonReachable ? (
        <Placeholder
          icon="⌨️"
          title="Daemon offline"
          description="Start the daemon to configure input forwarding."
        />
      ) : !config ? (
        <GlassCard className="flex flex-col items-center justify-center gap-2 py-12 text-center">
          <div className="spinner" />
          <p className="text-[13px] text-black/45 dark:text-white/45">Loading configuration…</p>
        </GlassCard>
      ) : (
        <>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <StatCard
              icon={<Activity className="h-[18px] w-[18px]" />}
              tint="bg-indigo-500/15 text-indigo-600 dark:bg-indigo-400/20 dark:text-indigo-300"
              label="Input forwarding"
              valueNode={
                config.input.keyboard_forwarding || config.input.mouse_forwarding ? 'Active' : 'Off'
              }
              reduceMotion={reduceMotion}
            />
            <StatCard
              icon={<Keyboard className="h-[18px] w-[18px]" />}
              tint="bg-sky-500/15 text-sky-600 dark:bg-sky-400/20 dark:text-sky-300"
              label="Keyboard"
              valueNode={config.input.keyboard_forwarding ? 'On' : 'Off'}
              reduceMotion={reduceMotion}
            />
            <StatCard
              icon={<Mouse className="h-[18px] w-[18px]" />}
              tint="bg-teal-500/15 text-teal-600 dark:bg-teal-400/20 dark:text-teal-300"
              label="Mouse"
              valueNode={config.input.mouse_forwarding ? 'On' : 'Off'}
              reduceMotion={reduceMotion}
            />
          </div>

          <GlassCard className="p-6">
            <SectionHeader
              title="Forwarding rules"
              description="Toggles are saved to the daemon configuration."
            />
            <div className="flex flex-col">
              {[
                { key: 'keyboard_forwarding' as const, label: 'Forward keyboard events' },
                { key: 'mouse_forwarding' as const, label: 'Forward mouse movements' },
                { key: 'edge_switching' as const, label: 'Enable edge-crossing transition' },
              ].map(({ key, label }, i) => (
                <div key={key}>
                  {i > 0 && <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />}
                  <div className="flex items-center justify-between gap-4 py-3.5">
                    <span className="text-[13.5px] font-medium tracking-[-0.01em] text-neutral-900 dark:text-white">
                      {label}
                    </span>
                    <SpringSwitch
                      checked={config.input[key]}
                      onChange={value => setToggle(key, value)}
                      label={label}
                      reduceMotion={reduceMotion}
                      disabled={saving}
                    />
                  </div>
                </div>
              ))}
            </div>
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
        </>
      )}
    </>
  );
};

export default Input;
