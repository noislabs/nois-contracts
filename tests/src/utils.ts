import { AckWithMetadata, CosmWasmSigner, RelayInfo, testutils } from "@confio/relayer";
import { ChainDefinition } from "@confio/relayer/build/lib/helpers";
import { fromBinary, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { Random, sha256 } from "@cosmjs/crypto";
import { fromBase64, fromUtf8, toAscii, toBech32, toHex } from "@cosmjs/encoding";
import {
  Coin,
  decodeCosmosSdkDecFromProto,
  QueryClient,
  setupBankExtension,
  setupDistributionExtension,
} from "@cosmjs/stargate";
import { assert } from "@cosmjs/utils";

const { fundAccount, generateMnemonic, signingCosmWasmClient, wasmd } = testutils;

export const nois: ChainDefinition = {
  tendermintUrlWs: "ws://localhost:26655",
  tendermintUrlHttp: "http://localhost:26655",
  chainId: "noisd-1",
  prefix: "nois",
  denomStaking: "unois",
  denomFee: "unois",
  minFee: "0.05unois",
  blockTime: 250, // ms
  faucet: {
    mnemonic: "camera rice drop advance success club primary wonder diary inside raw tool",
    // TODO: update pubkey
    pubkey0: {
      type: "tendermint/PubKeySecp256k1",
      value: "A9cXhWb8ZpqCzkA8dQCPV29KdeRLV3rUYxrkHudLbQtS",
    },
    address0: "nois1hqg5nqnka82cwm3v02xj6ufns9tmss7rlpvucx",
  },
  ics20Port: "transfer",
  estimatedBlockTime: 400,
  estimatedIndexerTime: 80,
};

/* Queries the community pool funds in full unois. */
export async function communityPoolFunds(client: SigningCosmWasmClient): Promise<number> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const tmClient = (client as any).forceGetTmClient();
  const queryClient = QueryClient.withExtensions(tmClient, setupDistributionExtension);
  const resp = await queryClient.distribution.communityPool();
  const unois = resp.pool.find((coin) => coin.denom === "unois");
  if (!unois) {
    return 0;
  } else {
    return decodeCosmosSdkDecFromProto(unois.amount).floor().toFloatApproximation();
  }
}

/* Queries the community pool funds in full unois. */
export async function totalSupply(client: SigningCosmWasmClient): Promise<Coin> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const tmClient = (client as any).forceGetTmClient();
  const queryClient = QueryClient.withExtensions(tmClient, setupBankExtension);
  return queryClient.bank.supplyOf("unois");
}

export const noisValidator = {
  // cat ci-scripts/nois/template/.noisd/config/genesis.json | jq '.app_state.genutil.gen_txs[0].body.messages[0].validator_address' -r
  address: "noisvaloper13k69ev2re0vlk952cf8cnuua5znhvv7dvrayrm",
};

export const NoisProtocolIbcVersion = "nois-v7";

// This creates a client for the CosmWasm chain, that can interact with contracts
export async function setupWasmClient(): Promise<CosmWasmSigner> {
  // create apps and fund an account
  const mnemonic = generateMnemonic();
  const cosmwasm = await signingCosmWasmClient(wasmd, mnemonic);
  await fundAccount(wasmd, cosmwasm.senderAddress, "4000000");
  return cosmwasm;
}

// This creates a client for the CosmWasm chain, that can interact with contracts
export async function setupNoisClient(): Promise<CosmWasmSigner> {
  // create apps and fund an account
  const mnemonic = generateMnemonic();
  const cosmwasm = await signingCosmWasmClient(nois, mnemonic);
  await fundAccount(nois, cosmwasm.senderAddress, "4000000");
  return cosmwasm;
}

// throws error if not all are success
export function assertAckSuccess(acks: AckWithMetadata[]) {
  for (const ack of acks) {
    const parsed = JSON.parse(fromUtf8(ack.acknowledgement));
    if (parsed.error) {
      throw new Error(`Unexpected error in ack: ${parsed.error}; Events: ${JSON.stringify(ack.txEvents)}`);
    }
    if (!parsed.result) {
      throw new Error(`Ack result unexpectedly empty`);
    }
  }
}

// throws error if not all are errors
export function assertAckErrors(acks: AckWithMetadata[]) {
  for (const ack of acks) {
    const parsed = JSON.parse(fromUtf8(ack.acknowledgement));
    if (parsed.result) {
      throw new Error(`Ack result unexpectedly set`);
    }
    if (!parsed.error) {
      throw new Error(`Ack error unexpectedly empty`);
    }
  }
}

export function assertPacketsFromA(relay: RelayInfo, count: number, success: boolean) {
  if (relay.packetsFromA !== count) {
    throw new Error(`Expected ${count} packets, got ${relay.packetsFromA}`);
  }
  if (relay.acksFromB.length !== count) {
    throw new Error(`Expected ${count} acks, got ${relay.acksFromB.length}`);
  }
  if (success) {
    assertAckSuccess(relay.acksFromB);
  } else {
    assertAckErrors(relay.acksFromB);
  }
}

export function assertPacketsFromB(relay: RelayInfo, count: number, success: boolean) {
  if (relay.packetsFromB !== count) {
    throw new Error(`Expected ${count} packets, got ${relay.packetsFromB}`);
  }
  if (relay.acksFromA.length !== count) {
    throw new Error(`Expected ${count} acks, got ${relay.acksFromA.length}`);
  }
  if (success) {
    assertAckSuccess(relay.acksFromA);
  } else {
    assertAckErrors(relay.acksFromA);
  }
}

interface IbcAcknowledgement {
  /** base64 data */
  data: string;
}

/** See cosmwasm_std::IbcPacketAckMsg */
interface IbcPacketAckMsg {
  acknowledgement: IbcAcknowledgement;
  original_packet: unknown;
  relayer: string;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function parseIbcPacketAckMsg(m: IbcPacketAckMsg): any {
  const stdAck = fromBinary(m.acknowledgement.data);
  const result = stdAck.result;
  assert(typeof result === "string");
  return fromBinary(result);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function successAckToJson(acknowledgement: Uint8Array): any {
  const data = successAckToData(acknowledgement);
  return JSON.parse(fromUtf8(data));
}

export function successAckToData(acknowledgement: Uint8Array): Uint8Array {
  const stdAck = JSON.parse(fromUtf8(acknowledgement));
  const result = stdAck.result;
  assert(typeof result === "string");
  assert(result.length !== 0, "Result data must not be empty");
  return fromBase64(result);
}

export function randomAddress(prefix: string): string {
  const random = Random.getBytes(20);
  return toBech32(prefix, random);
}

export function ibcDenom(sourceChannel: string, originalDenom: string): string {
  return "ibc/" + toHex(sha256(toAscii(`transfer/${sourceChannel}/${originalDenom}`))).toUpperCase();
}
