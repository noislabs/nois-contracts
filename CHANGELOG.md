# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changes

- Bump drand-verify to 0.4, using pairing for the BLS verification.
- Reduce the number of verification executions per round from 6 to 3 to increase the number of processable jobs.
- Introduce reward point system for drand submissions.
- Upgrade CosmWasm to 1.2.
- icecube: Rename `admin` to `manager`.

## [0.8.0]

Base version for starting the CHANGELOG.

[unreleased]: https://github.com/noislabs/nois-contracts/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/noislabs/nois-contracts/tree/v0.8.0
