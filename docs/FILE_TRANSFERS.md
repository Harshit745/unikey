# File transfer format and throughput

Files already stream in 64 KiB chunks; splitting them into fragments is not a
new transport requirement. The earlier chunks serialized each byte as a JSON
number. For byte values spread across 0–255, that costs approximately 3.57 wire
bytes per file byte, plus packet/encryption headers and JSON parsing work.

Updated peers advertise `file_chunk_binary` in Hello. A file sender selects its
encoding once for that concrete session. Binary-v1 chunks contain a 5-byte
magic/version, 16-byte transfer UUID, 4-byte big-endian chunk index and the raw
bytes. A full 65,536-byte chunk therefore has a 65,561-byte payload. No compression
or base64 is involved. Older peers without the optional capability continue
receiving JSON chunks; updated receivers accept either representation. Both
devices must be updated and reconnected to use binary-v1.

The capability is only a performance hint, not an authenticated permission or a
replacement for pairing. Both encodings retain the existing encrypted frames,
identity pinning, replay checks, trusted-peer routing, chunk bounds, SHA-256
verification, atomic final write and receiver acknowledgement. `Verified` still
means the receiver reported successful verification and persistence.

Full service receive queues now exert TCP backpressure rather than silently
discarding packets. The transport releases its session lock before waiting for
queue capacity. Short middle chunks and out-of-order appends are rejected. Fast
receiver acknowledgements cannot be overwritten by late sender bookkeeping.

## Reproduce the measurement

```sh
cargo test -p files --release --test transfer_e2e large_file -- --ignored --nocapture --test-threads=1
```

On the development Mac on 2026-10-24, one paired test run transferred generated
95 MiB (99,614,720-byte) data over real encrypted loopback TCP, saved it to a real
temporary download folder, checked the bytes and waited for the reverse ACK:

| Encoding | Preparation included | Verified total | Effective throughput |
| --- | --- | --- | --- |
| Legacy JSON, with queue fix | 353 ms | 3,442 ms | 27.60 MiB/s |
| Binary-v1, with queue fix | 357 ms | 1,193 ms | 79.59 MiB/s |

This comparison isolates encoding with the same reliable queue behavior. An
initial run of the old, dropping-queue implementation did not finish within the
120-second test limit; it is not a valid throughput baseline. Parallel test
fixtures also needed UUID-based folders to prevent collisions on coarse clocks.

These are host-dependent loopback results, not Wi-Fi, cross-device, or promised
end-user transfer times. Automated tests additionally exercise legacy fallback,
a deliberately slow receiver with a two-packet inbound queue, chunk validation,
and the receiver-ACK state race.

## Physical retest and remaining limits

1. Install the same updated source build on both computers and reconnect.
2. Send a disposable ~95 MB file through the normal Files or edge-handoff UI.
3. Time until **Verified**, not until the sender reaches 100%; check the received
   file's SHA-256 as well. Repeat in the opposite direction on the same network.
4. Compare like-for-like file sizes, Wi-Fi connections and concurrent traffic.

The sender streams bounded chunks, but the receiver still assembles the file
in memory before final hashing and saving; disk-backed receiving/resume remains
future work. Bulk data and cursor/control packets still share a TCP connection,
so heavy transfers can delay interactive input. No unencrypted parallel socket,
security bypass, resumable protocol, or arbitrary remote-app drag bridge is
claimed by this update.
