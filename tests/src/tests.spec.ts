import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
import { coin, coins } from "@cosmjs/amino";
import { ExecuteInstruction, fromBinary, toBinary } from "@cosmjs/cosmwasm-stargate";
import { fromUtf8 } from "@cosmjs/encoding";
import { Decimal } from "@cosmjs/math";
import { assert } from "@cosmjs/utils";
import test, { ExecutionContext } from "ava";
import { Coin } from "cosmjs-types/cosmos/base/v1beta1/coin";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

import { ibcPacketsSent, MockBot } from "./bot";
import {
  GatewayExecuteMsg,
  GatewayInstantiateMsg,
  NoisContractPaths,
  noisContracts,
  ProxyInstantiateMsg,
  SinkInstantiateMsg,
  uploadContracts,
  wasmContracts,
  WasmdContractPaths,
} from "./contracts";
import {
  assertPacketsFromA,
  assertPacketsFromB,
  communityPoolFunds,
  nois,
  NoisProtocolIbcVersion,
  setupNoisClient,
  setupWasmClient,
} from "./utils";

const { setup, wasmd, fundAccount } = testutils;

interface TestContext {
  wasmCodeIds: Record<keyof WasmdContractPaths, number>;
  noisCodeIds: Record<keyof NoisContractPaths, number>;
}

test.before(async (t) => {
  const [wasmClient, noisClient] = await Promise.all([setupWasmClient(), setupNoisClient()]);
  t.log("Upload contracts ...");
  const [wasmCodeIds, noisCodeIds] = await Promise.all([
    uploadContracts(t, wasmClient, wasmContracts),
    uploadContracts(t, noisClient, noisContracts),
  ]);
  const context: TestContext = {
    wasmCodeIds,
    noisCodeIds,
  };
  t.context = context;
  t.pass();
});

