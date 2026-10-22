import React, { useCallback, useState } from 'react';
import { motion } from 'motion/react';
import { useDaemon } from '@/context/DaemonContext';
import { useAppearance } from '@/context/AppearanceContext';
import type { Theme } from '@/context/AppearanceContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { ConfigResponse } from '@/api/types';
import { GlassCard, SectionHeader, SpringSwitch, Segmented, IconTile } from '@/components/ui/primitives';
import { Button } from '@/components/ui/Button';
import { Field, TextInput, Select } from '@/components/ui/controls';
import {
  Palette,
  Monitor,
  Shield,
  Wifi,
  Save,
  RotateCcw,
  CheckCircle2,
  AlertTriangle,
  Loader2,
  ClipboardList,
  Keyboard,
  FolderUp,
  ScrollText,
} from 'lucide-react';

/** A grouped section caption with a small tinted icon tile. */
const SectionCaption = ({
  icon,
  title,
  className = '',
}: {
  icon: React.ReactNode;
  title: string;
  className?: string;
}) => (
  <div className={['mb-1 flex items-center gap-2', className].join(' ')}>
    <span className="grid h-6 w-6 shrink-0 place-items-center rounded-[7px] bg-black/[0.05] text-black/50 dark:bg-white/[0.08] dark:text-white/50">
      {icon}
    </span>
    <span className="text-[11px] font-semibold uppercase tracking-[0.1em] text-black/45 dark:text-white/45">
      {title}
    </span>
  </div>
);

/** A labeled switch row bound to a config boolean. */
const SwitchRow = ({
  title,
  checked,
  onChange,
  reduceMotion,
}: {
  title: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  reduceMotion: boolean;
}) => (
  <div className="flex items-center justify-between gap-4 py-3.5">
    <span className="text-[13.5px] font-medium tracking-[-0.01em] text-neutral-900 dark:text-white">{title}</span>
    <SpringSwitch checked={checked} onChange={onChange} label={title} reduceMotion={reduceMotion} />
  </div>
);

