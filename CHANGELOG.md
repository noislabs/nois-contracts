# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2023-02-20

### Added

- New sink contract ([#151])

[#151]: https://github.com/noislabs/nois-contracts/pull/151

### Changes

- Bump drand-verify to 0.4, using pairing for the BLS verification.
- Reduce the number of verification executions per round from 6 to 3 to increase the number of processable jobs.
- Introduce reward point system for drand submissions.
- Upgrade CosmWasm to 1.2.
- icecube: Rename `admin` to `manager`.
- drand: Only pay out rewards for bots in the right group ([#147]).
- drand: Store height and tx_index of submission to allow finding transaction for a submission ([#153]).
- proxy: Make callback gas limit configurable and reduce value to 500k in tests.
- Pull out `RequestBeaconOrigin` struct which belongs to the proxy-dapp communication.
- Bump IBC protocol version to "nois-v5".
- protocol: Remove unused job_id from `DeliverBeaconPacketAck`
- drand: Add `reward_points` to bot stats

[#147]: https://github.com/noislabs/nois-contracts/pull/147
[#153]: https://github.com/noislabs/nois-contracts/issues/153

## [0.8.0]

Base version for starting the CHANGELOG.

[unreleased]: https://github.com/noislabs/nois-contracts/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/noislabs/nois-contracts/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/noislabs/nois-contracts/tree/v0.8.0
