# Local mDNS cache bound

This directory contains `mdns-sd` 0.21.5 from crates.io, licensed under MIT or Apache-2.0 (see the adjacent license files). SeamlessControl keeps the source here because the upstream library has no record-count limit for service discovery. An unauthenticated LAN advertiser can otherwise retain arbitrarily many records with long DNS TTLs in the Windows app's daemon.

Local changes are confined to `src/dns_cache.rs` and `src/dns_parser.rs`:

- Cap retained cache storage at 1,024 units, counting both records and map keys. Existing records can refresh when the cap is reached; new entries are ignored until space is freed.
- Cap received record TTLs at 120 seconds and remove empty map entries before admission. Published records retain their configured TTL.
- Test a stream of 5,000 distinct, maximum-TTL PTR advertisements.

The Windows application also caps its displayed nearby list at 64 computers and publishes UI changes no more than twice per second. Discovery is a hint only; pairing remains mandatory.

When updating `mdns-sd`, preserve or replace these bounds and rerun the adversarial cache test. The original package metadata, tests, examples and license files are retained for review.
