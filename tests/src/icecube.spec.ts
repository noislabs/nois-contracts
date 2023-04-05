import { testutils } from "@confio/relayer";
import test from "ava";

import {
  IcecubeExecuteMsg,
  IcecubeInstantiateMsg,
  NoisContractPaths,
  noisContracts,
  uploadContracts,
} from "./contracts";
import { nois, noisValidator, setupNoisClient } from "./utils";
const { fundAccount } = testutils;

interface TestContext {
  noisCodeIds: Record<keyof NoisContractPaths, number>;
}

test.before(async (t) => {
  const noisClient = await setupNoisClient();
  t.log("Upload contracts ...");
  const noisCodeIds = await uploadContracts(t, noisClient, noisContracts, ["drand", "sink"]);
  const context: TestContext = { noisCodeIds };
  t.context = context;
  t.pass();
});

test.serial("icecube works", async (t) => {
  const noisClient = await setupNoisClient();

  const msg: IcecubeInstantiateMsg = {
    manager: noisClient.senderAddress,
  };
  const { contractAddress } = await noisClient.sign.instantiate(
    noisClient.senderAddress,
    (t.context as TestContext).noisCodeIds.icecube,
    msg,
    "Icecube instance",
    "auto"
  );
  await fundAccount(nois, contractAddress, "4000000");

  const delegateMsg: IcecubeExecuteMsg = {
    delegate: {
      addr: noisValidator.address,
      amount: "1000",
    },
  };
  const { events: delegateEvents } = await noisClient.sign.execute(
    noisClient.senderAddress,
    contractAddress,
    delegateMsg,
    "auto"
  );
  // t.log(JSON.stringify(delegateEvents, null, 2));
  const delegate = delegateEvents.find((event) => event.type == "delegate");
  t.truthy(delegate);

  const undelegateMsg: IcecubeExecuteMsg = {
    undelegate: {
      addr: noisValidator.address,
      amount: "200",
    },
  };
  const { events: undelegateEvents } = await noisClient.sign.execute(
    noisClient.senderAddress,
    contractAddress,
    undelegateMsg,
    "auto"
  );
  // t.log(JSON.stringify(undelegateEvents, null, 2));
  const withdrawRewards = undelegateEvents.find((event) => event.type == "withdraw_rewards");
  t.truthy(withdrawRewards);
  const unbond = undelegateEvents.find((event) => event.type == "unbond");
  t.truthy(unbond);
});
