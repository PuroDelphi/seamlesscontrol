# Local mDNS discovery bounds

This directory contains `mdns-sd` 0.21.5 from crates.io, licensed under MIT or Apache-2.0 (see the adjacent license files). SeamlessControl keeps the source here because the upstream library has no record-count limit for service discovery. An unauthenticated LAN advertiser can otherwise retain arbitrarily many records with long DNS TTLs in the Windows app's daemon.

Local changes are confined to `src/dns_cache.rs`, `src/dns_parser.rs` and `src/service_daemon.rs`:

- Cap retained cache storage at 1,024 units, counting both records and map keys. Existing records can refresh when the cap is reached; new entries are ignored until space is freed.
- Cap received record TTLs at 120 seconds and remove empty map entries before admission. Published records retain their configured TTL.
- Coalesce timer wakeups into 50 ms slots. Refreshed records reuse a slot instead of appending timers for every packet. The received TTL cap confines remotely scheduled deadlines to about 2,400 slots, independent of refresh volume. Expired slots are removed on every event-loop turn.
- Read at most 32 multicast datagrams per socket and turn, then run timer and cache cleanup. A backlog flag keeps a socket eligible on the next turn even if edge-triggered polling does not announce its unread packets again.
- Limit delayed multicast responses to 128, evict orphan SRV/TXT/NSEC records after their TTL, and discard resolved-instance tracking when a service is removed.
- Test a stream of 5,000 distinct, maximum-TTL PTR advertisements.
- Test 100,000 repeated timer refreshes, the finite packet budget, and orphan-record expiry.

The Windows application also caps its displayed nearby list at 64 computers and publishes UI changes no more than twice per second. Discovery is a hint only; pairing remains mandatory.

When updating `mdns-sd`, preserve or replace these bounds and rerun all targeted adversarial tests in `.github/workflows/validate.yml`. The original package metadata, tests, examples and license files are retained for review.
