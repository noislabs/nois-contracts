import { readFileSync } from "fs";

import { CosmWasmSigner } from "@confio/relayer";
import { ExecutionContext } from "ava";
import { Coin } from "cosmjs-types/cosmos/base/v1beta1/coin";

export interface DelegatorInstantiateMsg {
  readonly admin_addr: string;
}

export interface DrandInstantiateMsg {
  readonly manager: string;
  readonly min_round: number;
  readonly incentive_amount: string;
  readonly incentive_denom: string;
}

export interface DrandExecuteMsg {
  readonly add_round?: {
    readonly round: number;
    readonly signature: string;
    readonly previous_signature: string;
  };
  readonly register_bot?: {
    readonly moniker: string;
  };
  readonly set_gateway_addr?: { addr: string };
}

// eslint-disable-next-line @typescript-eslint/no-empty-interface
export interface GatewayInstantiateMsg {}

export interface GatewayExecuteMsg {
  readonly add_verified_round: {
    readonly round: number;
    readonly randomness: string;
  };
}

export interface ProxyInstantiateMsg {
  readonly prices: Array<Coin>;
  readonly withdrawal_address: string;
  readonly test_mode: boolean;
}

export interface WasmdContractPaths {
  readonly proxy: string;
  readonly demo: string;
}

export interface OsmosisContractPaths {
  readonly delegator: string;
  readonly gateway: string;
  readonly drand: string;
}

export const wasmContracts: WasmdContractPaths = {
  proxy: "./internal/nois_proxy.wasm",
  demo: "./internal/nois_demo.wasm",
};

export const osmosisContracts: OsmosisContractPaths = {
  delegator: "./internal/nois_delegator.wasm",
  gateway: "./internal/nois_gateway.wasm",
  drand: "./internal/nois_drand.wasm",
};

export async function uploadContracts(
  t: ExecutionContext,
  cosmwasm: CosmWasmSigner,
  contracts: WasmdContractPaths | OsmosisContractPaths
): Promise<Record<string, number>> {
  const results: Record<string, number> = {};

  for (const [name, path] of Object.entries(contracts)) {
    t.log(`Storing ${name} from ${path}...`);
    const wasm = readFileSync(path);
    const receipt = await cosmwasm.sign.upload(cosmwasm.senderAddress, wasm, "auto", `Upload ${name}`);
    t.log(`Uploaded ${name} with code ID: ${receipt.codeId}; Gas used: ${receipt.gasUsed}/${receipt.gasWanted}`);
    results[name] = receipt.codeId;
  }

  return results;
}
