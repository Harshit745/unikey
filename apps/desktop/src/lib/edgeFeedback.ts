import type { TransferInfo } from '@/api/types';

export function dragFingerprint(paths: string[]): string {
  return JSON.stringify([...new Set(paths)].sort());
}

/** Queuing an API request is never evidence that the receiver saved a file. */
export function transferFeedback(ids: string[], transfers: TransferInfo[]): {
  state: 'sending' | 'done' | 'error'; detail: string;
} {
  const rows = ids.map(id => transfers.find(item => item.transfer_id === id));
  const failed = rows.find(row => row?.status === 'Failed' || row?.status === 'Cancelled');
  if (failed) return { state: 'error', detail: failed.error ?? 'Transfer failed or was cancelled' };
  if (ids.length > 0 && rows.every(row => row?.status === 'Verified')) {
    return { state: 'done', detail: 'Received and verified on the paired device' };
  }
  return { state: 'sending', detail: 'Sending — waiting for receiver verification' };
}
