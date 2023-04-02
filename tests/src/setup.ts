import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
import { coin, coins } from "@cosmjs/amino";
import { assert } from "@cosmjs/utils";
import { ExecutionContext } from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

import {
  GatewayExecuteMsg,
  GatewayInstantiateMsg,
  NoisContractPaths,
  ProxyInstantiateMsg,
  SinkInstantiateMsg,
  WasmdContractPaths,
} from "./contracts";
import { assertPacketsFromB, nois, NoisProtocolIbcVersion, setupNoisClient, setupWasmClient } from "./utils";

const { setup, wasmd, fundAccount } = testutils;

export interface TestContext {
  wasmCodeIds: Record<keyof WasmdContractPaths, number>;
  noisCodeIds: Record<keyof NoisContractPaths, number>;
}

export interface SetupInfo {
  wasmClient: CosmWasmSigner;
  noisClient: CosmWasmSigner;
  /// Address on app chain (wasmd)
  noisProxyAddress: string;
  /// Address on app chain (wasmd)
  noisDemoAddress: string | undefined;
  /// Address on Nois
  noisGatewayAddress: string;
  link: Link;
  noisChannel: {
    wasmChannelId: string;
    osmoChannelId: string;
  };
  ics20Channel: {
    wasmChannelId: string;
    osmoChannelId: string;
  };
}

export interface InstantiateAndConnectOptions {
  readonly testMode?: boolean;
  readonly mockDrandAddr: string;
  readonly callback_gas_limit?: number;
  readonly enablePayment?: boolean; // defaults to false
}

export async function instantiateAndConnectIbc(
  t: ExecutionContext,
  options: InstantiateAndConnectOptions
): Promise<SetupInfo> {
  const context = t.context as TestContext;
  const [wasmClient, noisClient] = await Promise.all([setupWasmClient(), setupNoisClient()]);

  // Instantiate proxy on appchain
  const proxyMsg: ProxyInstantiateMsg = {
    prices: coins(1_000_000, "ucosm"),
    withdrawal_address: wasmClient.senderAddress,
    test_mode: options.testMode ?? true,
    callback_gas_limit: options.callback_gas_limit ?? 500_000,
  };
  const { contractAddress: noisProxyAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    context.wasmCodeIds.proxy,
    proxyMsg,
    "Proxy instance",
    "auto"
  );

  // Instantiate sink on Nois
  let sinkAddress: string | undefined;
  if (options.enablePayment) {
    const sinkMsg: SinkInstantiateMsg = {};
    const { contractAddress } = await noisClient.sign.instantiate(
      noisClient.senderAddress,
      context.noisCodeIds.sink,
      sinkMsg,
      "Sink instance",
      "auto"
    );
    sinkAddress = contractAddress;
  }

  // Instantiate Gateway on Nois
  const instantiateMsg: GatewayInstantiateMsg = {
    manager: noisClient.senderAddress,
    price: coin(options.enablePayment ? 100 : 0, "unois"),
    payment_code_id: context.noisCodeIds.payment,
    payment_initial_funds: coin(options.enablePayment ? 500 : 0, "unois"), // enough to pay 5 beacon requests
    sink: sinkAddress ?? "nois1ffy2rz96sjxzm2ezwkmvyeupktp7elt6w3xckt",
  };
  const { contractAddress: noisGatewayAddress } = await noisClient.sign.instantiate(
    noisClient.senderAddress,
    context.noisCodeIds.gateway,
    instantiateMsg,
    "Gateway instance",
    "auto"
  );
  if (options.enablePayment) {
    await fundAccount(nois, noisGatewayAddress, "1500"); // 1500 unois can fund 3 payment contracts
  }

  const setDrandMsg: GatewayExecuteMsg = { set_config: { drand_addr: options.mockDrandAddr } };
  await noisClient.sign.execute(noisClient.senderAddress, noisGatewayAddress, setDrandMsg, "auto");

  const [noisProxyInfo, noisGatewayInfo] = await Promise.all([
    wasmClient.sign.getContract(noisProxyAddress),
    noisClient.sign.getContract(noisGatewayAddress),
  ]);
  const { ibcPortId: proxyPort } = noisProxyInfo;
  assert(proxyPort);
  const { ibcPortId: gatewayPort } = noisGatewayInfo;
  assert(gatewayPort);

  // Create a connection between the chains
  const [src, dest] = await setup(wasmd, nois);
  const link = await Link.createWithNewConnections(src, dest);

  // Create a channel for the Nois protocol
  const info = await link.createChannel("A", proxyPort, gatewayPort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
  const noisChannel = {
    wasmChannelId: info.src.channelId,
    osmoChannelId: info.dest.channelId,
  };
  const info2 = await link.relayAll();
  assertPacketsFromB(info2, 1, true); // Welcome packet

  // Also create a ics20 channel
  const ics20Info = await link.createChannel("A", wasmd.ics20Port, nois.ics20Port, Order.ORDER_UNORDERED, "ics20-1");
  const ics20Channel = {
    wasmChannelId: ics20Info.src.channelId,
    osmoChannelId: ics20Info.dest.channelId,
  };

  // Instantiate demo app
  let noisDemoAddress: string | undefined;
  if (context.wasmCodeIds.demo) {
    const { contractAddress } = await wasmClient.sign.instantiate(
      wasmClient.senderAddress,
      context.wasmCodeIds.demo,
      { nois_proxy: noisProxyAddress },
      "A demo contract",
      "auto"
    );
    noisDemoAddress = contractAddress;
  }

  return {
    wasmClient,
    noisClient,
    noisProxyAddress,
    noisDemoAddress,
    noisGatewayAddress,
    link,
    noisChannel,
    ics20Channel,
  };
}