test.serial("set up channel", async (t) => {
  const context = t.context as TestContext;

  // Instantiate proxy on appchain
  const wasmClient = await setupWasmClient();
  const proxyMsg: ProxyInstantiateMsg = {
    prices: coins(1_000_000, "ucosm"),
    withdrawal_address: wasmClient.senderAddress,
    test_mode: true,
    callback_gas_limit: 500_000,
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
    payment_initial_funds: coin(0, "unois"),
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

  const [src, dest] = await setup(wasmd, nois);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", proxyPort, gatewayPort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
  const info2 = await link.relayAll();
  assertPacketsFromB(info2, 1, true); // Welcome packet
});

interface SetupInfo {
  wasmClient: CosmWasmSigner;
  noisClient: CosmWasmSigner;
  /// Address on app chain (wasmd)
  noisProxyAddress: string;
  /// Address on app chain (wasmd)
  noisDemoAddress: string;
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

interface InstantiateAndConnectOptions {
  readonly testMode?: boolean;
  readonly mockDrandAddr: string;
  readonly callback_gas_limit?: number;
}

async function instantiateAndConnectIbc(
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
    (t.context as TestContext).wasmCodeIds.proxy,
    proxyMsg,
    "Proxy instance",
    "auto"
  );

  // Instantiate sink on Nois
  const sinkMsg: SinkInstantiateMsg = {};
  const { contractAddress: sinkAddress } = await noisClient.sign.instantiate(
    noisClient.senderAddress,
    context.noisCodeIds.sink,
    sinkMsg,
    "Sink instance",
    "auto"
  );

  // Instantiate Gateway on Nois
  const instantiateMsg: GatewayInstantiateMsg = {
    manager: noisClient.senderAddress,
    price: coin(100, "unois"),
    payment_code_id: context.noisCodeIds.payment,
    payment_initial_funds: coin(500, "unois"), // enough to pay 5 beacon requests
    sink: sinkAddress,
  };
  const { contractAddress: noisGatewayAddress } = await noisClient.sign.instantiate(
    noisClient.senderAddress,
    context.noisCodeIds.gateway,
    instantiateMsg,
    "Gateway instance",
    "auto"
  );
  await fundAccount(nois, noisGatewayAddress, "1500"); // 1500 unois can fund 3 payment contracts

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
  const { contractAddress: noisDemoAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    context.wasmCodeIds.demo,
    { nois_proxy: noisProxyAddress },
    "A demo contract",
    "auto"
  );

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

test.serial("proxy works", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisClient, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(t, {
    mockDrandAddr: bot.address,
  });
  bot.setGatewayAddress(noisGatewayAddress);

  t.log(`Getting randomness prices ...`);
  const { prices } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { prices: {} });
  t.log(`All available randomness prices: ${prices.map((p: Coin) => p.amount + p.denom).join(",")}`);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  t.log("Executing get_next_randomness for a round that already exists");
  {
    await bot.submitNext();
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_next_randomness: { job_id: "eins" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const commPool1 = await communityPoolFunds(noisClient.sign);
    const info1 = await link.relayAll();
    assertPacketsFromA(info1, 1, true);
    const ack1 = JSON.parse(fromUtf8(info1.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(ack1.result), {
      processed: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:800" },
    });
    const commPool2 = await communityPoolFunds(noisClient.sign);
    const commPoolIncrease = commPool2.minus(commPool1);
    t.deepEqual(commPoolIncrease, Decimal.fromUserInput("45", 18)); // 45% of the gateway `price`

    t.log("Relaying DeliverBeacon");
    const info2 = await link.relayAll();
    assertPacketsFromB(info2, 1, true);
    const ack2 = JSON.parse(fromUtf8(info2.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(ack2.result), {});
  }

  t.log("Executing get_next_randomness for a round that does not yet exists");
  {
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_next_randomness: { job_id: "zwei" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAck.result), {
      queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
    });
  }

  t.log("Executing get_randomness_after for a round that does not yet exists");
  {
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      // Wednesday, 5. April 2023 06:07:08
      // 1680674828
      { get_randomness_after: { after: "1680674828000000000", job_id: "drei" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAck.result), {
      // Expected round: (1680674828-1677685200) / 3 = 996542.6666666666
      queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:996550" },
    });
  }
});

test.serial("proxy works for get_randomness_after", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(t, {
    testMode: false,
    mockDrandAddr: bot.address,
  });
  bot.setGatewayAddress(noisGatewayAddress);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  t.log("Executing get_randomness_after time between 3nd and 4rd round");
  {
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_randomness_after: { after: "1677687666000000000", job_id: "first job" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    assertPacketsFromB(info, 0, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAck.result), {
      queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:830" },
    });
  }

  t.log("Executing get_randomness_after time between 1nd and 2rd round");
  {
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_randomness_after: { after: "1677687603000000000", job_id: "second job" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    assertPacketsFromB(info, 0, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAck.result), {
      queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
    });
  }

  {
    t.log("Submit 1st round and check for no DeliverBeacon");
    await bot.submitNext();
    const info = await link.relayAll();
    assertPacketsFromB(info, 0, true);
  }

  {
    t.log("Submit 2nd round and check DeliverBeacon");
    await bot.submitNext();
    const info = await link.relayAll();
    assertPacketsFromB(info, 1, true);
    const ack = JSON.parse(fromUtf8(info.acksFromA[0].acknowledgement));
    t.deepEqual(ack, { result: toBinary({}) });
  }

  {
    t.log("Submit 3nd round and check for no DeliverBeacon");
    await bot.submitNext();
    const info = await link.relayAll();
    assertPacketsFromB(info, 0, true);
  }

  {
    t.log("Submit 4th round and check DeliverBeacon");
    await bot.submitNext();
    const info = await link.relayAll();
    assertPacketsFromB(info, 1, true);
    const ack = JSON.parse(fromUtf8(info.acksFromA[0].acknowledgement));
    t.deepEqual(ack, { result: toBinary({}) });
  }
});

test.serial("demo contract can be used", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisDemoAddress, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(
    t,
    { mockDrandAddr: bot.address }
  );
  bot.setGatewayAddress(noisGatewayAddress);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  // Correct round submitted before request
  {
    await bot.submitNext();

    const jobId = Date.now().toString();
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { job_id: jobId } },
      "auto",
      undefined,
      [payment]
    );

    // RequestBeacon packet
    const infoA2B = await link.relayAll();
    assertPacketsFromA(infoA2B, 1, true);
    const stdAck = JSON.parse(fromUtf8(infoA2B.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAck.result), {
      processed: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:800" },
    });

    // DeliverBeacon packet
    const infoB2A = await link.relayAll();
    assertPacketsFromB(infoB2A, 1, true);
    const stdAckDeliver = JSON.parse(fromUtf8(infoB2A.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(stdAckDeliver.result), {});

    const myResult = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      result: { job_id: jobId },
    });
    t.log(myResult);
    t.regex(myResult, /3\.1[0-9]+/);

    const results = await wasmClient.sign.queryContractSmart(noisDemoAddress, { results: {} });
    t.log(results);
  }

  // Round submitted after request
  {
    const jobId = Date.now().toString();
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { job_id: jobId } },
      "auto",
      undefined,
      [payment]
    );

    // RequestBeacon packet
    const infoA2B = await link.relayAll();
    assertPacketsFromA(infoA2B, 1, true);
    const stdAck = JSON.parse(fromUtf8(infoA2B.acksFromB[0].acknowledgement));
    t.deepEqual(stdAck, {
      result: toBinary({
        queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
      }),
    });

    // DeliverBeacon packet not yet
    const infoB2A = await link.relayAll();
    assertPacketsFromB(infoB2A, 0, true);

    const myResult = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      result: { job_id: jobId },
    });
    t.is(myResult, null);

    const results = await wasmClient.sign.queryContractSmart(noisDemoAddress, { results: {} });
    t.log(results);

    // Round incoming
    await bot.submitNext();

    // DeliverBeacon packet
    const infoB2A2 = await link.relayAll();
    assertPacketsFromB(infoB2A2, 1, true);
    const stdAckDeliver = JSON.parse(fromUtf8(infoB2A2.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(stdAckDeliver.result), {});

    const myResult2 = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      result: { job_id: jobId },
    });
    t.log(myResult2);
    t.regex(myResult2, /3\.1[0-9]+/);

    const results2 = await wasmClient.sign.queryContractSmart(noisDemoAddress, { results: {} });
    t.log(results2);
  }
});

