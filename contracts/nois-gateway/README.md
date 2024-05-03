# Nois Gateway

The gateway processes incoming job requests and routes them the right backend.

## Jobs

TODO: jobs and jobs queue

## Archive

If a request comes in for which the corresponding drand beacon was already
verified before, it is looked up and used from the archive. In this case no job
is enqueued by the gateway but instead the beacon delivery packet is sent right
away.

After the migration to fast randomness, it is unlikely that a beacon is needed
multiple times. So it is important to store very little data per beacon. Right
now the archive is a KV table where the value us a 32 byte randomness (hash of
the signature). The key is 11 bytes long and composed as follows:

```rust
// fastnet
[
    7,    // BELL
    b'd', // drand
    b'm', // mainnet aka. fastnet
    round[0], round[1], round[2], round[3], round[4], round[5], round[6], round[7],
]

// quicknet
[
    7,    // BELL
    b'd', // drand
    b'q', // quicknet
    round[0], round[1], round[2], round[3], round[4], round[5], round[6], round[7],
]
```

After the migration to quicknet is completed (see
https://github.com/noislabs/nois-contracts/issues/293 and friends), all entries
with prefix `b"\x07dm"` can be deleted as they will never be accessed again.
