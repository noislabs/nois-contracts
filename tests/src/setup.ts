import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
import { coin, coins } from "@cosmjs/amino";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { assertIsDeliverTxSuccess, GasPrice, SigningStargateClient } from "@cosmjs/stargate";
import { assert } from "@cosmjs/utils";
import { ExecutionContext } from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";
import Long from "long";

import {
  GatewayExecuteMsg,
  GatewayInstantiateMsg,
  NoisContractPaths,
  ProxyExecuteMsg,
  ProxyInstantiateMsg,
  ProxyOperationalMode,
  SinkInstantiateMsg,
  WasmdContractPaths,
} from "./contracts";
import { assertPacketsFromB, ibcDenom, nois, NoisProtocolIbcVersion, setupNoisClient, setupWasmClient } from "./utils";

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
  /// Address on Nois
  sinkAddress: string | undefined;
  link: Link;
  noisChannel: {
    wasmChannelId: string;
    noisChannelId: string;
  };
  ics20Channel: {
    wasmChannelId: string;
    noisChannelId: string;
  };
  realyerWasm: string;
  realyerNois: string;
}

export interface InstantiateAndConnectOptions {
  readonly testMode?: boolean;
  /** Set this drand address in the gateway. Don't set any address if undefined. */
  readonly mockDrandAddr?: string;
  readonly callback_gas_limit?: number;
  readonly enablePayment?: "funded" | "ibc_pay"; // defaults to false
}

export async function instantiateAndConnectIbc(
  t: ExecutionContext,
  options: InstantiateAndConnectOptions
): Promise<SetupInfo> {
  const context = t.context as TestContext;
  const [wasmClient, noisClient] = await Promise.all([setupWasmClient(), setupNoisClient()]);

  // Create a connection between the chains
  const [src, dest] = await setup(wasmd, nois);
  const link = await Link.createWithNewConnections(src, dest);

  // Create an ics20 channel
  const ics20Info = await link.createChannel("A", wasmd.ics20Port, nois.ics20Port, Order.ORDER_UNORDERED, "ics20-1");
  const ics20Channel = {
    wasmChannelId: ics20Info.src.channelId,
    noisChannelId: ics20Info.dest.channelId,
  };
  const unoisOnWasm = ibcDenom(ics20Channel.wasmChannelId, "unois");

  const mode: ProxyOperationalMode =
    options.enablePayment === "ibc_pay"
      ? { ibc_pay: { unois_denom: { ics20_channel: ics20Channel.wasmChannelId, denom: unoisOnWasm } } }
      : { funded: {} };

  // Instantiate proxy on appchain
  const proxyMsg: ProxyInstantiateMsg = {
    manager: wasmClient.senderAddress,
    prices: coins(1_000_000, "ucosm"),
    test_mode: options.testMode ?? true,
    callback_gas_limit: options.callback_gas_limit ?? 500_000,
    mode,
  };
  const { contractAddress: noisProxyAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    context.wasmCodeIds.proxy,
    proxyMsg,
    "Proxy instance",
    "auto",
    { funds: coins(1_000, "ucosm") } // some funds to test withdrawals
  );

  const updateProxyConfig: ProxyExecuteMsg = {
    set_config: {
      // drand genesis https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info
      // to allow old rounds in tests
      min_after: "1677685200000000000",
    },
  };
  await wasmClient.sign.execute(wasmClient.senderAddress, noisProxyAddress, updateProxyConfig, "auto");

  if (options.enablePayment == "ibc_pay") {
    // fund the proxy such that it can pay in NOIS
    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(nois.faucet.mnemonic, { prefix: nois.prefix });
    const address = (await wallet.getAccounts())[0].address;
    const noisClient = await SigningStargateClient.connectWithSigner(nois.tendermintUrlHttp, wallet, {
      gasPrice: GasPrice.fromString(nois.minFee),
    });

    const wasmClient = await setupWasmClient();
    const res = await noisClient.sendIbcTokens(
      address,
      noisProxyAddress,
      coin(2 * 50_000000, "unois"),
      nois.ics20Port,
      ics20Channel.noisChannelId,
      { revisionHeight: Long.fromNumber((await wasmClient.sign.getHeight()) + 100), revisionNumber: Long.UONE },
      undefined,
      "auto",
      "funds to the other chain"
    );
    assertIsDeliverTxSuccess(res);

    const transferInfo = await link.relayAll();
    assertPacketsFromB(transferInfo, 1, true);
  }

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
    price: coin(options.enablePayment ? 50_000000 : 0, "unois"),
    payment_code_id: context.noisCodeIds.payment,
    payment_initial_funds: options.enablePayment == "funded" ? coin(100_000000, "unois") : null, // enough to pay 2 beacon requests
    sink: sinkAddress ?? "nois1ffy2rz96sjxzm2ezwkmvyeupktp7elt6w3xckt",
  };
  const { contractAddress: noisGatewayAddress } = await noisClient.sign.instantiate(
    noisClient.senderAddress,
    context.noisCodeIds.gateway,
    instantiateMsg,
    "Gateway instance",
    "auto"
  );
  if (options.enablePayment == "funded") {
    await fundAccount(nois, noisGatewayAddress, "100000000"); // 100 NOIS can fund 1 payment contracts
  }

  if (options.mockDrandAddr) {
    const setConfigMsg: GatewayExecuteMsg = { set_config: { trusted_sources: [options.mockDrandAddr] } };
    await noisClient.sign.execute(noisClient.senderAddress, noisGatewayAddress, setConfigMsg, "auto");
  }

  const [noisProxyInfo, noisGatewayInfo] = await Promise.all([
    wasmClient.sign.getContract(noisProxyAddress),
    noisClient.sign.getContract(noisGatewayAddress),
  ]);
  const { ibcPortId: proxyPort } = noisProxyInfo;
  assert(proxyPort);
  const { ibcPortId: gatewayPort } = noisGatewayInfo;
  assert(gatewayPort);

  // Create a channel for the Nois protocol
  const info = await link.createChannel("A", proxyPort, gatewayPort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
  const noisChannel = {
    wasmChannelId: info.src.channelId,
    noisChannelId: info.dest.channelId,
  };
  const info2 = await link.relayAll();
  assertPacketsFromB(info2, 2, true); // Welcome+PushBeaconPrice packet

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
    sinkAddress,
    link,
    noisChannel,
    ics20Channel,
    realyerWasm: src.senderAddress,
    realyerNois: dest.senderAddress,
  };
}
