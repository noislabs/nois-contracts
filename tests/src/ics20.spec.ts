import { Link, testutils } from "@confio/relayer";
import { coin } from "@cosmjs/amino";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { assertIsDeliverTxSuccess, GasPrice, SigningStargateClient } from "@cosmjs/stargate";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";
import Long from "long";

import { assertPacketsFromB, ibcDenom, nois, randomAddress, setupWasmClient } from "./utils";

const { setup, wasmd } = testutils;

test.serial("set up ICS20 channel and transfer NOIS", async (t) => {
  // Create a connection between the chains
  const [src, dest] = await setup(wasmd, nois);
  const link = await Link.createWithNewConnections(src, dest);

  // Also create a ics20 channel
  const ics20Info = await link.createChannel("A", wasmd.ics20Port, nois.ics20Port, Order.ORDER_UNORDERED, "ics20-1");
  const ics20Channel = {
    wasmChannelId: ics20Info.src.channelId,
    noisChannelId: ics20Info.dest.channelId,
  };
  const unoisOnWasm = ibcDenom(ics20Channel.wasmChannelId, "unois");

  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(nois.faucet.mnemonic, { prefix: nois.prefix });
  const address = (await wallet.getAccounts())[0].address;
  const noisClient = await SigningStargateClient.connectWithSigner(nois.tendermintUrlHttp, wallet, {
    gasPrice: GasPrice.fromString(nois.minFee),
  });

  const wasmClient = await setupWasmClient();
  const recipient = randomAddress(wasmd.prefix);
  const res = await noisClient.sendIbcTokens(
    address,
    recipient,
    coin(123, "unois"),
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

  const balance = await wasmClient.sign.getBalance(recipient, unoisOnWasm);
  t.deepEqual(balance, {
    amount: "123",
    denom: unoisOnWasm,
  });

  noisClient.disconnect();
});
