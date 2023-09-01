/* eslint-disable @typescript-eslint/ban-types */
/* eslint-disable @typescript-eslint/no-empty-interface */
import { readFileSync } from "fs";

import { CosmWasmSigner } from "@confio/relayer";
import { ExecutionContext } from "ava";
import { Coin } from "cosmjs-types/cosmos/base/v1beta1/coin";

export interface IcecubeInstantiateMsg {
  readonly manager: string;
}

export interface IcecubeExecuteMsg {
  readonly delegate?: {
    readonly addr: string;
    readonly amount: string;
  };
  readonly undelegate?: {
    readonly addr: string;
    readonly amount: string;
  };
  readonly redelegate?: {
    readonly src_addr: string;
    readonly dest_addr: string;
    readonly amount: string;
  };
  // ... some more options, see contract
}

export interface DrandInstantiateMsg {
  readonly manager: string;
  readonly min_round: number;
  readonly incentive_point_price: string;
  readonly incentive_denom: string;
}

export interface DrandExecuteMsg {
  readonly add_round?: {
    readonly round: number;
    readonly signature: string;
  };
  readonly register_bot?: {
    readonly moniker: string;
  };
  readonly set_gateway_addr?: { addr: string };
}

export interface SinkInstantiateMsg {}

export interface GatewayInstantiateMsg {
  readonly manager: string;
  readonly price: Coin;
  readonly payment_code_id: number;
  readonly payment_initial_funds?: null | Coin;
  /** Address of the Nois sink */
  readonly sink: string;
}

export interface GatewayExecuteMsg {
  readonly add_verified_round?: {
    readonly round: number;
    readonly randomness: string;
    readonly is_verifying_tx: boolean;
  };
  readonly set_config?: {
    readonly manager?: null | string;
    readonly price?: null | Coin;
    readonly drand_addr?: null | string;
    readonly payment_initial_funds?: null | Coin;
    readonly trusted_sources?: null | string[];
  };
}

export interface GatewayQueriedCustomer {
  readonly channel_id: string;
  readonly payment: string;
  readonly requested_beacons: number;
}

export interface GatewayCustomerResponse {
  readonly customer: null | GatewayQueriedCustomer;
}

export interface IbcDenom {
  readonly ics20_channel: string;
  /** The ibc/* denom for the token */
  readonly denom: string;
}

export interface ProxyOperationalMode {
  readonly funded?: {};
  readonly ibc_pay?: {
    readonly unois_denom: IbcDenom;
  };
}

export interface ProxyInstantiateMsg {
  readonly manager: null | string;
  readonly prices: Array<Coin>;
  readonly test_mode?: null | boolean;
  readonly callback_gas_limit: number;
  readonly mode: ProxyOperationalMode;
  readonly allowlist_enabled?: null | boolean;
  readonly allowlist?: null | string[];
}

export interface ProxyExecuteMsg {
  readonly get_next_randomness?: {
    readonly job_id: string;
  };
  readonly get_randomness_after?: {
    readonly job_id: string;
    /** Timestamp in nanoseconds since epoch */
    readonly after: string;
  };
  readonly withdraw?: {
    readonly denom: string;
    readonly amount: null | string;
    readonly address: string;
  };
  readonly set_config?: {
    readonly min_after?: null | string;
    readonly max_after?: null | string;
    // ... some more options, see contract
  };
  // ... some more options, see contract
}

export interface WasmdContractPaths {
  readonly proxy: string;
  readonly demo: string;
}

export interface NoisContractPaths {
  readonly icecube: string;
  readonly gateway: string;
  readonly drand: string;
  readonly payment: string;
  readonly sink: string;
}

export const wasmContracts: WasmdContractPaths = {
  proxy: "./internal/nois_proxy.wasm",
  demo: "./internal/nois_demo.wasm",
};

export const noisContracts: NoisContractPaths = {
  icecube: "./internal/nois_icecube.wasm",
  gateway: "./internal/nois_gateway.wasm",
  drand: "./internal/nois_drand.wasm",
  payment: "./internal/nois_payment.wasm",
  sink: "./internal/nois_sink.wasm",
};

export async function uploadContracts(
  t: ExecutionContext,
  cosmwasm: CosmWasmSigner,
  contracts: WasmdContractPaths | NoisContractPaths,
  enable: (keyof WasmdContractPaths | keyof NoisContractPaths)[]
): Promise<Record<string, number>> {
  const results: Record<string, number> = {};

  for (const [name, path] of Object.entries(contracts)) {
    if (!(enable as string[]).includes(name)) {
      results[name] = Number.NaN;
      continue;
    }
    t.log(`Storing ${name} from ${path}...`);
    const wasm = readFileSync(path);
    const multiplier = 1.1; // see https://github.com/cosmos/cosmjs/issues/1360
    const receipt = await cosmwasm.sign.upload(cosmwasm.senderAddress, wasm, multiplier, `Upload ${name}`);
    t.log(`Uploaded ${name} with code ID: ${receipt.codeId}; Gas used: ${receipt.gasUsed}/${receipt.gasWanted}`);
    results[name] = receipt.codeId;
  }

  return results;
}
