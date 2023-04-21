import { Link, testutils } from "@confio/relayer";
import { coin, coins } from "@cosmjs/amino";
import { ExecuteInstruction, fromBinary } from "@cosmjs/cosmwasm-stargate";
import { fromUtf8 } from "@cosmjs/encoding";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

import { ibcPacketsSent, MockBot } from "./bot";
import { GatewayInstantiateMsg, noisContracts, ProxyInstantiateMsg, uploadContracts, wasmContracts } from "./contracts";
import { instantiateAndConnectIbc, TestContext } from "./setup";
import {
  assertPacketsFromA,
  assertPacketsFromB,
  nois,
  NoisProtocolIbcVersion,
  setupNoisClient,
  setupWasmClient,
} from "./utils";

const { setup, wasmd, fundAccount } = testutils;

test.before(async (t) => {
  const [wasmClient, noisClient] = await Promise.all([setupWasmClient(), setupNoisClient()]);
  t.log("Upload contracts ...");
  const [wasmCodeIds, noisCodeIds] = await Promise.all([
    uploadContracts(t, wasmClient, wasmContracts, ["demo", "proxy"]),
    uploadContracts(t, noisClient, noisContracts, ["gateway", "payment"]),
  ]);
  const context: TestContext = {
    wasmCodeIds,
    noisCodeIds,
  };
  t.context = context;
  t.pass();
});

test.serial("set up nois channel", async (t) => {
  const context = t.context as TestContext;

  const [src, dest] = await setup(wasmd, nois);
  const link = await Link.createWithNewConnections(src, dest);

  // Instantiate proxy on appchain
  const wasmClient = await setupWasmClient();
  const proxyMsg: ProxyInstantiateMsg = {
    manager: wasmClient.senderAddress,
    prices: coins(1_000_000, "ucosm"),
    test_mode: true,
    callback_gas_limit: 500_000,
    mode: {
      funded: {},
    },
  };
  const { contractAddress: proxyAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    context.wasmCodeIds.proxy,
    proxyMsg,
    "Proxy instance",
    "auto"
  );
  t.truthy(proxyAddress);
  const { ibcPortId: proxyPort } = await wasmClient.sign.getContract(proxyAddress);
  t.log(`Proxy port: ${proxyPort}`);
  assert(proxyPort);

  const noisClient = await setupNoisClient();
  const msg: GatewayInstantiateMsg = {
    manager: noisClient.senderAddress,
    price: coin(0, "unois"),
    payment_code_id: context.noisCodeIds.payment,
    payment_initial_funds: null,
    // any dummy address is good here because we only test channel creation
    sink: "nois1ffy2rz96sjxzm2ezwkmvyeupktp7elt6w3xckt",
  };
  const { contractAddress: gatewayAddress } = await noisClient.sign.instantiate(
    noisClient.senderAddress,
    context.noisCodeIds.gateway,
    msg,
    "Gateway instance",
    "auto"
  );
  t.truthy(gatewayAddress);
  const { ibcPortId: gatewayPort } = await noisClient.sign.getContract(gatewayAddress);
  t.log(`Gateway port: ${gatewayPort}`);
  assert(gatewayPort);

  await link.createChannel("A", proxyPort, gatewayPort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
  const info2 = await link.relayAll();
  // Welcome+PushBeaconPrice packet
  assertPacketsFromB(info2, 2, true);
  const ackWelcome = JSON.parse(fromUtf8(info2.acksFromA[0].acknowledgement));
  t.deepEqual(fromBinary(ackWelcome.result), { welcome: {} });
  const ackPushBeaconPrice = JSON.parse(fromUtf8(info2.acksFromA[1].acknowledgement));
  t.deepEqual(fromBinary(ackPushBeaconPrice.result), { push_beacon_price: {} });
});

test.before(async (t) => {
  const [wasmClient, noisClient] = await Promise.all([setupWasmClient(), setupNoisClient()]);
  t.log("Upload contracts ...");
  const [wasmCodeIds, noisCodeIds] = await Promise.all([
    uploadContracts(t, wasmClient, wasmContracts, ["proxy"]),
    uploadContracts(t, noisClient, noisContracts, ["drand", "gateway", "payment"]),
  ]);
  const context: TestContext = {
    wasmCodeIds,
    noisCodeIds,
  };
  t.context = context;
  t.pass();
});

test.serial("submit randomness for various job counts", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(t, {
    testMode: false,
    mockDrandAddr: bot.address,
  });
  bot.setGatewayAddress(noisGatewayAddress);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  await fundAccount(wasmd, wasmClient.senderAddress, "40000000");

  function before(beaconReleaseTimestamp: string): string {
    return (BigInt(beaconReleaseTimestamp) - BigInt(1)).toString();
  }

  const afterValues = [
    before("1677687597000000000"), // round 800
    before("1677687627000000000"), // round 810
    before("1677687657000000000"), // round 820
    before("1677687687000000000"), // round 830
    before("1677687717000000000"), // round 840
    before("1677687747000000000"), // round 850
    before("1677687777000000000"), // round 860
    before("1677687807000000000"), // round 870
  ];

  for (const [i, jobs] of [0, 1, 2, 3, 4].entries()) {
    t.log(`Executing get_next_randomness ${jobs} times for a round that does not yet exists`);

    const msgs = Array.from({ length: jobs }).map(
      (_, j): ExecuteInstruction => ({
        contractAddress: noisProxyAddress,
        msg: { get_randomness_after: { after: afterValues[i], job_id: `job-${j}` } },
        funds: [payment],
      })
    );
    if (msgs.length > 0) {
      await wasmClient.sign.executeMultiple(wasmClient.senderAddress, msgs, "auto");
    }

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, jobs, true);

    const result = await bot.submitNext();
    t.log(`Gas: ${result.gasUsed}/${result.gasWanted}`);
    const packetsSentCount = ibcPacketsSent(result.logs);
    t.log("Number of packets sent:", packetsSentCount);
    t.is(packetsSentCount, Math.min(jobs, 2));
  }
});
