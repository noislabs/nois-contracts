import { AckWithMetadata, CosmWasmSigner, RelayInfo, testutils } from "@confio/relayer";
import { ChainDefinition } from "@confio/relayer/build/lib/helpers";
import { fromBinary } from "@cosmjs/cosmwasm-stargate";
import { fromUtf8 } from "@cosmjs/encoding";
import { assert } from "@cosmjs/utils";

const { fundAccount, generateMnemonic, signingCosmWasmClient, wasmd } = testutils;

export const nois: ChainDefinition = {
  tendermintUrlWs: "ws://localhost:26655",
  tendermintUrlHttp: "http://localhost:26655",
  chainId: "noisd-1",
  prefix: "nois",
  denomStaking: "unois",
  denomFee: "unois",
  minFee: "0.025unois",
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

export const NoisProtocolIbcVersion = "nois-v5";

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
      throw new Error(`Unexpected error in ack: ${parsed.error}`);
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
