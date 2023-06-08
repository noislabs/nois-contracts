import test from "ava";

import { Bot } from "./bot";
import { DrandInstantiateMsg, NoisContractPaths, noisContracts, uploadContracts } from "./contracts";
import { setupNoisClient } from "./utils";

interface TestContext {
  noisCodeIds: Record<keyof NoisContractPaths, number>;
}

test.before(async (t) => {
  const noisClient = await setupNoisClient();
  t.log("Upload contracts ...");
  const noisCodeIds = await uploadContracts(t, noisClient, noisContracts, ["drand"]);
  const context: TestContext = { noisCodeIds };
  t.context = context;
  t.pass();
});

test.serial("drand: bot can submit", async (t) => {
  const context = t.context as TestContext;
  // Instantiate Drand on Nois
  const noisClient = await setupNoisClient();

  const msg: DrandInstantiateMsg = {
    manager: noisClient.senderAddress,
    min_round: 800,
    incentive_point_price: "0",
    incentive_denom: "unois",
  };
  const { contractAddress: drandAddress } = await noisClient.sign.instantiate(
    noisClient.senderAddress,
    context.noisCodeIds.drand,
    msg,
    "Drand instance",
    "auto"
  );
  t.log(`Instantiated drand at ${drandAddress} with msg ${JSON.stringify(msg)}`);
  t.truthy(drandAddress);

  const before = await noisClient.sign.queryContractSmart(drandAddress, {
    beacon: { round: 890 },
  });
  t.deepEqual(before, { beacon: null });

  const bot = await Bot.connect(drandAddress);

  // Register
  await bot.register("joe");

  // Submit
  const addRundRes = await bot.submitRound(890);
  t.log(`Gas used: ${addRundRes.gasUsed}/${addRundRes.gasWanted}`);

  const after = await noisClient.sign.queryContractSmart(drandAddress, {
    beacon: { round: 890 },
  });
  t.regex(after.beacon.published, /^1677687867000000000$/);
  t.regex(after.beacon.verified, /^1[0-9]{18}$/);
  t.is(after.beacon.randomness, "a96bc028106ba9c03d2af47130f907282af36a900f4fc60a20c5a99cf12d639d");
});
