import { CosmWasmSigner, Link, testutils } from "@confio/relayer";
import { fromBinary } from "@cosmjs/cosmwasm-stargate";
import { assert, sleep } from "@cosmjs/utils";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";

import { Bot } from "./bot";
import {
  assertPacketsFromA,
  loeMainnetPubkey,
  NoisProtocolIbcVersion,
  setupContracts,
  setupOsmosisClient,
  setupWasmClient,
} from "./utils";

const { osmosis: oldOsmo, setup, wasmd, randomAddress } = testutils;
const osmosis = { ...oldOsmo, minFee: "0.025uosmo" };

let wasmCodeIds: Record<string, number> = {};
let osmosisCodeIds: Record<string, number> = {};

test.before(async (t) => {
  console.debug("Upload contracts to wasmd...");
  const wasmContracts = {
    proxy: "./internal/nois_proxy.wasm",
    demo: "./internal/nois_demo.wasm",
  };
  const wasmSign = await setupWasmClient();
  wasmCodeIds = await setupContracts(wasmSign, wasmContracts);

  console.debug("Upload contracts to osmosis...");
  const osmosisContracts = {
    terrand: "./internal/nois_terrand.wasm",
  };
  const osmosisSign = await setupOsmosisClient();
  osmosisCodeIds = await setupContracts(osmosisSign, osmosisContracts);

  t.pass();
});

test.serial("Bot can submit to Terrand", async (t) => {
  // Instantiate Terrand on osmosis
  const osmoClient = await setupOsmosisClient();
  const { contractAddress: terrandAddress } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisCodeIds.terrand,
    { pubkey: loeMainnetPubkey },
    "Terrand instance",
    "auto"
  );
  t.truthy(terrandAddress);

  const bot = await Bot.connect(terrandAddress);
  await bot.submitRound(2183666);
});

test.serial("set up channel", async (t) => {
  // Instantiate proxy on appchain
  const wasmClient = await setupWasmClient();
  const { contractAddress: proxyAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmCodeIds.proxy,
    {},
    "Proxy instance",
    "auto"
  );
  t.truthy(proxyAddress);
  const { ibcPortId: proxyPort } = await wasmClient.sign.getContract(proxyAddress);
  t.log(`Proxy port: ${proxyPort}`);
  assert(proxyPort);

  // Instantiate Terrand on osmosis
  const osmoClient = await setupOsmosisClient();
  const { contractAddress: terrandAddress } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisCodeIds.terrand,
    { pubkey: loeMainnetPubkey },
    "Terrand instance",
    "auto"
  );
  t.truthy(terrandAddress);
  const { ibcPortId: terrandPort } = await osmoClient.sign.getContract(terrandAddress);
  t.log(`Terrand port: ${terrandPort}`);
  assert(terrandPort);

  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);
  await link.createChannel("A", proxyPort, terrandPort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
});

interface SetupInfo {
  wasmClient: CosmWasmSigner;
  osmoClient: CosmWasmSigner;
  /// Address on app chain (wasmd)
  noisProxyAddress: string;
  /// Address on app chain (wasmd)
  noisDemoAddress: string;
  /// Address on randomness chain (osmosis)
  noisTerrandAddress: string;
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

async function demoSetup(): Promise<SetupInfo> {
  // Instantiate proxy on appchain
  const wasmClient = await setupWasmClient();
  const { contractAddress: noisProxyAddress } = await wasmClient.sign.instantiate(
    wasmClient.senderAddress,
    wasmCodeIds.proxy,
    {},
    "Proxy instance",
    "auto"
  );
  const { ibcPortId: proxyPort } = await wasmClient.sign.getContract(noisProxyAddress);
  assert(proxyPort);

  // Instantiate Terrand on Osmosis
  const osmoClient = await setupOsmosisClient();
  const { contractAddress: noisTerrandAddress } = await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    osmosisCodeIds.terrand,
    { pubkey: loeMainnetPubkey },
    "Terrand instance",
    "auto"
  );
  const { ibcPortId: terrandPort } = await osmoClient.sign.getContract(noisTerrandAddress);
  assert(terrandPort);

