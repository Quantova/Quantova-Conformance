# Quantova-Conformance

The frozen cross repo test vectors for Quantova and the runner that reproduces them. It keeps the virtual machine, the language, the SDK, and the node in agreement on one wire format, and it proves that a classical artifact cannot be parsed at all.

Quantova is a sovereign post quantum Layer 1 that shares no wire and no encoding with any other chain. A claim like that has to be checkable, not asserted. The vectors here are the checkable form. Each is a fixed input and the exact value the reference must recompute, and the hostile vectors are inputs the reference must refuse.

## What it does

The runner reproduces every frozen vector with the reference crates and confirms two things. A positive vector must recompute bit for bit, and a hostile vector must be refused. The reference crates are pinned by git tag, so a vector is measured against a known build and not a moving target.

```
qtv-idfmt, qtv-account, qtv-codec, qtv-tx   from Quantova-Chain at tag v0.2.0
qtv-crypto                                   from Q-Crypto at tag v0.1.0
```

Run it as a binary that prints one line per area, or as the test suite.

```
cargo run -p qtv-conformance-runner
cargo test -p qtv-conformance-runner
```

## The positive vectors

These fix the wire format the whole stack must agree on.

- Codec. The canonical encoding of `u32`, `u64`, `u128`, byte strings, and `Option`, checked both ways, encode to the frozen bytes and decode back to the value.
- Address derivation. A master seed and an account index derive the q1 address and its scheme, byte for byte.
- Transaction. A transfer body encodes to the frozen bytes and signs to the frozen transaction identifier.
- Scheme hash. The same account and transfer under an explicit signature scheme, so the scheme is pinned into the address and the signed wrapper.
- Identifier families. One input renders the whole Quantova identifier set, q1 for addresses, and the q2, qtx, qbk, qst, qcid, and qpf families, each to its frozen string.

## The hostile vectors

These prove refusal, which is the harder half of the property. The runner enforces the identifier, address, cryptographic, and bridge refusals against the reference.

- A hex hash, the shape of an Ethereum identifier, parses in none of the identifier families.
- A payload below the key floor renders in no address or secret family, so an under length key cannot masquerade as one.
- The classical 256 bit digest of the empty input differs from the FIPS 202 SHA-3 256 digest of the same input, so a classical digest is never a stack digest.
- Bridge boundary cases are refused. Two origins sharing one symbol cannot commingle, an artifact that is not an Airlock form is unparseable, and a bridged asset offered as stake is rejected.

The vector tree also freezes further areas as the surface grows, fee band and stale rate behavior, a virtual machine scratch memory case, a consensus vector that a non module lattice attestation is rejected, an unknown scheme identifier that does not parse, and the hostile governance vectors. The governance vectors are mirrored from QONCORD and are enforced by the constitutional gate in that repository, where a referendum that crosses an invariant must be unenactable.

## Why it is frozen

A conformance vector is only worth anything if it does not move. Once a value is committed here, a change to it is a visible change to the wire format that every consumer sees, rather than a quiet drift in one implementation. That is what keeps the VM, the language, the SDK, and the node speaking the same format.

## Cryptography

Hashing is SHA-3 and SHAKE from FIPS 202 and signatures are ML-DSA-65 from FIPS 204. There is no elliptic curve anywhere, and the hostile vectors exist to prove it. The stack cryptography is a from scratch reference implementation validated against the NIST vectors. It has not been independently audited, and the chain is at testnet.

## Governance and license

Governed by the crypto policy, POLICY-crypto, in the Quantova-Specs repository. Commits are authored by the owner only. Dual licensed under Apache 2.0 and MIT.
