import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api } from '@/api/client';
import { dragFingerprint, transferFeedback } from '@/lib/edgeFeedback';

/**
 * Content for the narrow native edge-capture window. The window itself is
 * created and positioned by the Tauri shell. A drop is only accepted after
 * the daemon confirms that the selected peer is actively paired and trusted.
 */
const EdgeDrop: React.FC = () => {
  const [query] = useSearchParams();
  const peerId = query.get('peer') ?? '';
  const [active, setActive] = useState(false);
  const [state, setState] = useState<'ready' | 'sending' | 'done' | 'error'>('ready');
  const handledDrag = useRef<string | null>(null);
  const resetTimer = useRef<number | null>(null);
  const pending = useRef<string[]>([]);
  const preparing = useRef(0);
  const submissionFailed = useRef(false);
  const [detail, setDetail] = useState('Drag files to this edge');

  useEffect(() => {
    pending.current = [];
    handledDrag.current = null;
    setState('ready');
    setDetail('Drag files to this edge');
  }, [peerId]);

  useEffect(() => {
    let disposed = false;
    let polling = false;
    const timer = window.setInterval(async () => {
      if (polling || preparing.current > 0 || submissionFailed.current || pending.current.length === 0) return;
      polling = true;
      const ids = pending.current;
      try {
        const feedback = transferFeedback(ids, await api.transfers());
        if (disposed || pending.current !== ids) return;
        setState(feedback.state); setDetail(feedback.detail);
        if (feedback.state !== 'sending') pending.current = [];
      } catch (error) {
        if (!disposed) { setState('error'); setDetail(`Cannot confirm receipt: ${String(error)}`); }
      } finally { polling = false; }
    }, 500);
    return () => { disposed = true; window.clearInterval(timer); };
  }, []);

  const handoff = useCallback(async (paths: string[]) => {
    if (paths.length === 0 || !peerId) return;
    const fingerprint = dragFingerprint(paths);
    if (handledDrag.current === fingerprint) return;
    handledDrag.current = fingerprint;
    preparing.current += 1;
    submissionFailed.current = false;
    setState('sending');
    setDetail('Preparing file handoff');
    try {
      const ids: string[] = [];
      for (const path of new Set(paths)) ids.push(await api.sendTransfer(peerId, path));
      pending.current = [...pending.current, ...ids];
      setDetail('Sending — waiting for receiver verification');
    } catch (error) {
      submissionFailed.current = true;
      setState('error');
      setDetail(error instanceof Error ? error.message : String(error));
    } finally {
      preparing.current -= 1;
    }
  }, [peerId]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWindow().onDragDropEvent(event => {
      if (disposed) return;
      if (event.payload.type === 'enter') {
        setActive(true);
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
    }).catch(error => {
      if (!disposed) { setState('error'); setDetail(`Native file drop unavailable: ${String(error)}`); }
    });
    return () => {
      disposed = true;
      if (resetTimer.current !== null) window.clearTimeout(resetTimer.current);
      unlisten?.();
    };
  }, [handoff]);

  const colour = state === 'error'
    ? 'bg-red-700'
    : state === 'done'
      ? 'bg-emerald-700'
      : active || state === 'sending'
        ? 'bg-blue-600'
        : 'bg-blue-500/20';

  return (
    <main
      aria-label="UniKey screen-edge file handoff"
      className={`h-screen w-screen transition-colors duration-150 ${colour}`}
      title={detail}
    >
      <p role="status" aria-live="polite" className={`flex h-full items-center justify-center text-[11px] font-medium ${active || state !== 'ready' ? 'text-white' : 'text-[var(--ink)]'}`} style={{ writingMode: 'vertical-rl' }}>
        {state === 'error' ? detail : state === 'done' ? 'Received ✓' : state === 'sending' ? 'Sending…' : 'Drag files here'}
      </p>
    </main>
  );
};

export default EdgeDrop;