  // Create a connection between the chains
  const [src, dest] = await setup(wasmd, osmosis);
  const link = await Link.createWithNewConnections(src, dest);

  // Create a channel for nois-v1
  const info = await link.createChannel("A", proxyPort, terrandPort, Order.ORDER_UNORDERED, NoisProtocolIbcVersion);
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
    noisTerrandAddress,
    link,
    noisChannel,
    ics20Channel,
  };
}

test.serial("proxy works", async (t) => {
  const { wasmClient, noisProxyAddress, link, osmoClient, noisTerrandAddress } = await demoSetup();
  const bot = await Bot.connect(noisTerrandAddress);

  // make a new empty account on osmosis
  const emptyAddr = randomAddress(osmosis.prefix);
  const noFunds = await osmoClient.sign.getBalance(emptyAddr, osmosis.denomFee);
  t.is(noFunds.amount, "0");

  // Query round 1 (existing)
  {
    await bot.submitRound(2183668);
    const getRoundQuery = await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_round: { round: "2183668" } },
      "auto"
    );
    console.log(getRoundQuery);

    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);

    await sleep(1000);

    const latestResult = await wasmClient.sign.queryContractSmart(noisProxyAddress, {
      latest_get_round_result: {},
    });
    // console.log(latestResult);
    // console.log(latestResult.response.acknowledgement.data);
    const result: string = fromBinary(latestResult.response.acknowledgement.data).result;
    const response = fromBinary(result);
    t.deepEqual(response, {
      beacon: { randomness: "3436462283a07e695c41854bb953e5964d8737e7e29745afe54a9f4897b6c319" },
    });
    console.log(response);
  }

  // Query round 3 (non-existing)
  {
    const getRoundQuery = await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisProxyAddress,
      { get_round: { round: "2999999" } },
      "auto"
    );
    console.log(getRoundQuery);

    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);

    await sleep(1000);

    const latestResult = await wasmClient.sign.queryContractSmart(noisProxyAddress, {
      latest_get_round_result: {},
    });
    // console.log(latestResult);
    // console.log(latestResult.response.acknowledgement.data);
    const result: string = fromBinary(latestResult.response.acknowledgement.data).result;
    const response = fromBinary(result);
    console.log(response);
    t.deepEqual(response, { beacon: null });
  }
});

test.serial("demo contract can be used", async (t) => {
  const { wasmClient, noisDemoAddress, link, noisTerrandAddress } = await demoSetup();
  const bot = await Bot.connect(noisTerrandAddress);

  // Query round 2183667 (existing)
  {
    await bot.submitRound(2183667);

    const getRoundQuery = await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { round: "2183667", job_id: Date.now().toString() } },
      "auto"
    );
    console.log(getRoundQuery);

    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);

    await sleep(1000);

    const latestResult = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      latest_result: {},
    });
    console.log(latestResult);
    t.regex(latestResult, /3\.1[0-9]+/);

    const results = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      results: {},
    });
    console.log(results);
  }

  // a few more values
  await bot.submitRound(2183668);
  await bot.submitRound(2183669);
  await bot.submitRound(2183670);

  for (const round of ["2183668", "2183669", "2183670"]) {
    const getRoundQuery = await wasmClient.sign.execute(
      wasmClient.senderAddress,
      noisDemoAddress,
      { estimate_pi: { round, job_id: Date.now().toString() } },
      "auto"
    );
    console.log(getRoundQuery);

    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);

    await sleep(1000);

    const latestResult = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      latest_result: {},
    });
    console.log(latestResult);
    t.regex(latestResult, /3\.1[0-9]+/);

    const results = await wasmClient.sign.queryContractSmart(noisDemoAddress, {
      results: {},
    });
    console.log(results);
  }
});