const Settings: React.FC = () => {
  const { daemonReachable } = useDaemon();
  const { theme, setTheme, reduceMotion, setReduceMotion, reduceTransparency, setReduceTransparency } = useAppearance();
  const fetcher = useCallback(() => api.config(), []);
  const { data: config, refetch } = usePoll<ConfigResponse>(fetcher, 5000);
  const [draft, setDraft] = useState<ConfigResponse | null>(null);
  const [saved, setSaved] = useState<ConfigResponse | null>(null);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  const save = useCallback(async () => {
    if (!draft) return;
    setSaving(true);
    setNotice(null);
    try {
      const next = await api.updateConfig(draft);
      setDraft(next);
      setSaved(next);
      setNotice({ ok: true, text: 'Configuration saved' });
      refetch();
    } catch (e: any) {
      setNotice({ ok: false, text: e?.message ?? 'Save failed' });
    } finally {
      setSaving(false);
    }
  }, [draft, refetch]);

  const discard = useCallback(() => setDraft(null), []);

  const live = draft ?? config;
  const dirty = draft !== null && draft !== saved;
  const busy = saving;

  const update = (fn: (d: ConfigResponse) => ConfigResponse) => {
    if (live) setDraft(fn(live));
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={reduceMotion ? { duration: 0 } : { duration: 0.3, ease: 'easeOut' }}
      className="flex flex-col gap-6"
    >
      {/* Appearance */}
      <GlassCard className="p-5">
        <SectionHeader
          title="Appearance"
          description="Theme and display preferences"
          right={
            <IconTile
              icon={<Palette className="h-[18px] w-[18px]" />}
              tint="bg-indigo-500/15 text-indigo-600 dark:bg-indigo-400/20 dark:text-indigo-300"
            />
          }
        />
        <div className="flex flex-col">
          <div className="flex items-center justify-between gap-4 py-3.5">
            <div>
              <p className="text-[13.5px] font-medium tracking-[-0.01em] text-neutral-900 dark:text-white">Theme</p>
              <p className="text-[12px] text-black/45 dark:text-white/45">System, light or dark</p>
            </div>
            <Segmented<Theme>
              value={theme}
              onChange={setTheme}
              reduceMotion={reduceMotion}
              ariaLabel="Theme"
              layoutId="seg-theme-settings"
              className="w-fit"
              options={[
                { value: 'system', label: 'System' },
                { value: 'light', label: 'Light' },
                { value: 'dark', label: 'Dark' },
              ]}
            />
          </div>
          <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
          <div className="flex items-center justify-between gap-4 py-3.5">
            <div>
              <p className="text-[13.5px] font-medium tracking-[-0.01em] text-neutral-900 dark:text-white">Reduce Motion</p>
              <p className="text-[12px] text-black/45 dark:text-white/45">Disable spring and slide effects</p>
            </div>
            <SpringSwitch
              checked={reduceMotion}
              onChange={setReduceMotion}
              label="Reduce Motion"
              reduceMotion={reduceMotion}
            />
          </div>
          <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
          <div className="flex items-center justify-between gap-4 py-3.5">
            <div>
              <p className="text-[13.5px] font-medium tracking-[-0.01em] text-neutral-900 dark:text-white">Reduce Transparency</p>
              <p className="text-[12px] text-black/45 dark:text-white/45">Make surfaces frostier and solid</p>
            </div>
            <SpringSwitch
              checked={reduceTransparency}
              onChange={setReduceTransparency}
              label="Reduce Transparency"
              reduceMotion={reduceMotion}
            />
          </div>
        </div>
      </GlassCard>

      {/* Daemon configuration */}
      <GlassCard className="p-5">
        <SectionHeader title="Daemon configuration" description="Live settings for the background service" />

        {!daemonReachable || !config ? (
          <div className="flex flex-col items-center gap-3 py-10 text-center">
            {daemonReachable && <Loader2 className="h-5 w-5 animate-spin text-black/40 dark:text-white/40" />}
            <p className="text-[13px] text-black/45 dark:text-white/45">
              {daemonReachable ? 'Loading configuration…' : 'Daemon offline — settings unavailable.'}
            </p>
          </div>
        ) : live ? (
          <>
            {notice && (
              <div
                className={`mb-4 flex items-center gap-2 text-[13px] font-medium ${
                  notice.ok ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-rose-400'
                }`}
              >
                {notice.ok ? (
                  <CheckCircle2 className="h-[18px] w-[18px] shrink-0" />
                ) : (
                  <AlertTriangle className="h-[18px] w-[18px] shrink-0" />
                )}
                {notice.text}
              </div>
            )}

            <div className="flex flex-col">
              {/* Device */}
              <SectionCaption icon={<Monitor className="h-4 w-4" />} title="Device" />
              <div className="py-3.5">
                <Field label="Device name" hint="Leave empty to use the hostname">
                  <TextInput
                    value={live.daemon.device_name ?? ''}
                    placeholder="(from hostname)"
                    onChange={e =>
                      update(d => ({ ...d, daemon: { ...d.daemon, device_name: e.target.value || null } }))
                    }
                  />
                </Field>
              </div>
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <div className="flex items-center justify-between gap-4 py-3.5">
                <div>
                  <p className="text-[13.5px] font-medium tracking-[-0.01em] text-neutral-900 dark:text-white">Mode</p>
                  <p className="text-[12px] text-black/45 dark:text-white/45">Daemon operation mode</p>
                </div>
                <span className="rounded-lg bg-black/[0.05] px-2.5 py-1 font-mono text-[12px] leading-tight text-black/70 dark:bg-white/[0.08] dark:text-white/70">
                  {live.mode}
                </span>
              </div>

              {/* Clipboard */}
              <SectionCaption icon={<ClipboardList className="h-4 w-4" />} title="Clipboard" className="mt-6" />
              <SwitchRow
                title="Auto-sync"
                checked={live.clipboard.auto_sync}
                onChange={v => update(d => ({ ...d, clipboard: { ...d.clipboard, auto_sync: v } }))}
                reduceMotion={reduceMotion}
              />
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <SwitchRow
                title="Sync images"
                checked={live.clipboard.sync_images}
                onChange={v => update(d => ({ ...d, clipboard: { ...d.clipboard, sync_images: v } }))}
                reduceMotion={reduceMotion}
              />
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <div className="py-3.5">
                <Field label="History limit" hint="Number of clipboard entries retained">
                  <TextInput
                    type="number"
                    min={1}
                    value={live.clipboard.history_limit}
                    onChange={e =>
                      update(d => ({ ...d, clipboard: { ...d.clipboard, history_limit: Number(e.target.value) } }))
                    }
                  />
                </Field>
              </div>

              {/* Input */}
              <SectionCaption icon={<Keyboard className="h-4 w-4" />} title="Input" className="mt-6" />
              <SwitchRow
                title="Forward keyboard"
                checked={live.input.keyboard_forwarding}
                onChange={v => update(d => ({ ...d, input: { ...d.input, keyboard_forwarding: v } }))}
                reduceMotion={reduceMotion}
              />
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <SwitchRow
                title="Forward mouse"
                checked={live.input.mouse_forwarding}
                onChange={v => update(d => ({ ...d, input: { ...d.input, mouse_forwarding: v } }))}
                reduceMotion={reduceMotion}
              />
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <SwitchRow
                title="Edge switching"
                checked={live.input.edge_switching}
                onChange={v => update(d => ({ ...d, input: { ...d.input, edge_switching: v } }))}
                reduceMotion={reduceMotion}
              />

              {/* Transfers */}
              <SectionCaption icon={<FolderUp className="h-4 w-4" />} title="Transfers" className="mt-6" />
              <div className="py-3.5">
                <Field label="Download folder" hint="Where received files are saved">
                  <TextInput
                    value={live.transfer.download_dir ?? ''}
                    placeholder="(default downloads folder)"
                    onChange={e =>
                      update(d => ({ ...d, transfer: { ...d.transfer, download_dir: e.target.value || null } }))
                    }
                  />
                </Field>
              </div>

              {/* Security */}
              <SectionCaption icon={<Shield className="h-4 w-4" />} title="Security" className="mt-6" />
              <SwitchRow
                title="Require pairing"
                checked={live.encryption.require_pairing}
                onChange={v => update(d => ({ ...d, encryption: { ...d.encryption, require_pairing: v } }))}
                reduceMotion={reduceMotion}
              />
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <div className="py-3.5">
                <Field label="Key rotation (min)" hint="How often session keys are rotated">
                  <TextInput
                    type="number"
                    min={0}
                    value={live.encryption.key_rotation_minutes}
                    onChange={e =>
                      update(d => ({
                        ...d,
                        encryption: { ...d.encryption, key_rotation_minutes: Number(e.target.value) },
                      }))
                    }
                  />
                </Field>
              </div>
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <div className="flex items-center justify-between gap-4 py-3.5">
                <div className="flex items-center gap-2.5">
                  <Wifi className="h-[18px] w-[18px] text-black/50 dark:text-white/50" />
                  <div>
                    <p className="text-[13.5px] font-medium tracking-[-0.01em] text-neutral-900 dark:text-white">Network</p>
                    <p className="text-[12px] text-black/45 dark:text-white/45">Ports bound by the daemon</p>
                  </div>
                </div>
                <span className="rounded-lg bg-black/[0.05] px-2.5 py-1 font-mono text-[12px] leading-tight text-black/70 dark:bg-white/[0.08] dark:text-white/70">
                  TCP {live.network.tcp_port} · Disc {live.network.discovery_port} · API {live.network.api_port}
                </span>
              </div>

              {/* Logging */}
              <SectionCaption icon={<ScrollText className="h-4 w-4" />} title="Logging" className="mt-6" />
              <div className="py-3.5">
                <Field label="Log level">
                  <Select
                    value={live.logging.level}
                    onChange={e => update(d => ({ ...d, logging: { ...d.logging, level: e.target.value } }))}
                  >
                    {['trace', 'debug', 'info', 'warn', 'error'].map(l => (
                      <option key={l} value={l}>{l}</option>
                    ))}
                  </Select>
                </Field>
              </div>
              <div className="h-px bg-black/[0.06] dark:bg-white/[0.08]" />
              <SwitchRow
                title="File logging"
                checked={live.logging.file_enabled}
                onChange={v => update(d => ({ ...d, logging: { ...d.logging, file_enabled: v } }))}
                reduceMotion={reduceMotion}
              />
            </div>

            {/* Footer */}
            <div className="mt-6 border-t border-black/[0.06] pt-4 dark:border-white/[0.08]">
              {dirty && (
                <div className="mb-3 flex items-center gap-2 text-[13px] font-medium text-amber-600 dark:text-amber-400">
                  <AlertTriangle className="h-[18px] w-[18px] shrink-0" />
                  You have unsaved changes
                </div>
              )}
              <div className="flex items-center gap-3">
                <Button variant="primary" onClick={save} disabled={!dirty || busy}>
                  {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
                  {busy ? 'Saving…' : 'Save changes'}
                </Button>
                <Button variant="secondary" onClick={discard} disabled={!dirty}>
                  <RotateCcw className="h-4 w-4" />
                  Discard
                </Button>
              </div>
            </div>
          </>
        ) : null}
      </GlassCard>
    </motion.div>
  );
};

export default Settings;
