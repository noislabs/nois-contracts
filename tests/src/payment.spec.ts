import { coin } from "@cosmjs/amino";
import { fromBinary } from "@cosmjs/cosmwasm-stargate";
import { fromUtf8 } from "@cosmjs/encoding";
import test from "ava";
import { Coin } from "cosmjs-types/cosmos/base/v1beta1/coin";

import { MockBot } from "./bot";
import { noisContracts, uploadContracts, wasmContracts } from "./contracts";
import { instantiateAndConnectIbc, TestContext } from "./setup";
import { assertPacketsFromA, assertPacketsFromB, communityPoolFunds, setupNoisClient, setupWasmClient } from "./utils";

test.before(async (t) => {
  const [wasmClient, noisClient] = await Promise.all([setupWasmClient(), setupNoisClient()]);
  t.log("Upload contracts ...");
  const [wasmCodeIds, noisCodeIds] = await Promise.all([
    uploadContracts(t, wasmClient, wasmContracts, ["demo"]),
    uploadContracts(t, noisClient, noisContracts),
  ]);
  const context: TestContext = {
    wasmCodeIds,
    noisCodeIds,
  };
  t.context = context;
  t.pass();
});

test.serial("payment works", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisClient, noisProxyAddress, link, noisGatewayAddress } = await instantiateAndConnectIbc(t, {
    mockDrandAddr: bot.address,
    enablePayment: true,
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
    const commPoolIncrease = commPool2 - commPool1;
    t.deepEqual(commPoolIncrease, 45); // 45% of the gateway `price`

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
    const commPool1 = await communityPoolFunds(noisClient.sign);
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAck.result), {
      queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
    });
    const commPool2 = await communityPoolFunds(noisClient.sign);
    const commPoolIncrease = commPool2 - commPool1;
    t.deepEqual(commPoolIncrease, 45); // 45% of the gateway `price`
  }
});
