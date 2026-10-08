# Wire errors

## Frame and codec errors

- `InvalidMagic`: required `0xDA11` prefix is absent.
- `UnsupportedVersion`: frame version is not supported.
- `UnknownFlags`: reserved flag bits are set.
- `PayloadTooLarge`: payload exceeds 1024 bytes.
- `Truncated`: input ends before required metadata or frame bytes.
- `InsufficientCapacity`: caller output cannot hold the requested result.
- `PayloadLengthMismatch`: payload buffer length differs from header length.
- `ChecksumMismatch`: CRC does not match header and payload.
- `InvalidRange`: checksum range is inverted.
- `PolicyRejected`: a reserved higher-level policy rejected the operation.

## Sequence errors

`SequenceDuplicate`, `SequenceStale`, `SequenceJumpTooLarge`, and
`SequenceContextMismatch` distinguish replay-window decisions. These errors do
not authenticate the frame and do not themselves prove malicious behavior.

## Fragment errors

`FragmentUnsupportedVersion`, `FragmentCountInvalid`, `FragmentIndexInvalid`,
`FragmentRangeInvalid`, `FragmentTooLarge`, `FragmentDuplicate`,
`FragmentOverlap`, `FragmentStale`, `ReassemblyCapacity`, `ReassemblyState`,
and `ReassemblyIncomplete` identify separate metadata, lifecycle, capacity,
and completion failures.

## Endpoint errors

The optional endpoint parser has its own `WireEndpointError` enum for bounded
syntax. Endpoint parsing errors never indicate a transport connection failure
because the parser performs no connection attempt.

## Handling rule

Branch on the typed result. Do not retry a checksum failure as if it were a
capacity error, treat a stale sequence as a duplicate, or publish a partial
payload after `Truncated` or `ChecksumMismatch`.
