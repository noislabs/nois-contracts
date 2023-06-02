# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- proxy: Streamline event attributes
- proxy: Ensure the `after` value is not in the past when using
  `GetRandomnessAfter`.

## [0.13.4] - 2023-05-17

### Added

- proxy: Add support for updating `callback_gas_limit` via SetConfig ([#240]).

[#240]: https://github.com/noislabs/nois-contracts/pull/240

### Fixed

- proxy: Updating the `allowlist_enabled` value via `SetConfig` was fixed. In
  0.13.3 an unset value caused the config to set `allowlist_enabled` to
  None/false. Now it does not change the value.

## [0.13.3] - 2023-05-04

### Changed

- Use anybuf from crates.io
- Upgrade cosmwasm to 1.2.5
- drand: Add `QueryMsg::IsAllowlisted` and `QueryMsg::Allowlist` (one word)
  analogue to `QueryMsg::IsAllowListed` and `QueryMsg::AllowList`.
- drand: Rename `AllowListResponse` to `AllowlistResponse`. Rename
  `IsAllowListedResponse` to `IsAllowlistedResponse`.
- proxy: Add allowlist

## [0.13.2] - 2023-04-26

### Fix

- nois-proxy: Fix typo in `ExecuteMsg::Withdaw`/`SudoMsg::Withdaw`. Renamed to
  `::Withdraw`.

## [0.13.1] - 2023-04-25

### Fix

- nois-proxy: Embrace the use of empty `prices` lists to deactivate the proxy.
  Turn a panic into an error when this happens.
- all: Make all `ContractError`s `#[non_exhaustive]` since error cases can come
  up over time.

## [0.13.0] - 2023-04-23

### Changed

- nois-proxy: the config parameters can be changed. And it is possible to add a
  manager (optional)
- nois-proxy: add sudo messages to control the proxy when compile with
  `governance_owned` enabled.
- Upgrade the nois standard library to version 0.7.
- Bump IBC protocol version to `nois-v7`. This bring the publication time as a
  field to DeliverBeacon and NoisCallback.

## [0.12.0] - 2023-04-13

### Changed

- Upgrade Rust to 1.68.2 and workspace-optimizer to 0.12.13 to use sparse
  protocol for Cargo.
- Gateway: Instantiate payment contracts.
- Gateway: Start customers database.
- Payment: Replace `community_pool` address config with `Anything`
  implementation to send `MsgFundCommunityPool`s via CosmosMsg::Stargate.

**IBC protocol**

- Convert IBC packets into enums InPacket/OutPacket for extensibility.
- Ensure IBC connection is established in the one direction (user chain to
  Nois).
- Create Welcome packet after establishing a connection.
- Bump protocol version to `nois-v6`.
- Allow proxy to pay its randomness via IBC.

**Testing**

- Improve file structure of test files to better allow for individual execution
  and general maintainability.

## [0.11.0] - 2023-03-26

### Changed

- Gateway: Add manager and price to config. This is a state breaking change.
  ([#193])
- Upgrade cosmwasm to 1.2.3
- Payment: Support zero amounts ([#198])

[#193]: https://github.com/noislabs/nois-contracts/pull/192
[#198]: https://github.com/noislabs/nois-contracts/pull/198

## [0.10.2] - 2023-03-24

### Changed

- Icecube: Ensure only manager can set drand address. This way an attacker
  cannot set a wrong address during the deployment process. ([#192])

[#192]: https://github.com/noislabs/nois-contracts/pull/192

## [0.10.1] - 2023-03-23

### Changed

- Allow manager of nois-drand to set configugation ([#191]).

[#191]: https://github.com/noislabs/nois-contracts/pull/191

## [0.10.0] - 2023-03-20

### Added

- New payment contract
- An icecube or drand manager can set another manager (multisig rekey)

### Changed

- Migrade to new drand mainnet (chain hash
  `dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493`) ([#177]).
- Store submission more efficiently in the database

[#177]: https://github.com/noislabs/nois-contracts/pull/177

## [0.9.0] - 2023-02-20

[v0.9.0 article](https://scripta.network/@desmos1mvwy0d9kerz6yp9gj0u3alge9jjyjdu5m0hkpe/fd070691-1d67-4131-b0c7-034476c088e2)

### Added

- New sink contract ([#151])

[#151]: https://github.com/noislabs/nois-contracts/pull/151

### Changed

- Bump drand-verify to 0.4, using pairing for the BLS verification.
- Reduce the number of verification executions per round from 6 to 3 to increase
  the number of processable jobs.
- Introduce reward point system for drand submissions.
- Upgrade CosmWasm to 1.2.
- icecube: Rename `admin` to `manager`.
- drand: Only pay out rewards for bots in the right group ([#147]).
- drand: Store height and tx_index of submission to allow finding transaction
  for a submission ([#153]).
- proxy: Make callback gas limit configurable and reduce value to 500k in tests.
- Pull out `RequestBeaconOrigin` struct which belongs to the proxy-dapp
  communication.
- Bump IBC protocol version to "nois-v5".
- protocol: Remove unused job_id from `DeliverBeaconPacketAck`
- drand: Add `reward_points` to bot stats

[#147]: https://github.com/noislabs/nois-contracts/pull/147
[#153]: https://github.com/noislabs/nois-contracts/issues/153

## [0.8.0]

[v0.8.0 article](https://scripta.network/@desmos1s5rsl054mufsu2nhqn2wmvsmx0s2vwkcxwwwuv/d3e8db51-a111-4870-8fa0-4c37df9081b5)

Base version for starting the CHANGELOG.

[unreleased]: https://github.com/noislabs/nois-contracts/compare/v0.13.4...HEAD
[0.13.4]: https://github.com/noislabs/nois-contracts/compare/v0.13.3...v0.13.4
[0.13.3]: https://github.com/noislabs/nois-contracts/compare/v0.13.2...v0.13.3
[0.13.2]: https://github.com/noislabs/nois-contracts/compare/v0.13.1...v0.13.2
[0.13.1]: https://github.com/noislabs/nois-contracts/compare/v0.13.0...v0.13.1
[0.13.0]: https://github.com/noislabs/nois-contracts/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/noislabs/nois-contracts/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/noislabs/nois-contracts/compare/v0.10.2...v0.11.0
[0.10.2]: https://github.com/noislabs/nois-contracts/compare/v0.10.1...v0.10.2
[0.10.1]: https://github.com/noislabs/nois-contracts/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/noislabs/nois-contracts/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/noislabs/nois-contracts/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/noislabs/nois-contracts/tree/v0.8.0
