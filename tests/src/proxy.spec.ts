import { coin } from "@cosmjs/amino";
import { fromBinary, toBinary } from "@cosmjs/cosmwasm-stargate";
import { fromUtf8 } from "@cosmjs/encoding";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Coin } from "cosmjs-types/cosmos/base/v1beta1/coin";

import { MockBot } from "./bot";
import { noisContracts, uploadContracts, wasmContracts } from "./contracts";
import { instantiateAndConnectIbc, TestContext } from "./setup";
import { assertPacketsFromA, assertPacketsFromB, setupNoisClient, setupWasmClient } from "./utils";

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

test.serial("proxy works", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(t, {
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
    const info1 = await link.relayAll();
    assertPacketsFromA(info1, 1, true);
    const ack1 = JSON.parse(fromUtf8(info1.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(ack1.result), {
      request_processed: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:800" },
    });

    t.log("Relaying DeliverBeacon");
    const info2 = await link.relayAll();
    assertPacketsFromB(info2, 1, true);
    const ack2 = JSON.parse(fromUtf8(info2.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(ack2.result), { deliver_beacon: {} });
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
      request_queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
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
      request_queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:996550" },
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
      request_queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:830" },
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
      request_queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
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
    t.deepEqual(fromBinary(ack.result), { deliver_beacon: {} });
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
    t.deepEqual(fromBinary(ack.result), { deliver_beacon: {} });
  }
});

test.serial("demo contract can be used", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisDemoAddress, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(
    t,
    { mockDrandAddr: bot.address }
  );
  assert(noisDemoAddress);
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
      request_processed: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:800" },
    });

    // DeliverBeacon packet
    const infoB2A = await link.relayAll();
    assertPacketsFromB(infoB2A, 1, true);
    const stdAckDeliver = JSON.parse(fromUtf8(infoB2A.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(stdAckDeliver.result), { deliver_beacon: {} });

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
        request_queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
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
    t.deepEqual(fromBinary(stdAckDeliver.result), { deliver_beacon: {} });

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
  assert(noisDemoAddress);
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
      request_processed: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:800" },
    });

    // DeliverBeacon packet (check ack and transaction of the ack)
    const infoB2A = await link.relayAll();
    assertPacketsFromB(infoB2A, 1, true);
    const stdAckDeliver = JSON.parse(fromUtf8(infoB2A.acksFromA[0].acknowledgement));
    t.deepEqual(fromBinary(stdAckDeliver.result), { deliver_beacon: {} });

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
    t.deepEqual(fromBinary(stdAck.result), {
      request_queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
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
    t.deepEqual(fromBinary(stdAckDeliver.result), { deliver_beacon: {} });

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
