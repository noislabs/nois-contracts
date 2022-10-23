import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
import { coin, coins } from "@cosmjs/amino";
import { ExecuteInstruction, toBinary } from "@cosmjs/cosmwasm-stargate";
import { fromUtf8 } from "@cosmjs/encoding";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Coin } from "cosmjs-types/cosmos/base/v1beta1/coin";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

import { Bot } from "./bot";
import {
  assertPacketsFromA,
  assertPacketsFromB,
  NoisProtocolIbcVersion,
  setupContracts,
  setupOsmosisClient,
  setupWasmClient,
} from "./utils";

const { osmosis: oldOsmo, setup, wasmd, fundAccount } = testutils;
const osmosis = { ...oldOsmo, minFee: "0.025uosmo" };

let wasmCodeIds: Record<string, number> = {};
let osmosisCodeIds: Record<string, number> = {};

interface OracleInstantiateMsg {
  readonly min_round: number;
  readonly incentive_amount: string;
  readonly incentive_denom: string;
}

interface ProxyInstantiateMsg {
  readonly prices: Array<Coin>;
  readonly withdrawal_address: string;
  readonly test_mode: boolean;
}

test.before(async (t) => {
  t.log("Upload contracts to wasmd...");
  const wasmContracts = {
    proxy: "./internal/nois_proxy.wasm",
    demo: "./internal/nois_demo.wasm",
  };
  const wasmSign = await setupWasmClient();
  wasmCodeIds = await setupContracts(t, wasmSign, wasmContracts);

  t.log("Upload contracts to osmosis...");
  const osmosisContracts = {
    oracle: "./internal/nois_oracle.wasm",
  };
  const osmosisSign = await setupOsmosisClient();
  osmosisCodeIds = await setupContracts(t, osmosisSign, osmosisContracts);

  t.pass();
});

test.serial("Bot can submit to Oracle", async (t) => {
  // Instantiate Oracle on osmosis
  const osmoClient = await setupOsmosisClient();
  const msg: OracleInstantiateMsg = {
    min_round: 2183660,
    incentive_amount: "0",
    incentive_denom: "unois",
  };
  const { contractAddress: oracleAddress } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisCodeIds.oracle,
    msg,
    "Oracle instance",
    "auto"
  );
  t.log(`Instantiated oracle at ${oracleAddress} with msg ${JSON.stringify(msg)}`);
  t.truthy(oracleAddress);

  const before = await osmoClient.sign.queryContractSmart(oracleAddress, {
    beacon: { round: 2183666 },
  });
  t.deepEqual(before, { beacon: null });

  const bot = await Bot.connect(oracleAddress);
  const res = await bot.submitRound(2183666);
  t.log(`Gas used: ${res.gasUsed}/${res.gasWanted}`);

  const after = await osmoClient.sign.queryContractSmart(oracleAddress, {
    beacon: { round: 2183666 },
  });
  t.regex(after.beacon.published, /^1660941000000000000$/);
  t.regex(after.beacon.verified, /^1[0-9]{18}$/);
  t.is(after.beacon.randomness, "768bd188a948f1f2959d15c657f159dd34bdf741b7d4b17a29b877eb36c04dcf");
});

test.serial("set up channel", async (t) => {
  // Instantiate proxy on appchain
  const wasmClient = await setupWasmClient();
  const proxyMsg: ProxyInstantiateMsg = {
    prices: coins(1_000_000, "ucosm"),
    withdrawal_address: wasmClient.senderAddress,
    test_mode: true,
  };
  const { contractAddress: proxyAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmCodeIds.proxy,
    proxyMsg,
    "Proxy instance",
    "auto"
  );
  t.truthy(proxyAddress);
  const { ibcPortId: proxyPort } = await wasmClient.sign.getContract(proxyAddress);
  t.log(`Proxy port: ${proxyPort}`);
  assert(proxyPort);

  // Instantiate Oracle on osmosis
  const osmoClient = await setupOsmosisClient();
  const msg: OracleInstantiateMsg = {
    min_round: 2183660,
    incentive_amount: "0",
    incentive_denom: "unois",
  };
  const { contractAddress: oracleAddress } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisCodeIds.oracle,
    msg,
    "Oracle instance",
    "auto"
  );
  t.truthy(oracleAddress);
  const { ibcPortId: oraclePort } = await osmoClient.sign.getContract(oracleAddress);
  t.log(`Oracle port: ${oraclePort}`);
  assert(oraclePort);

  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", proxyPort, oraclePort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
});

