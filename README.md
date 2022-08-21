# Nois Contracts

This is the development repo for the Nois contracts, including a fully featured test environment
that allows testing contract to contract IBC communication.

## The chains

There are two CosmWasm-enabled blockchains running locally.

1. **The randomness chain:** This is where randomness is verified and distributed from.
   Currently implemented using an instance of osmosisd but this could be swapped for any
   other ComsWasm chain.
2. **The app chain:** This is where the users deploy their contracts and request the
   randomness from. Currently this uses wasmd. An example for app chains in production would
   be Juno, Terra or Tgrade.

## The contracts

- nois-terrand (runs on the randomness chain; one instance globally)
- nois-proxy (runs on the app chain; one instance per app chain)
- nois-demo (runs on the app chain; a demo app)

The IBC interaction is only between nois-terrand and nois-proxy, such that
the user (nois-demo) does not need to worry about that part.

## Development

Follow all those steps to test things.

### Build the contracts

The repo root is a Rust workspace containing all the contracts.
Basic tests can be run like this:

```
cargo build --all-targets
cargo clippy --all-targets
cargo fmt
```

The production grade Wasm builds are compiled with:

```
./devtools/build_integration_wasm.sh
```

### Starting/stopping the chains

In terminal 1 run:

```
./ci-scripts/osmosis/start.sh
```

which will log in `debug-osmosis.log`.

In terminal 2 run:

```
./ci-scripts/wasmd/start.sh
```

which will log in `debug-wasmd.log`.

With `docker ps` you can see the running chains. `docker ps osmosis` and `docker ps wasmd` allows you to stop them.

### Run tests

The tests are written in JavaScript in the `./tests` folder

```
cd tests
npm install
npm run test
```

That's it 🎉
