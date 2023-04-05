import { coin } from "@cosmjs/amino";
import { fromBinary } from "@cosmjs/cosmwasm-stargate";
import { fromUtf8 } from "@cosmjs/encoding";
import { assert } from "@cosmjs/utils";
import test from "ava";
import { Coin } from "cosmjs-types/cosmos/base/v1beta1/coin";

import { MockBot } from "./bot";
import { GatewayCustomerResponse, noisContracts, uploadContracts, wasmContracts } from "./contracts";
import { instantiateAndConnectIbc, TestContext } from "./setup";
import {
  assertPacketsFromA,
  assertPacketsFromB,
  communityPoolFunds,
  setupNoisClient,
  setupWasmClient,
  totalSupply,
} from "./utils";

test.before(async (t) => {
  const [wasmClient, noisClient] = await Promise.all([setupWasmClient(), setupNoisClient()]);
  t.log("Upload contracts ...");
  const [wasmCodeIds, noisCodeIds] = await Promise.all([
    uploadContracts(t, wasmClient, wasmContracts, ["demo"]),
    uploadContracts(t, noisClient, noisContracts, ["icecube"]),
  ]);
  const context: TestContext = {
    wasmCodeIds,
    noisCodeIds,
  };
  t.context = context;
  t.pass();
});

function printCoin(c: Coin): string {
  return `${c.amount}${c.denom}`;
}

test.serial("payment works", async (t) => {
  const bot = await MockBot.connect();
  const { wasmClient, noisClient, noisProxyAddress, link, noisGatewayAddress, sinkAddress, noisChannel, realyerNois } =
    await instantiateAndConnectIbc(t, {
      mockDrandAddr: bot.address,
      enablePayment: true,
    });
  assert(sinkAddress);
  bot.setGatewayAddress(noisGatewayAddress);

  const gatewayPrice = 50_000000; // the gateway `price`
  const burnAmount = 0.5 * gatewayPrice; // 50% of the gateway `price`
  const poolAmount = 0.45 * gatewayPrice; // 45% of the gateway `price`
  const relayerAmount = 0.05 * gatewayPrice; // 5% of the gateway `price`

  const { customer }: GatewayCustomerResponse = await noisClient.sign.queryContractSmart(noisGatewayAddress, {
    customer: { channel_id: noisChannel.noisChannelId },
  });
  assert(customer, "customer not set");
  t.is(customer.requested_beacons, 0);
  const paymentAddress = customer.payment;
  assert(typeof paymentAddress === "string");

  const paymentBalanceInitial = await noisClient.sign.getBalance(paymentAddress, "unois");
  t.log(`Initial balance of payment contract ${paymentAddress}: ${printCoin(paymentBalanceInitial)}`);

  t.log(`Getting randomness prices ...`);
  const { prices } = await wasmClient.sign.queryContractSmart(noisProxyAddress, { prices: {} });
  t.log(`All available randomness prices: ${prices.map(printCoin).join(",")}`);

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
    const paymentBalance1 = parseInt((await noisClient.sign.getBalance(paymentAddress, "unois")).amount, 10);
    const commPool1 = await communityPoolFunds(noisClient.sign);
    const total1 = parseInt((await totalSupply(noisClient.sign)).amount, 10);
    const relayer1 = parseInt((await noisClient.sign.getBalance(realyerNois, "unois")).amount, 10);
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    const ack1 = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(ack1.result), {
      processed: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:800" },
    });
    const total2 = parseInt((await totalSupply(noisClient.sign)).amount, 10);
    const reduction = total1 - total2;
    t.assert(reduction >= 0.99 * burnAmount && reduction <= burnAmount);
    const relayer2 = parseInt((await noisClient.sign.getBalance(realyerNois, "unois")).amount, 10);
    const relayeIncrease = relayer2 - relayer1;
    t.assert(relayeIncrease >= 0.98 * relayerAmount && relayeIncrease <= relayerAmount);
    const paymentBalance2 = parseInt((await noisClient.sign.getBalance(paymentAddress, "unois")).amount, 10);
    const paymentBalanceDecrease = paymentBalance1 - paymentBalance2;
    t.deepEqual(paymentBalanceDecrease, gatewayPrice);
    const commPool2 = await communityPoolFunds(noisClient.sign);
    const commPoolIncrease = commPool2 - commPool1;
    t.deepEqual(commPoolIncrease, poolAmount);
    const { ashes } = await noisClient.sign.queryContractSmart(sinkAddress, { ashes_desc: {} });
    t.deepEqual(ashes.length, 1);
    t.deepEqual(ashes[0].burner, paymentAddress);
    t.deepEqual(ashes[0].amount, coin(burnAmount, "unois"));

    const { customer: customer2 }: GatewayCustomerResponse = await noisClient.sign.queryContractSmart(
      noisGatewayAddress,
      {
        customer: { channel_id: noisChannel.noisChannelId },
      }
    );
    assert(customer2, "customer not set");
    t.is(customer2.requested_beacons, 1);

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
    const paymentBalance1 = parseInt((await noisClient.sign.getBalance(paymentAddress, "unois")).amount, 10);
    const commPool1 = await communityPoolFunds(noisClient.sign);
    const total1 = parseInt((await totalSupply(noisClient.sign)).amount, 10);
    const info = await link.relayAll();
    assertPacketsFromA(info, 1, true);
    const stdAck = JSON.parse(fromUtf8(info.acksFromB[0].acknowledgement));
    t.deepEqual(fromBinary(stdAck.result), {
      queued: { source_id: "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810" },
    });
    const total2 = parseInt((await totalSupply(noisClient.sign)).amount, 10);
    const reduction = total1 - total2;
    t.assert(reduction >= 0.99 * burnAmount && reduction <= burnAmount);
    const paymentBalance2 = parseInt((await noisClient.sign.getBalance(paymentAddress, "unois")).amount, 10);
    const paymentBalanceDecrease = paymentBalance1 - paymentBalance2;
    t.deepEqual(paymentBalanceDecrease, gatewayPrice);
    const commPool2 = await communityPoolFunds(noisClient.sign);
    const commPoolIncrease = commPool2 - commPool1;
    t.deepEqual(commPoolIncrease, poolAmount);
    const { ashes } = await noisClient.sign.queryContractSmart(sinkAddress, { ashes_desc: {} });
    t.deepEqual(ashes.length, 2);
    t.deepEqual(ashes[0].burner, paymentAddress);
    t.deepEqual(ashes[0].amount, coin(burnAmount, "unois"));

    const { customer: customer3 }: GatewayCustomerResponse = await noisClient.sign.queryContractSmart(
      noisGatewayAddress,
      {
        customer: { channel_id: noisChannel.noisChannelId },
      }
    );
    assert(customer3, "customer not set");
    t.is(customer3.requested_beacons, 2);
  }
});
