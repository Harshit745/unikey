import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/lib/edgeFeedback.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { dragFingerprint, transferFeedback } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
const row = (id, status, error = null) => ({ transfer_id: id, status, error });

test('enter and drop deduplicate regardless of path order or duplicate paths', () => {
  assert.equal(dragFingerprint(['b', 'a', 'a']), dragFingerprint(['a', 'b']));
  assert.notEqual(dragFingerprint(['a']), dragFingerprint(['b']));
});
test('queued, written and missing transfers do not show success', () => {
  for (const status of ['Queued', 'InProgress', 'AwaitingConfirmation', 'Completed']) {
    assert.equal(transferFeedback(['a'], [row('a', status)]).state, 'sending');
  }
  assert.equal(transferFeedback(['a'], []).state, 'sending');
  assert.equal(transferFeedback([], []).state, 'sending');
});
test('all files must be receiver-verified', () => {
  assert.equal(transferFeedback(['a', 'b'], [row('a', 'Verified'), row('b', 'InProgress')]).state, 'sending');
  assert.equal(transferFeedback(['a', 'b'], [row('a', 'Verified'), row('b', 'Verified')]).state, 'done');
});
test('receiver errors and cancellation are visible', () => {
  assert.equal(transferFeedback(['a'], [row('a', 'Failed', 'Disk full')]).detail, 'Disk full');
  assert.equal(transferFeedback(['a'], [row('a', 'Cancelled')]).state, 'error');
});