test.serial("demo contract runs into out of gas in callback", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisDemoAddress, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(
    t,
    {
      mockDrandAddr: bot.address,
      callback_gas_limit: 1_000, // Very low value
    }
  );
  bot.setGatewayAddress(noisGatewayAddress);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  // Correct round submitted before request
  {
    await bot.submitNext();

    const jobId = Date.now().toString();
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { job_id: jobId } },
      "auto",
      undefined,
      [payment]
    );

    // RequestBeacon packet
    const infoA2B = await link.relayAll();
    assertPacketsFromA(infoA2B, 1, true);
    const stdAckRequest = JSON.parse(fromUtf8(infoA2B.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAckRequest.result), {
      processed: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:800" },
    });

    // DeliverBeacon packet (check ack and transaction of the ack)
    const infoB2A = await link.relayAll();
    assertPacketsFromB(infoB2A, 1, true);
    const stdAckDeliver = JSON.parse(fromUtf8(infoB2A.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(stdAckDeliver.result), {});

    const callbackEvent = infoB2A.acksFromA[0].txEvents.find((e) => e.type.startsWith("wasm-nois-callback"));
    t.deepEqual(callbackEvent?.attributes, [
      {
        key: "_contract_address",
        value: noisProxyAddress,
      },
      {
        key: "success",
        value: "false",
      },
      {
        key: "log",
        value: "codespace: sdk, code: 11",
      },
    ]);
  }

  // Round submitted after request
  {
    const jobId = Date.now().toString();
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { job_id: jobId } },
      "auto",
      undefined,
      [payment]
    );

    // RequestBeacon packet
    const infoA2B = await link.relayAll();
    assertPacketsFromA(infoA2B, 1, true);
    const stdAck = JSON.parse(fromUtf8(infoA2B.acksFromB[0].acknowledgement));
    t.deepEqual(stdAck, {
      result: toBinary({
        queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
      }),
    });

    // DeliverBeacon packet not yet
    const infoB2A = await link.relayAll();
    assertPacketsFromB(infoB2A, 0, true);

    // Round incoming
    await bot.submitNext();

    // DeliverBeacon packet (check ack and transaction of the ack)
    const infoB2A2 = await link.relayAll();
    assertPacketsFromB(infoB2A2, 1, true);
    const stdAckDeliver = JSON.parse(fromUtf8(infoB2A2.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(stdAckDeliver.result), {});

    const callbackEvent = infoB2A2.acksFromA[0].txEvents.find((e) => e.type.startsWith("wasm-nois-callback"));
    t.deepEqual(callbackEvent?.attributes, [
      {
        key: "_contract_address",
        value: noisProxyAddress,
      },
      {
        key: "success",
        value: "false",
      },
      {
        key: "log",
        value: "codespace: sdk, code: 11",
      },
    ]);
  }
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