interface SetupInfo {
  wasmClient: CosmWasmSigner;
  osmoClient: CosmWasmSigner;
  /// Address on app chain (wasmd)
  noisProxyAddress: string;
  /// Address on app chain (wasmd)
  noisDemoAddress: string;
  /// Address on randomness chain (osmosis)
  noisOracleAddress: string;
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

async function instantiateAndConnectIbc(testMode: boolean): Promise<SetupInfo> {
  const [wasmClient, osmoClient] = await Promise.all([setupWasmClient(), setupOsmosisClient()]);

  // Instantiate proxy on appchain
  const proxyMsg: ProxyInstantiateMsg = {
    prices: coins(1_000_000, "ucosm"),
    withdrawal_address: wasmClient.senderAddress,
    test_mode: testMode,
  };
  const { contractAddress: noisProxyAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmCodeIds.proxy,
    proxyMsg,
    "Proxy instance",
    "auto"
  );

  // Instantiate Oracle on Osmosis
  const msg: OracleInstantiateMsg = {
    min_round: 2183660,
    incentive_amount: "0",
    incentive_denom: "unois",
  };
  const { contractAddress: noisOracleAddress } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisCodeIds.oracle,
    msg,
    "Oracle instance",
    "auto"
  );

  const [noisProxyInfo, noisOracleInfo] = await Promise.all([
    wasmClient.sign.getContract(noisProxyAddress),
    osmoClient.sign.getContract(noisOracleAddress),
  ]);
  const { ibcPortId: proxyPort } = noisProxyInfo;
  assert(proxyPort);
  const { ibcPortId: oraclePort } = noisOracleInfo;
  assert(oraclePort);

  // Create a connection between the chains
  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);

  // Create a channel for nois-v3
  const info = await link.createChannel("A", proxyPort, oraclePort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
  const noisChannel = {
    wasmChannelId: info.src.channelId,
    osmoChannelId: info.dest.channelId,
  };

  // Also create a ics20 channel
  const ics20Info = await link.createChannel("A", wasmd.ics20Port, osmosis.ics20Port, Order.ORDER_UNORDERED, "ics20-1");
  const ics20Channel = {
    wasmChannelId: ics20Info.src.channelId,
    osmoChannelId: ics20Info.dest.channelId,
  };

  // Instantiate demo app
  const { contractAddress: noisDemoAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmCodeIds.demo,
    { nois_proxy: noisProxyAddress },
    "A demo contract",
    "auto"
  );

  return {
    wasmClient,
    osmoClient,
    noisProxyAddress,
    noisDemoAddress,
    noisOracleAddress,
    link,
    noisChannel,
    ics20Channel,
  };
}

test.serial("proxy works", async (t) => {
  const { wasmClient, noisProxyAddress, link, noisOracleAddress } = await instantiateAndConnectIbc(true);
  const bot = await Bot.connect(noisOracleAddress);

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
    const info1 = await link.relayAll();
    assertPacketsFromA(info1, 1, true);
    const ack1 = JSON.parse(fromUtf8(info1.acksFromB[0].acknowledgement));
    t.deepEqual(ack1, {
      result: toBinary({
        processed: { source_id: "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2183660" },
      }),
    });

    t.log("Relaying DeliverBeacon");
    const info2 = await link.relayAll();
    assertPacketsFromB(info2, 1, true);
    const ack2 = JSON.parse(fromUtf8(info2.acksFromA[0].acknowledgement));
    t.deepEqual(ack2, { result: toBinary({ delivered: { job_id: "eins" } }) });
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
    t.deepEqual(stdAck, {
      result: toBinary({
        queued: { source_id: "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2183661" },
      }),
    });
  }

  t.log("Executing get_randomness_after for a round that does not yet exists");
  {
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_randomness_after: { after: "1663357574000000000", job_id: "drei" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(stdAck, {
      result: toBinary({
        queued: { source_id: "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2264219" },
      }),
    });
  }
});

