import { MsgExecuteContractEncodeObject, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { fromHex, toBase64, toUtf8 } from "@cosmjs/encoding";
import { assertIsDeliverTxSuccess } from "@cosmjs/stargate";
import { assert } from "@cosmjs/utils";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";

import { setupOsmosisClient } from "./utils";

interface Beacon {
  readonly round: number;
  readonly randomness: string;
  readonly signature: string;
  readonly previous_signature: string;
}

const localDataSource: Map<number, Beacon> = new Map(
  // Generate items with shell:
  //   for r in {2183660..2183670}; do echo "    [$r, $(curl -sS https://api3.drand.sh/public/$r)],"; done

  // prettier-ignore
  [
    [2183660, {"round":2183660,"randomness":"cbc851305a9b82e38863a77e5bc61b8707554adb3920418a6903489b284f88c2","signature":"b7cc14cb609b83ab5a9b95a095d3482a3b101450c7dbf9eff544c69db9d12ccd50751b2a0ff936885d254f3ddb0b143312aef9a9487dd2b7d766e35b5ccf0e34677070d3c612142b2c0d1d47633fd365a1a4b9bf58d8c745fb65d33c0d7323c0","previous_signature":"82af59ce7dfdfab98af6553c0f6a5bad22d2e246eb128740e45378dba3caf17572cf52b3d2c4d2fd68ba85357b1ab8b2052db62300a6007c6d82de0a1231a6ad75acc41f4174a1428873ed83db3bebe8e58e7bc0b13ec1cc4498a5a2a391baf0"}],
    [2183661, {"round":2183661,"randomness":"298403ad854a067cc64c9518a1bf1406425ad109269a49778b42d65c88919b1f","signature":"b6129952af337fed2e0a46fec8eb99167bd7a4d0ef1872ac4903f736f4628ae61b7d3d605a88ace5b03b4c52746c55f6056c8cd34058ef15282fd2ac054e1236b57921e8a0d4e824934cae04807b255d885c416be45f33014835023cb36f94f5","previous_signature":"b7cc14cb609b83ab5a9b95a095d3482a3b101450c7dbf9eff544c69db9d12ccd50751b2a0ff936885d254f3ddb0b143312aef9a9487dd2b7d766e35b5ccf0e34677070d3c612142b2c0d1d47633fd365a1a4b9bf58d8c745fb65d33c0d7323c0"}],
    [2183662, {"round":2183662,"randomness":"5059bd56c8f1a6bb541636c27346b660fea3a2b8fa2565da6f44601da93606da","signature":"a9ddbfc829c7fcbc2149419463017d13978851b4e5fa06b27b07a4bf94217d20f0645715abb847c4e2db30ef270325160c848419d3b227ccb4248c6c6c05d3551742d396b69e46b91e11e77c1b7c5eb6482db5c205f0fff844a03c60c8841c8b","previous_signature":"b6129952af337fed2e0a46fec8eb99167bd7a4d0ef1872ac4903f736f4628ae61b7d3d605a88ace5b03b4c52746c55f6056c8cd34058ef15282fd2ac054e1236b57921e8a0d4e824934cae04807b255d885c416be45f33014835023cb36f94f5"}],
    [2183663, {"round":2183663,"randomness":"519e33609b0f4eb617b58ae7cac13b80f47a3035804e553d1765400d04fc85cb","signature":"a6ceb9cbe5135e749641ce48377ee2a8c93bb1aa754de156a134d3fd83b0937d8426f05ae74627a37a7d7aea39b5f3ac094207e62adcd46546d539ca5a3b16cb8c973992b5d948dfc3110da6cad61103300f8bd463187d146c5c2671b79fec16","previous_signature":"a9ddbfc829c7fcbc2149419463017d13978851b4e5fa06b27b07a4bf94217d20f0645715abb847c4e2db30ef270325160c848419d3b227ccb4248c6c6c05d3551742d396b69e46b91e11e77c1b7c5eb6482db5c205f0fff844a03c60c8841c8b"}],
    [2183664, {"round":2183664,"randomness":"5b55519446ece9bb310bc5634ab0ffc8f76b1497566c97515f06920e19909746","signature":"b36d0d8ffba466f5671d202c8292986a680df788231d9debcebe1648f73c09ec3734508a0e9988a96c407aa91ca4c0e0025d15e53a271a334a026cc3556850dffeab8f6350bcdf722845bc2373c742d3fe8b3da9c423c3d55aa7c7c52d3077ac","previous_signature":"a6ceb9cbe5135e749641ce48377ee2a8c93bb1aa754de156a134d3fd83b0937d8426f05ae74627a37a7d7aea39b5f3ac094207e62adcd46546d539ca5a3b16cb8c973992b5d948dfc3110da6cad61103300f8bd463187d146c5c2671b79fec16"}],
    [2183665, {"round":2183665,"randomness":"c2a8f26d59ec6693e41216c25d9d4f1f8479171d3d702e74b59aca3482e0d662","signature":"ad245ed733a081751bf92191c44fa4d2752d225d49c8ca1ceeccb09fc78a4c1cf6dd1d71d2cd606453207e54c90dcefb02a4de1f613c0091c69cc27815d6d1fba414b737ea5433e946f258f4f78accbed0ed979919c74077395c1383ac362cf9","previous_signature":"b36d0d8ffba466f5671d202c8292986a680df788231d9debcebe1648f73c09ec3734508a0e9988a96c407aa91ca4c0e0025d15e53a271a334a026cc3556850dffeab8f6350bcdf722845bc2373c742d3fe8b3da9c423c3d55aa7c7c52d3077ac"}],
    [2183666, {"round":2183666,"randomness":"768bd188a948f1f2959d15c657f159dd34bdf741b7d4b17a29b877eb36c04dcf","signature":"93e948877a14c62abb1b611580b86c3c08ed1a732390f976e028475077e22312ada06e7f60e42a69ff8e256727a39ae60476738c74dd0485782664d4a882a6e75fef73feb3647e2261ba7a0358dfa15ecd9d67060e00adf201fbbbc86c7dd90d","previous_signature":"ad245ed733a081751bf92191c44fa4d2752d225d49c8ca1ceeccb09fc78a4c1cf6dd1d71d2cd606453207e54c90dcefb02a4de1f613c0091c69cc27815d6d1fba414b737ea5433e946f258f4f78accbed0ed979919c74077395c1383ac362cf9"}],
    [2183667, {"round":2183667,"randomness":"3fde1bdae10b7c8c826bccee66f534b82d374f88c1f8d1836063b00d2817e327","signature":"b0272269d87be8f146a0dc4f882b03add1e0f98ee7c55ee674107c231cfa7d2e40d9c88dd6e72f2f52d1abe14766b2c40dd392eec82d678a4c925c6937717246e8ae96d54d8ea70f85f8282cf14c56e5b547b7ee82df4ff61f3523a0eefcdf41","previous_signature":"93e948877a14c62abb1b611580b86c3c08ed1a732390f976e028475077e22312ada06e7f60e42a69ff8e256727a39ae60476738c74dd0485782664d4a882a6e75fef73feb3647e2261ba7a0358dfa15ecd9d67060e00adf201fbbbc86c7dd90d"}],
    [2183668, {"round":2183668,"randomness":"3436462283a07e695c41854bb953e5964d8737e7e29745afe54a9f4897b6c319","signature":"b06969214b8a7c8d705c4c5e00262626d95e30f8583dc21670508d6d4751ae95ddf675e76feabe1ee5f4000dd21f09d009bb2b57da6eedd10418e83c303c2d5845914175ffe13601574d039a7593c3521eaa98e43be927b4a00d423388501f05","previous_signature":"b0272269d87be8f146a0dc4f882b03add1e0f98ee7c55ee674107c231cfa7d2e40d9c88dd6e72f2f52d1abe14766b2c40dd392eec82d678a4c925c6937717246e8ae96d54d8ea70f85f8282cf14c56e5b547b7ee82df4ff61f3523a0eefcdf41"}],
    [2183669, {"round":2183669,"randomness":"408de94b8c7e1972b06a4ab7636eb1ba2a176022a30d018c3b55e89289d41149","signature":"990538b0f0ca3b934f53eb41d7a4ba24f3b3800abfc06275eb843df75a53257c2dbfb8f6618bb72874a79303429db13e038e6619c08726e8bbb3ae58ebb31e08d2aed921e4246fdef984285eb679c6b443f24bd04f78659bd4230e654db4200d","previous_signature":"b06969214b8a7c8d705c4c5e00262626d95e30f8583dc21670508d6d4751ae95ddf675e76feabe1ee5f4000dd21f09d009bb2b57da6eedd10418e83c303c2d5845914175ffe13601574d039a7593c3521eaa98e43be927b4a00d423388501f05"}],
    [2183670, {"round":2183670,"randomness":"e5f7ba655389eee248575dde70cb9f3293c9774c8538136a135601907158d957","signature":"a63dcbd669534b049a86198ee98f1b68c24aac50de411d11f2a8a98414f9312cd04027810417d0fa60461c0533d604630ada568ef83af93ce05c1620c8bee1491092c11e5c7d9bb679b5b8de61bbb48e092164366ae6f799c082ddab691d1d78","previous_signature":"990538b0f0ca3b934f53eb41d7a4ba24f3b3800abfc06275eb843df75a53257c2dbfb8f6618bb72874a79303429db13e038e6619c08726e8bbb3ae58ebb31e08d2aed921e4246fdef984285eb679c6b443f24bd04f78659bd4230e654db4200d"}],
  ]
);

export class Bot {
  public static async connect(terrandAddress: string): Promise<Bot> {
    const signer = await setupOsmosisClient();
    return new Bot(signer.senderAddress, signer.sign, terrandAddress);
  }

  private readonly address: string;
  private readonly client: SigningCosmWasmClient;
  private readonly terrandAddress: string;

  private constructor(address: string, client: SigningCosmWasmClient, terrandAddress: string) {
    this.address = address;
    this.client = client;
    this.terrandAddress = terrandAddress;
  }

  public async submitRound(round: number): Promise<void> {
    this.submitRounds([round]);
  }

  public async submitRounds(rounds: number[]): Promise<void> {
    const beacons = rounds.map((round) => {
      const beacon = localDataSource.get(round);
      assert(beacon, `No data source for round ${round} available`);
      return beacon;
    });

    // TODO: use executeMultiple after upgrading to CosmJS 0.29.
    const msgs: MsgExecuteContractEncodeObject[] = beacons.map((beacon) => {
      const msg = {
        add_round: {
          round: beacon.round,
          signature: toBase64(fromHex(beacon.signature)),
          previous_signature: toBase64(fromHex(beacon.previous_signature)),
        },
      };
      return {
        typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
        value: MsgExecuteContract.fromPartial({
          sender: this.address,
          contract: this.terrandAddress,
          msg: toUtf8(JSON.stringify(msg)),
        }),
      };
    });
    const result = await this.client.signAndBroadcast(this.address, msgs, "auto");
    assertIsDeliverTxSuccess(result);
  }
}
