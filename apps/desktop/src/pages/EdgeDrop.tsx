import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
  const handledDrag = useRef<string | null>(null);
  const resetTimer = useRef<number | null>(null);

  const handoff = useCallback(async (paths: string[]) => {
    if (paths.length === 0) return;
    const fingerprint = paths.join('\u0000');
    if (handledDrag.current === fingerprint) return;
    handledDrag.current = fingerprint;
    setState('sending');
    try {
      for (const path of paths) await api.sendTransfer(peerId, path);
      setState('done');
    } catch {
      setState('error');
    }
  }, [peerId]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWindow().onDragDropEvent(event => {
      if (disposed) return;
      if (event.payload.type === 'enter') {
        setActive(true);
        setState('ready');
        // Crossing the configured edge is the handoff gesture. Tauri exposes
        // the real Finder/Explorer paths on `enter`, so the user does not
        // need to hit a tiny drop target or bring the UniKey window forward.
        // `drop` remains a fallback, while handledDrag prevents duplicates.
        void handoff(event.payload.paths);
        return;
      }
      if (event.payload.type === 'over') {
        setActive(true);
        return;
      }
      if (event.payload.type === 'drop') {
        const paths = event.payload.paths;
        void handoff(paths);
        setActive(false);
        if (resetTimer.current !== null) window.clearTimeout(resetTimer.current);
        resetTimer.current = window.setTimeout(() => {
          handledDrag.current = null;
        }, 750);
        return;
      }
      setActive(false);
      if (resetTimer.current !== null) window.clearTimeout(resetTimer.current);
      resetTimer.current = window.setTimeout(() => {
        handledDrag.current = null;
      }, 750);
    }).then(stop => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      if (resetTimer.current !== null) window.clearTimeout(resetTimer.current);
      unlisten?.();
    };
  }, [handoff]);

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
      title="Drag files to this edge to hand them to the paired device"
    />
  );
};

export default EdgeDrop;
