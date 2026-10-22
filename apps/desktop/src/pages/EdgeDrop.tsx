import React, { useEffect, useMemo, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api } from '@/api/client';

/**
 * Content for the narrow native edge-capture window. The window itself is
 * created and positioned by the Tauri shell. A drop is only accepted after
 * the daemon confirms that the selected peer is actively paired and trusted.
 */
const EdgeDrop: React.FC = () => {
  const peerId = useMemo(() => {
    const query = window.location.hash.split('?')[1] ?? '';
    return new URLSearchParams(query).get('peer') ?? '';
  }, []);
  const [active, setActive] = useState(false);
  const [state, setState] = useState<'ready' | 'sending' | 'done' | 'error'>('ready');

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWindow().onDragDropEvent(event => {
      if (disposed) return;
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        setActive(true);
        setState('ready');
        return;
      }
      if (event.payload.type === 'drop') {
        const paths = event.payload.paths;
        setState('sending');
        void (async () => {
          try {
            for (const path of paths) await api.sendTransfer(peerId, path);
            setState('done');
          } catch {
            setState('error');
          } finally {
            setActive(false);
          }
        })();
        return;
      }
      setActive(false);
    }).then(stop => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [peerId]);

  const colour = state === 'error'
    ? 'bg-red-500'
    : state === 'done'
      ? 'bg-emerald-500'
      : active || state === 'sending'
        ? 'bg-blue-500'
        : 'bg-blue-500/20';

  return (
    <main
      aria-label="UniKey screen-edge file handoff"
      className={`h-screen w-screen transition-colors duration-150 ${colour}`}
      title="Drop files here to hand them to the paired device"
    />
  );
};

export default EdgeDrop;