test.serial("proxy works for get_randomness_after", async (t) => {
  const { wasmClient, noisProxyAddress, link, noisOracleAddress } = await instantiateAndConnectIbc(false);
  const bot = await Bot.connect(noisOracleAddress);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  t.log("Executing get_randomness_after time between 3nd and 4rd round");
  {
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_randomness_after: { after: "1660940884222222222", job_id: "first job" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    assertPacketsFromB(info, 0, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(stdAck, {
      result: toBinary({
        queued: { source_id: "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2183663" },
      }),
    });
  }

  t.log("Executing get_randomness_after time between 1nd and 2rd round");
  {
    await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_randomness_after: { after: "1660940820000000000", job_id: "second job" } },
      "auto",
      undefined,
      [payment]
    );

    t.log("Relaying RequestBeacon");
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    assertPacketsFromB(info, 0, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(stdAck, {
      result: toBinary({
        queued: { source_id: "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2183661" },
      }),
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
    t.deepEqual(ack, { result: toBinary({ delivered: { job_id: "second job" } }) });
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
    t.deepEqual(ack, { result: toBinary({ delivered: { job_id: "first job" } }) });
  }
});

test.serial("demo contract can be used", async (t) => {
  const { wasmClient, noisDemoAddress, noisProxyAddress, link, noisOracleAddress } = await instantiateAndConnectIbc(
    true
  );
  const bot = await Bot.connect(noisOracleAddress);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  // Correct round submitted before request
  {
    await bot.submitNext();

    const jobId = Date.now().toString();
    const getRoundQuery = await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { job_id: jobId } },
      "auto",
      undefined,
      [payment]
    );
    t.log(getRoundQuery);

    // RequestBeacon packet
    const infoA2B = await link.relayAll();
    assertPacketsFromA(infoA2B, 1, true);
    const stdAck = JSON.parse(fromUtf8(infoA2B.acksFromB[0].acknowledgement));
    t.deepEqual(stdAck, {
      result: toBinary({
        processed: { source_id: "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2183660" },
      }),
    });

    // DeliverBeacon packet
    const infoB2A = await link.relayAll();
    assertPacketsFromB(infoB2A, 1, true);

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
    const getRoundQuery = await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { job_id: jobId } },
      "auto",
      undefined,
      [payment]
    );
    t.log(getRoundQuery);

    // RequestBeacon packet
    const infoA2B = await link.relayAll();
    assertPacketsFromA(infoA2B, 1, true);
    const stdAck = JSON.parse(fromUtf8(infoA2B.acksFromB[0].acknowledgement));
    t.deepEqual(stdAck, {
      result: toBinary({
        queued: { source_id: "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2183661" },
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

    const myResult2 = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      result: { job_id: jobId },
    });
    t.log(myResult2);
    t.regex(myResult2, /3\.1[0-9]+/);

    const results2 = await wasmClient.sign.queryContractSmart(noisDemoAddress, { results: {} });
    t.log(results2);
  }
});

test.serial("submit randomness for various job counts", async (t) => {
  const { wasmClient, noisProxyAddress, link, noisOracleAddress } = await instantiateAndConnectIbc(false);
  const bot = await Bot.connect(noisOracleAddress);

  const { price } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { price: { denom: "ucosm" } });
  const payment = coin(price, "ucosm");
  t.log(`Got randomness price from query: ${payment.amount}${payment.denom}`);

  await fundAccount(wasmd, wasmClient.senderAddress, "40000000");

  function before(num: string): string {
    return (BigInt(num) - BigInt(1)).toString();
  }

  const afterValues = [
    before("1660940820000000000"),
    before("1660940850000000000"),
    before("1660940880000000000"),
    before("1660940910000000000"),
    before("1660940940000000000"),
    before("1660940970000000000"),
    before("1660941000000000000"),
    before("1660941030000000000"),
    before("1660941060000000000"),
    before("1660941090000000000"),
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
    t.is(result.logs.length, 1);
    const packetsEvents = result.logs[0].events.filter((e) => e.type === "send_packet");
    const attributes = packetsEvents.flatMap((e) => e.attributes);
    const packetsSentCount = attributes.filter((a) => a.key === "packet_sequence").length;
    t.log("Number of packets sent:", packetsSentCount);
    t.is(packetsSentCount, Math.min(jobs, 3));
  }
});
