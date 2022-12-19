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

export interface OracleInstantiateMsg {
  readonly min_round: number;
  readonly incentive_amount: string;
  readonly incentive_denom: string;
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
  readonly oracle: string;
  readonly drand: string;
}

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
