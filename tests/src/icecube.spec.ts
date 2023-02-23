import test from "ava";

import { IcecubeInstantiateMsg, OsmosisContractPaths, osmosisContracts, uploadContracts } from "./contracts";
import { setupOsmosisClient } from "./utils";

interface TestContext {
  osmosisCodeIds: Record<keyof OsmosisContractPaths, number>;
}

test.before(async (t) => {
  const osmoClient = await setupOsmosisClient();
  t.log("Upload contracts ...");
  const osmosisCodeIds = await uploadContracts(t, osmoClient, osmosisContracts);
  const context: TestContext = { osmosisCodeIds };
  t.context = context;
  t.pass();
});

test.serial("icecube works", async (t) => {
  const osmoClient = await setupOsmosisClient();

  const msg: IcecubeInstantiateMsg = {
    manager: osmoClient.senderAddress,
  };
  await osmoClient.sign.instantiate(
    osmoClient.senderAddress,
    (t.context as TestContext).osmosisCodeIds.icecube,
    msg,
    "Icecube instance",
    "auto"
  );

  // TODO: do something cool with the icecube contract

  t.pass();
});
