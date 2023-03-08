import test from "ava";

import { IcecubeInstantiateMsg, NoisContractPaths, noisContracts, uploadContracts } from "./contracts";
import { setupNoisClient } from "./utils";

interface TestContext {
  noisCodeIds: Record<keyof NoisContractPaths, number>;
}

test.before(async (t) => {
  const noisClient = await setupNoisClient();
  t.log("Upload contracts ...");
  const noisCodeIds = await uploadContracts(t, noisClient, noisContracts);
  const context: TestContext = { noisCodeIds };
  t.context = context;
  t.pass();
});

test.serial("icecube works", async (t) => {
  const noisClient = await setupNoisClient();

  const msg: IcecubeInstantiateMsg = {
    manager: noisClient.senderAddress,
  };
  await noisClient.sign.instantiate(
    noisClient.senderAddress,
    (t.context as TestContext).noisCodeIds.icecube,
    msg,
    "Icecube instance",
    "auto"
  );

  // TODO: do something cool with the icecube contract

  t.pass();
});
