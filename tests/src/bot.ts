import { ExecuteResult, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { logs } from "@cosmjs/stargate";
import { assert } from "@cosmjs/utils";

import { DrandExecuteMsg, GatewayExecuteMsg } from "./contracts";
import { setupNoisClient } from "./utils";

interface Beacon {
  readonly round: number;
  readonly randomness: string;
  readonly signature: string;
}

/**
 * This data source is a mock for the Drand network node.
 * It includes 101 rounds starting with 800.
 * Those rounds are also hardcoded in the `test_mode` of nois-proxy.
 */
const localDataSource: Map<number, Beacon> = new Map(
  // Generate items with shell:
  //   for r in {800..900}; do echo "    [$r, $(curl -sS https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/$r)],"; done

  // prettier-ignore
  [
    [800, {"round":800,"randomness":"dc9f6734b32abb0dbc11ba37eb11f89d014dba36c293dde746a329c0997da74c","signature":"83463aac7ce2762e9cd1333409e97a264985c941f5ccf158f40704b08e8a335d00dea8aadface32a91fbc811693def62"}],
    [801, {"round":801,"randomness":"a31eeff3aa4237d5ea7f37534ac8430b0ca03047614118113328c9e9b46d3c69","signature":"b19ef86071fade36fdefa87369b603789ebf6aa33566e7e907f5cb34dad17c2a6ee05dea1028dd9aa54daa5af37c59bd"}],
    [802, {"round":802,"randomness":"cde07f700d873acfd6495094e20254d030d3094d4b3b0a6074e89077c79b5f48","signature":"83ceca31fdf3d595caf3fd23dd30c32d95c10bea46c3ec6709b128d05c02653bf0f294072656475be413306010220ffc"}],
    [803, {"round":803,"randomness":"dfef365cdc94585abdc9bbdf61ecef347acdc9a0402f722d75e46547a948b8e7","signature":"8ebb21c371bbadfd7ffc476082e903acf3032993f7e306de1756dbe068972c61e1f732df55a98a34ec757ffacc7f0825"}],
    [804, {"round":804,"randomness":"724a4a89aae9e49896045d400643b50815b6b7775eeb58b4739f7462d8ef8b1e","signature":"9113eaf2767e24d4ecfcf1246ef1f8d45a946ab7c94d0ffd6f16ba17079e34ae2e37341779c0bff38146e01e97abae4e"}],
    [805, {"round":805,"randomness":"9ff54a7e3dd1e4eca7d7aed82bba2554808c753f7c37cbeb77aedef66cf54894","signature":"a27a460bf8c12d856119f170c59686597234405f3d3d077afae24cffed935a3ba3e51b9667e9e1bafcbe45797887e636"}],
    [806, {"round":806,"randomness":"c621d40c8abda2ee696061fc7e8c3e5ca09975412d3ddaaa5d361d7c3821bf7b","signature":"a89632b3007b931ed0b533b9a7c3088b225da8af13a08933b23204cb72a9dfd968a1714d652cfbc2cecb7ed1845eef7f"}],
    [807, {"round":807,"randomness":"e2a61f57887420ddd45ea6267c7f517d3fbe6cdc185f1489b54d3cf9931c8ab4","signature":"ad74db7ef805a5c0eef2ec67744c6c58bdf42457151fdad90fe0341eaf605495218c8e2beb2295c26640410fc8fb99ee"}],
    [808, {"round":808,"randomness":"9cc648a5ee6b1ceca60184beae05f0a910983de09ff6f05a5a6cda15d6dbb448","signature":"83c76585ae739889a31201231f091234061f54262daf5ae671201fb0db463c42061d67fa99339707af2e4ced060d688c"}],
    [809, {"round":809,"randomness":"cb6a3ba9e4f3296c54ad54a708cdd9c72a8791a2d7547be723ec63ca8398c19d","signature":"81294f3e4ce79344b37ce9135585c30aa1da0d41c2df2537cc9af4a9c3371f068b0f26c34f5180ea2f784470e66a646e"}],
    [810, {"round":810,"randomness":"192af38cb4e26fd9d15e8b4968fb3df137f3e6d9b4aeb04c7c5b6201091872cc","signature":"afd4511a3e0192f6781ad04a1dd83e43bad5b33213bc41a5c8829fc67b485d76c93a13a1e94d55d0dea0ddf2ac4778e1"}],
    [811, {"round":811,"randomness":"ec52a74efd59cb946d1d505cbf7bf4057af154af3af4b94288f62691f7513250","signature":"99bae0788fca5f5e19e93983859d827cca106f0e0bfdf9af18751ca1557857599d7b9e9c98b6eb2bac1fcc4cbfd24d87"}],
    [812, {"round":812,"randomness":"69751a1d0dc4b4a17b0c2ca45012d33fc492dd845dbcba8499fdfbb776fe6de3","signature":"a154362056b9f00d5b0099ed42a2f62f15c504a89b884f1a8c6caf20c5eca57593fc56692e5b733b668ed4907a4544c4"}],
    [813, {"round":813,"randomness":"196ef4312fb5345d9e3932816690893cbe5445361713ee43465bb856a81532d1","signature":"b4fbdfcb8ca17bff79deb4c1377e00bb76e051e2e46f650abb9b32c2458475c30437553a89429d31e2c609b8713f588b"}],
    [814, {"round":814,"randomness":"cef7f07b6d029f00cc8d43baca96cd7bfc092dad390fb194466069ce982bb277","signature":"abe26308f1313713d61674cc9ffeff67e3d707a12d345e0ce53df82905482687e8cb428510fecb90b96e4ddc673ba972"}],
    [815, {"round":815,"randomness":"6abd40ed7a302b35d3bb109d38c6dc20fdb159e39bab8b1e0c262fc4170c94bb","signature":"88f1e0c4f9494f9293face03ba8e186095cc141241ee3a166d3e351fdf8f3805d643abd5fd13a1c65b2ad96061d7ba7a"}],
    [816, {"round":816,"randomness":"402a3c21b50a4a87c1737c0f61302e7f3ab160d3eed1320281260de3070aaffb","signature":"aea8c9dc4a6c1f8ad1870bc7ade459eb611b717a2cd31e769fe023de9f36891c08c14c2b7eb5f59e524b0a6eeb2e7d53"}],
    [817, {"round":817,"randomness":"ebadec3b4c654cb1e159770c09e15679fa77447ada6e84ec9b30fcefa7131453","signature":"8ea7c1ebeef8c8d3725cd515b7964eff76406e9a7e94cb711bce34b67667d5501cc8f2c49077adce41d57f84d48c7c27"}],
    [818, {"round":818,"randomness":"1dfff3b92e34bb727454342531bdb5af67f1bcc73a45b84f1f086bb57a4c8ac3","signature":"8b4214a022b8e7b5b75eaf0f5d7656c81147b29244e67574d7dc9ff6aa60f10f9406c536c05fe864bff4413158b775b1"}],
    [819, {"round":819,"randomness":"2b7d8b620455434ca1abed70966d3db860c0236bffce4ddb687182702f8f907d","signature":"b88a4e7cc284259e60d445a1da41814aa980fcdcc7f265cc3c908e9edf5134393690b9aee529193da3e1210b62b98359"}],
    [820, {"round":820,"randomness":"32f614c72e9a382540f6cdca5f4d58537ea11de9b692bcdef7b10e892690d233","signature":"a6435fd2a1bfbe3c2ee80b9a0fe3e05b302733e0e34f29f1b3e5bc4559798ec485af2e46022b49c6b3932988f74f9d62"}],
    [821, {"round":821,"randomness":"4d9df41b928fb757ce55a2487cb0bab369eaf883102a69435f9c94e31bb4cbe0","signature":"88ad76966ab319f40f6ccd20b8fe7d40144e49dee65163ec6f93a43e0f6decb00a1162bfa16c93845fee0b5bfe2aa02b"}],
    [822, {"round":822,"randomness":"d2f0e35125e60abe157fb2b609eadd0542f4ada276bb974f83dbc62c5091a6ec","signature":"b09393e7d1bf3b5ed2bcf477ce3e2ecfa77713b7ecf3ea8dab8ccc56b7b42511f79254a70bb3edbdaeee6540fc304f1d"}],
    [823, {"round":823,"randomness":"6957eecc23eabeb5ed68a48f86b726cbd23d41df178da8ff7cef31676db89461","signature":"b7438cb73e2d541863b7a86874ab4a2e87405e3153409894d90b5176d6228f883e6c5023b665351886ebf53bbdd53641"}],
    [824, {"round":824,"randomness":"bc414be9536c82b35c7cb24564a336c81d34cc22448ab22d4ef0ae1e75f6574a","signature":"a08646add9b4affcab7337532211dabb20e83943cd99bb434f259c3ccbf4fca3e7bdadff54808fd6b6981d10fbd34035"}],
    [825, {"round":825,"randomness":"25227bf6ab5d7fd65d3cc90577f20f7a42eb401b4ff0c19206d75b31b285c2cb","signature":"991dfb8e8bbcd5509fedad466809440405bf341038d861a7a28a0215693d1e00d071836bd37ac6c2a4383ff5490b12a1"}],
    [826, {"round":826,"randomness":"25f60dc88bcf37acd60927abfedc8793240fcb49d7ff0b8d7eab091c6effd486","signature":"a30a63ff7d15edfc817b67155025f4745c49aae23688c619f255583b8a4958e206772547203c134e7c25171874e81580"}],
    [827, {"round":827,"randomness":"b441185c44913b308f11b1a95d3d7c377ded73d883d5c39efff7cc99f0f10143","signature":"b69328234810f336f097543ecbd01f501bfd8a60b132b453ba8212a0863f781a274c8d6dc95a08fac17e80f0a4562c64"}],
    [828, {"round":828,"randomness":"5a239dcc393a5bbeb50957739bd797a3d305329b26d2ae96660e886972248bba","signature":"b03b71b93453b1336998437b48962103cb15ad82bb6e3d50ebe86a9f9820a013332af54b0a6948bc030d5e4391c0582b"}],
    [829, {"round":829,"randomness":"2a21c2e651bae71d4bc84674a3d18925995e6942a8d3c46c7288d965fecfddb9","signature":"980d88e56e53eddff031574a119d4a637443df5e9d72ec1ebe87896eff53e1dcce31c04e96deea0d5385378db7a4db1e"}],
    [830, {"round":830,"randomness":"9e8d112e4c9b66e17ca3cd78aca91e6c076a42917a03fe1fe837f7eaf2fa8b86","signature":"8bd8e49f8b2c648847df79b8e526c3d8ed4d43a6dbe8fdc29d9320a1ec46b0296b2b5a641109ce94905f9767ca215daf"}],
    [831, {"round":831,"randomness":"db483a02908a9ea615864ec64f68db159b6c45fde6c613879958102a4f1bd630","signature":"8b90f940e0e0792d2d77b6319e1964913098c808767be754b3456006de906fd777583c38f0b697b00c3d8036d78d30f0"}],
    [832, {"round":832,"randomness":"36df2d369cbba7bbbc9d8ec96fc7d705ed34701364b6bf0ee288c7b450d4b573","signature":"8b54d5a2bbf25e87606bd9310430d2174b7592134fd2f7e4d705a13259a7d8bc85b64bc27efd181f5eafb0fcd251124d"}],
    [833, {"round":833,"randomness":"3efc80a47f320460bb33b7f2d45f9946ab93db3381a9ed5a9dfae829cec709e7","signature":"a3754abfbc26fb25efab423db172591baaf03e93d7b155212abfd276427319a8c6dc348bae1f92beed74c29ba5fd1519"}],
    [834, {"round":834,"randomness":"320884d53832895cae04144c1ef90baeda23ad0e22834f1a1d8a5c4908899d72","signature":"8f6f779b681e3b7981dce38a38105a7c636a84091d85374833c5683d9810aac45658c61f33b417fa54af5e16a00385f0"}],
    [835, {"round":835,"randomness":"d22e7bc3b4e4552171c369ff187a17e392d65b8e82152f160e9ffae8a7153ea7","signature":"905f5ad6899c601657d4f32673b3ae399ebba9e3958786b6e2e6757a499533fd42a4b311988c5ef9b0109a18d61398d6"}],
    [836, {"round":836,"randomness":"37f620d363581d51be1a278c9415a6342d6d2d729380857995e18c57a7753493","signature":"b2aa9c4331d2a6c032cd28f7e4c15459bcda30fd43ed79ced186a6ce772a78d5366822ea0dd73fbe021be9e29f542e32"}],
    [837, {"round":837,"randomness":"67ff1bfc9b46e43a931738f8937c985bd501bbd08192f589b2eaa98c1a21803b","signature":"aa475bb56988f5184e64c53debb3049cdb23fa8b2c6146876559d701974361c39b6abe50a54c15ac2773e35afcf8a7c5"}],
    [838, {"round":838,"randomness":"b156068ed2e637732492ad07cfa74402fd297c51409ceedc0f441cd3a46f01ba","signature":"8d24625a6891a0b2c4a74c8a5bbe58ee81c21c86a400cdd39d8f076f6b16d6281caf9d62eafe25495e627e471d0036ea"}],
    [839, {"round":839,"randomness":"caf0f97150b480dc55f7cdfb59d213c77724ccf34a409adba12df4d1e4e8deb1","signature":"b66a5a6066f5ac37a9c7ebfc04f4a8a539470bb78845da818f4d67076d90e8b96c88dfbb7f32ff88b8509b3c1acbceb4"}],
    [840, {"round":840,"randomness":"59b949f6455a6d7319232f8fe085cbba884727cccf79fa5239579078c0a19cd4","signature":"90ce009e12ca4a1a3b6be6f8148be264383e104df2bd34292771b5641fac1abcc460cf12c4a0fe22ec78892c0bc6b402"}],
    [841, {"round":841,"randomness":"7402719b0c566c4e3d1b21952fb86b20a4f7f3ca3e1e83269f72c958ad7585c6","signature":"ac22eeb075f8b3a5721eb0d62bd38f9d7b1d46fbd46c5999f9fff60f94acc579e5024f989ea9bad6ba9621fe804edbef"}],
    [842, {"round":842,"randomness":"72efee2ddcbcf744b6f8fed2d3099e835d5b157aed99450b7ef377913c5d9e13","signature":"b5166ac69d031507d8f40bbca2a93cd99587c3b3f3144fd2e4dc4d89764908fdecda9a8e464f716beb1d4d69dbe224ae"}],
    [843, {"round":843,"randomness":"d68f49925cd5f1b70b71a62de83b53539ffb22aa39cc9a60c76efe9798f9d30e","signature":"912754906b829dc6798ad330466feb5e24325b57d3ba9e449c27846da154fe554f05e59118d2ed0bf4431113f429c721"}],
    [844, {"round":844,"randomness":"3fe8a9b2c3668d1410e707a1276a1bbd886bae5464ec22d13001860a8465f0cd","signature":"b2f30b96c665feed6abc6b7d2e1a121d937d129e11767093c95d631096f9091857267136e979ee1d6cdd61c69c82f523"}],
    [845, {"round":845,"randomness":"e4c5cfa377fe5501b9d40082b415b5de1fc496b794b9cc2267c6e1caecda1674","signature":"a54c31f8b02b046fd651a6bac6903bdb78af45ba8e57f1fca0decd05e848091a1a50a33f7a35220373a97e8ce90bc156"}],
    [846, {"round":846,"randomness":"53022718394b5fdde1ced5993d5e2b5c4e5889edab3b40a001e000bfea44ff18","signature":"ae0271e232c8124b90663bd371d850c0e3f0875fba3faafc4cd69414c375b899ff7931149c4eb40e6dc919e3d280a6a5"}],
    [847, {"round":847,"randomness":"177ff22806e21e9147b11d3c7258e15c276dd37c847e965ac56151b5bd43ed26","signature":"80e6c22a86f2cd2e4301c492f0cd6cdcb37e5c856afd7c1087c838b03bb963dafd146b80c88853666d3147a119bb3d59"}],
    [848, {"round":848,"randomness":"d3accdbc27d11d0bbe6719478902bccf76fa278e954fc1711538787315e6fa5d","signature":"87be7652aa892a831e66f59a7a57ec38e52442bc13be6089026a4d3577d0fafea489e76d61924fdcbaaf658e0accf7e6"}],
    [849, {"round":849,"randomness":"0a0575babb0d0df8e6b5a2250c5098297f331c8257a7bb9919bfaf0b59bdd614","signature":"90471579d1e4ecff4bdd262aa7448b6662508c0e2356d2b9a1b91b42c88636769367c9c93b3b662026bb7a8f97559a2b"}],
    [850, {"round":850,"randomness":"e02e7b7b882e232c777b4f112e264f8c19ebe74d948f5e2e6f7fc3cb095090d5","signature":"9796fffec6cd763899f4eb25191852260b891b965dc02d8832f9585abdea30f3393818e63e9c1f3ed762d53fbd2e5caf"}],
    [851, {"round":851,"randomness":"88d2bbcf10b5fb1c1b14cc587312bb588849ef6caae9a43bb7353b81c796a5d9","signature":"b64beb6e436731d2ecb9c0bf0d4ce305c256dffdf0e1495c509f90e7bf20691518de33ad25ecb8b6bd022e7c477e6e8d"}],
    [852, {"round":852,"randomness":"0c9e5f0c8f1a2dae054999467d3c034dfbc0f7dcc03225fbe4e936279407a636","signature":"989bf888f84b6bab012f20abf8c2adc4c2b0cd953453d35664972a69ec66df3eff46ec60524d60dbc7ba9c640bb8681c"}],
    [853, {"round":853,"randomness":"23987f7caee956ab209427d23543490fd5e7b9150877222b2f2828525ac91776","signature":"91e5dc6cddb872263799a2146c03fd119a12ab98bfa4db8e471c137c4398c40279d39c82e5b7c3720d5f0314ffe16423"}],
    [854, {"round":854,"randomness":"9cdb782e6a3aa315cc35c9255f4cba2f04db1402a4d53c68de00cb4602dac58d","signature":"ac7a11691804fb1a0731fcc180db9fb9ebd6d49b694e6d748cfa2897a4253215b9dbf81d34fdad04d56d4c4334b5be4d"}],
    [855, {"round":855,"randomness":"6883ab3896c1aef91ed938c8862343e80ab99996b4b86461567e600584559fcd","signature":"8bac08d80085f933ee75fe441b71c3c5d2acea184edfd648fd67c8bc706eaccc8473cb5dd8f030de5c979c44dfd8d6b9"}],
    [856, {"round":856,"randomness":"01e12fd14663f8c6513b2d5960444c1831d4ae4718efb06429a6d149040bd3c4","signature":"b02949b9eee658c04b5eabba286258bdb473c8336131690be088fb6d18aa3b69b05c940e4b11d10171309ebc23163aee"}],
    [857, {"round":857,"randomness":"03ca143fdc0621ea04ead68067b7cd61039e7c4171881c4bb765355347a71e58","signature":"b8fb081d81e6bc3f059bb9f8a4c36a0ac8b035fcaf30027c2fa7a16cc979e921a2ee90618a6c1d992849c85dd01102bd"}],
    [858, {"round":858,"randomness":"87d0b3e541b4430e56cfdde270c3251904242046795de5df7a2fbfbcc0baf523","signature":"8f6c274b76826ae5bc514932f5d3ca903cc15d2b38e850e2596bce2c1dc4e24c92ff76806534f123da28076ccedd715b"}],
    [859, {"round":859,"randomness":"f29190255a513ec99b6621c8be6695a85deb44dcef6aec511be180fb370c9b69","signature":"a88ec45ba674bf66640622c366e4f0bc86e1ab009a4ebb72c7b09d410a4e5612f981cfaec9d03d29133c59492e6c4185"}],
    [860, {"round":860,"randomness":"6f5239015922282170988b630f19e79d0ba9a6ca635e8e21fa9b675981f706cd","signature":"9519f0e8dfede65e9c795813fa83c83f49c19d707d1c682ea64811f02eb773c4c9d65c2736ccccc393b3c89bdac13f8b"}],
    [861, {"round":861,"randomness":"379eadd3b686ff543cb7f5d1bb77eb32357368049bf3e1164fdd318bdcb8f042","signature":"976a4fc6399bcfe6ffffb7004af8af9437f413090f4cc407a386d1af9744536f2f2abdd18b1849ef6bfd24956d7ec386"}],
    [862, {"round":862,"randomness":"45b6a0bebcdba515c13fcf5a639289fdc1f9613491f2327ded10bf62d34d52f2","signature":"97d573d443aed61269233091acc9ab2ee19237e572276b82c594cf35ad9f3908394093ca32febc022c6ede68d238a2eb"}],
    [863, {"round":863,"randomness":"ec811cafea7494e94bd78228bea8577968fd4a339baa6c40a43fd2691d578d50","signature":"a1075fa20ededf39f4076db92052bd98c702f116747b8a3ad3fd5fe4302f5759d32714c76493f7a3c9b4ff84cb013f15"}],
    [864, {"round":864,"randomness":"d66a9aaace6d5ff7468b7e7b7eb20222d6472bd90c814a48d4e65a33c543fa9e","signature":"b8358f06a9b8cb464729bdecea2b194d7ef5ad6b98c746c995ae6bfc0a1d033f0b0513e8e3fbb27270411784175cd2ee"}],
    [865, {"round":865,"randomness":"1f4ddf8112550541d33b6a06c70fe3d7bec864596177f5a9bb87e9c9224590d3","signature":"94ff0b45bd5ff8bd098db02450094e4a34281d2b790ed4a3484c1b7055fc8994bff6c9781d061251cce96f0ca37a83da"}],
    [866, {"round":866,"randomness":"e51bf39ebb352a4578eb77f0bafdac9a75a1dd41f261d440bd52efa9630b8d2c","signature":"8bfef01e3d574c35810550269bf3cb29a292c42c395a2b4bb678013cfd4520bddb4afb77cd559738219a54643c4d3be2"}],
    [867, {"round":867,"randomness":"e520aef661c7c356d1d501d87709516403fd5ce14c5307e1f31156ad22895a18","signature":"b3b9da809e12daebc763890828cb42d631656ab1fdcd1b7e45a5eafd16504ef0e6c17ccb6a2068a3ad3b604d2fc613d4"}],
    [868, {"round":868,"randomness":"82454f7c55d4adc18a8adeb27ceb170985d5d8931ca3b6e7a9a9d85c3f2b29ba","signature":"b80061753ec63cc023e029556a97ab37748057813ba3d8e72baa3c2a0e42646c355714ecbc86e57b448e09022f2ca1ac"}],
    [869, {"round":869,"randomness":"3bea7d9e0c2f4dc9dbe0071444178c0ea710511158870489b9cf7cd197c5534c","signature":"8aa7b6ac9250ae61fd0b63bc6d8d6a0e55226fe58a92b0286bed88587a13180a8a904bf6e9313c0e435c0658726c6423"}],
    [870, {"round":870,"randomness":"4eb18a34c58cc88500f73d0e45ed41976ebb5db41002ace23e27a0639281c425","signature":"98c036d9855466429bb341268985d7e0136a15a13a74733f061b7f7e7c986e9157895c70c273c5e18c16d236aa84c37f"}],
    [871, {"round":871,"randomness":"bcc4a78baaa5bb56e4d67ccc06b90fd09740c81472eb22be9ab32992988aaed3","signature":"b45c0124c9fe8cb15bed69441ceae8dd241c9e9ba251bbd0fcd69e8ee8cfbcb2c32956f49ec53bc066dbefe41b232225"}],
    [872, {"round":872,"randomness":"1ba9eecb2a266d5ed4a5bd01de0a48b25dc88a45861747b7ed0a126470993c6c","signature":"8811a6b6ae763e362c05694d3379951157abe376f2fce5413d6c78f2f7a14a02275f5fec80e15bf76d05f9b5df7986c7"}],
    [873, {"round":873,"randomness":"60f23f4e7ae0d990cbaf2776306ebf9b042fbdc5f4a9231985c085a2ea0dcde2","signature":"90cc9334aef30d74bade9715d6b27b6b8128232a30803d0c0f3d7c298f8f7111ac9def8e9cb2c94388be599235ff5abc"}],
    [874, {"round":874,"randomness":"7e839b43d2e71083f7592d6d4232eb933985b9e9e0c0a04f6d3bd485d6db5da3","signature":"a8ad0f187dc8983105713534f8b7a5580495c9e46869260439ba64fc4e74f79a3739ba087a6d96aef3db5e81dab2fd08"}],
    [875, {"round":875,"randomness":"46cd54d4cb67183e96835902f2199114be5dff7d9de2c1805aa4ea02bd7190ba","signature":"aee524f717abf7fb1fb08802a27a26632f3fe570f8a426c634cfe202ecf9672e1a33e3dddad7c0f17d7caf974bfebe34"}],
    [876, {"round":876,"randomness":"3ccbaf692060740673ac7368ba5939a37d0939a62048a616ba9984e6ab9b562a","signature":"b1d4f558eb653ab869587a3137d2dc9cffd89f4fa39357770d469c6db46fa75f44e74770c8bb24bc016305e8db5269ec"}],
    [877, {"round":877,"randomness":"1030c6625a047c6f29503f97fbb3c64aff417f0830a26644dfcfb63a3845c06a","signature":"b3026cf39e2927cd82073e7991177159dca6859320653e6a0d69de6c559e9f518667aacf10e6945ac927519596437e7f"}],
    [878, {"round":878,"randomness":"79e6541e82b23b9a06a3ea78a03242ba168299972cd130bf9002cbbc6caa5345","signature":"aa4cf387a3a863621925eeeaaafa8045f52cfccd88c00f17a047a9c99ddcceddec8efc604caa0cfc7749936cc56a9243"}],
    [879, {"round":879,"randomness":"cbc51200be48a35b35c3fcb6b698e5b8418bf3952d2cf3df5e1f75808c1bb66f","signature":"90e0823c6d1fe96c7882e8a679114599277d76deaed9b0515dadef9f390af345418c9ac75f9b22ca1eeadb2ed25a5bda"}],
    [880, {"round":880,"randomness":"4d2fc6e19d1dd03a9651f04babf27a90d2780e3c4b8b7011ce49e2db13b04d31","signature":"b30bac238d5f09324c3c74781ff7de1f2acde2614e0833ca67ffd39532705e6f29f3cba0d6abe6438047fed2cc5069de"}],
    [881, {"round":881,"randomness":"4973219705065192a32e28579fd6ca07c31dc53575fec5b98005eada55c0ff0d","signature":"b45145b58ec1386eb97c2ead57182c465ae4c55e62be157c49a4904c86eadc290e0d4e8458fe137bf9a20ad9e8031abb"}],
    [882, {"round":882,"randomness":"4b4dcd4404cda14fb6bbd90ef55ff6a6637b46437ee492821edea6fa2712a049","signature":"80d98cdc4bcdb64daa670921aa343fb0395814efe1e1cad868256305576633f0980c091bc6b5fc1429b26d0cbb8ac678"}],
    [883, {"round":883,"randomness":"2b5ab71a659a2e69696bd59db2037d6c97950d01c35d1861a08578380b974b4f","signature":"8c8cb313d8c64a807d163e6dda70b98ecd6ff2f31ae51fdc72092c7d7c15231b70bfa75d9c2d4f9c0acd510dcf8a68dd"}],
    [884, {"round":884,"randomness":"7bbd517549887da31d09b690b51992db5470311f09bc6f2981ca9d9472ca434d","signature":"997a54c4407a5f6d3bce7b42308efcfb9c3f35347f4db2c7ac88248389bd1afc7a4b819c377201a0b97ca7daa127c36c"}],
    [885, {"round":885,"randomness":"177732b3e96d692037d0c7c26337ea03186cf3c7cd10afb0420e4a00cf770b83","signature":"b6804e84d6c713ea8d83962b6af9b8ea5b2f97d8be1eccee5c27806b5adc7d20fcdc59b868c282e623892ba1617b9497"}],
    [886, {"round":886,"randomness":"ac4f4984ec76f2d9df2a0825e43cee6cbfbd5c4491e8afb957e6a440bcdfb2a6","signature":"82d350c6687b222612106f289ac421867cd26b98b8997d4da2f4271c9831c102fcba4f526a2be65d228b8d156bc063ed"}],
    [887, {"round":887,"randomness":"0b88f56bd1d885b89aec582acdd70b7811f1b407df63963076ea37b12db527b1","signature":"b3ac0db9e9041d25f39bb1ef0fb74b4253d57513a7fbadd6e608d3f3b12340212fc2cca918a3584224ae1a2d72f9141f"}],
    [888, {"round":888,"randomness":"acd9436a489088e52bddce9891575687d2c97581b39145d643cdd05ffd0f1705","signature":"afed8cadf074d572a92e340f45cb38b1d74dce566e627b3555730ba40d2c6d482d6e3f4d8460d57df251f1a7bb46ea30"}],
    [889, {"round":889,"randomness":"05e7fdac92b02948b4c7fab5a7a597db2c8c6f8339cc04033e8071793fc75b07","signature":"915410864f65de3c14e228ed6d78e0adcb03d209f23f2859296dc0fa9bb51dcadd6690976f50949c5e20cb10bb2a8cfb"}],
    [890, {"round":890,"randomness":"a96bc028106ba9c03d2af47130f907282af36a900f4fc60a20c5a99cf12d639d","signature":"8f042aa6fecc45c8ffa7d2b6073e422af42f1ddca2d09118d36801f275d2e236358e6f00e152eaf33f43fe57cc463299"}],
    [891, {"round":891,"randomness":"70071e72119e6b7a73c57e84c3f3f95eac474477e6d151d5973a61d3a285c277","signature":"8e51c48abcec0ca69a7de404902481a6ca9d014c8de04f2d4a4548449b107fec9cf2cb7491a47ed3589e44c743ef9569"}],
    [892, {"round":892,"randomness":"287aca213d24766b0e6bdb811b376d1d54dc413d6064157f66ea8d469e6846e3","signature":"a901e4d7fb4b07bdf329a3eebc0ad432e0431d37bdd2ef119cfc008d076d2989b818e7e2d7866601794d43311dd7e4f4"}],
    [893, {"round":893,"randomness":"a95fbf5b1e6d615acf8aad50548529e858d8244ab261a9e5afb3b0ca3a1604cd","signature":"8f79d7f1e68e323b108c25c117b572cb2708e7889d1fc3e04e945b354d5e299c8d9aff1cba25f9c12e90b41ac46ec614"}],
    [894, {"round":894,"randomness":"fc3585b97e7dbf13cae4dd42e488619299378e1749d0798e1aa2247b9dd87f52","signature":"87c8cf0c0bbe379700318c0b6b8070ce714a06d026d4708ccba0db7044627177e5d843d48fa60c772f722b17402fc227"}],
    [895, {"round":895,"randomness":"593be9f6a9413690cd193cb39b9e64d3c58edbd6dfdc81667ae63be1f8049067","signature":"b9c2fb5e1c335633b0d7445425a927d3029985efbba46d322bed8a8ed317b9782416bf77b07f75069581f161be303eaa"}],
    [896, {"round":896,"randomness":"5df4875d32112578b8e50a4f15dcee26592ec6e81a7820fd5501199951782cb3","signature":"8895ad2e61889ee6b810ef115e35f02d383661a154fa3e3206396c4d1523dc85cb813e4f08c357ad3baa8710e6fb903a"}],
    [897, {"round":897,"randomness":"0c63329e487e3dd28292dd692775c774a38948a1ccc3d4f7f8477326633a3575","signature":"b339b9d8b812b792c6a76e32d366c7ce79b1fbeefd779a2738e1b3bc0b783d57555023316be421d89429a2eade342118"}],
    [898, {"round":898,"randomness":"cceafc795496af5878fd771bd8ed3121b968f6dd86eb8fa29a338d09b1b3ecad","signature":"abdbb49c2f51bd28610ef76196e9401d8cc702a7f588ca2f31964929de93a35d6b0c156f8666081e5d65036ab2c84198"}],
    [899, {"round":899,"randomness":"114ff85c73c904c483b4cb66392ac7a252a8585e5ba8d2c2d7c03f9cfa67fefa","signature":"8f7e4fbac5ca3ff0a7dc30ebf6a019446ce96dafd9bfcb0d4810a434d69853cbacee64424f36e463c77124ca7fba495d"}],
    [900, {"round":900,"randomness":"ee6851429b510a97473785145f0f42f9e5544505fb1d80d2ce31dfc64cbbd68b","signature":"a0a6579d1a8cf17ba35bec33a2112432c647afa43988acc92c386d1a3051f3aface8bea280e0a70f729c7b44203303d3"}],
  ],

  // Publish times (https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=4d0d6d84cdb9b48260594e3b74aa86ae)
  // Publish time of #800: 1677687597000000000
  // Publish time of #801: 1677687600000000000
  // Publish time of #802: 1677687603000000000
  // Publish time of #803: 1677687606000000000
  // Publish time of #804: 1677687609000000000
  // Publish time of #805: 1677687612000000000
  // Publish time of #806: 1677687615000000000
  // Publish time of #807: 1677687618000000000
  // Publish time of #808: 1677687621000000000
  // Publish time of #809: 1677687624000000000
  // Publish time of #810: 1677687627000000000
  // Publish time of #811: 1677687630000000000
  // Publish time of #812: 1677687633000000000
  // Publish time of #813: 1677687636000000000
  // Publish time of #814: 1677687639000000000
  // Publish time of #815: 1677687642000000000
  // Publish time of #816: 1677687645000000000
  // Publish time of #817: 1677687648000000000
  // Publish time of #818: 1677687651000000000
  // Publish time of #819: 1677687654000000000
  // Publish time of #820: 1677687657000000000
  // Publish time of #821: 1677687660000000000
  // Publish time of #822: 1677687663000000000
  // Publish time of #823: 1677687666000000000
  // Publish time of #824: 1677687669000000000
  // Publish time of #825: 1677687672000000000
  // Publish time of #826: 1677687675000000000
  // Publish time of #827: 1677687678000000000
  // Publish time of #828: 1677687681000000000
  // Publish time of #829: 1677687684000000000
  // Publish time of #830: 1677687687000000000
  // Publish time of #831: 1677687690000000000
  // Publish time of #832: 1677687693000000000
  // Publish time of #833: 1677687696000000000
  // Publish time of #834: 1677687699000000000
  // Publish time of #835: 1677687702000000000
  // Publish time of #836: 1677687705000000000
  // Publish time of #837: 1677687708000000000
  // Publish time of #838: 1677687711000000000
  // Publish time of #839: 1677687714000000000
  // Publish time of #840: 1677687717000000000
  // Publish time of #841: 1677687720000000000
  // Publish time of #842: 1677687723000000000
  // Publish time of #843: 1677687726000000000
  // Publish time of #844: 1677687729000000000
  // Publish time of #845: 1677687732000000000
  // Publish time of #846: 1677687735000000000
  // Publish time of #847: 1677687738000000000
  // Publish time of #848: 1677687741000000000
  // Publish time of #849: 1677687744000000000
  // Publish time of #850: 1677687747000000000
  // Publish time of #851: 1677687750000000000
  // Publish time of #852: 1677687753000000000
  // Publish time of #853: 1677687756000000000
  // Publish time of #854: 1677687759000000000
  // Publish time of #855: 1677687762000000000
  // Publish time of #856: 1677687765000000000
  // Publish time of #857: 1677687768000000000
  // Publish time of #858: 1677687771000000000
  // Publish time of #859: 1677687774000000000
  // Publish time of #860: 1677687777000000000
  // Publish time of #861: 1677687780000000000
  // Publish time of #862: 1677687783000000000
  // Publish time of #863: 1677687786000000000
  // Publish time of #864: 1677687789000000000
  // Publish time of #865: 1677687792000000000
  // Publish time of #866: 1677687795000000000
  // Publish time of #867: 1677687798000000000
  // Publish time of #868: 1677687801000000000
  // Publish time of #869: 1677687804000000000
  // Publish time of #870: 1677687807000000000
  // Publish time of #871: 1677687810000000000
  // Publish time of #872: 1677687813000000000
  // Publish time of #873: 1677687816000000000
  // Publish time of #874: 1677687819000000000
  // Publish time of #875: 1677687822000000000
  // Publish time of #876: 1677687825000000000
  // Publish time of #877: 1677687828000000000
  // Publish time of #878: 1677687831000000000
  // Publish time of #879: 1677687834000000000
  // Publish time of #880: 1677687837000000000
  // Publish time of #881: 1677687840000000000
  // Publish time of #882: 1677687843000000000
  // Publish time of #883: 1677687846000000000
  // Publish time of #884: 1677687849000000000
  // Publish time of #885: 1677687852000000000
  // Publish time of #886: 1677687855000000000
  // Publish time of #887: 1677687858000000000
  // Publish time of #888: 1677687861000000000
  // Publish time of #889: 1677687864000000000
  // Publish time of #890: 1677687867000000000
  // Publish time of #891: 1677687870000000000
  // Publish time of #892: 1677687873000000000
  // Publish time of #893: 1677687876000000000
  // Publish time of #894: 1677687879000000000
  // Publish time of #895: 1677687882000000000
  // Publish time of #896: 1677687885000000000
  // Publish time of #897: 1677687888000000000
  // Publish time of #898: 1677687891000000000
  // Publish time of #899: 1677687894000000000
  // Publish time of #900: 1677687897000000000
);

export class Bot {
  public static async connect(drandAddress: string): Promise<Bot> {
    const signer = await setupNoisClient();
    return new Bot(signer.senderAddress, signer.sign, drandAddress);
  }

  private readonly address: string;
  private readonly client: SigningCosmWasmClient;
  private readonly drandAddress: string;
  private nextRound = 800;

  private constructor(address: string, client: SigningCosmWasmClient, drandAddress: string) {
    this.address = address;
    this.client = client;
    this.drandAddress = drandAddress;
  }

  public async submitNext(): Promise<ExecuteResult> {
    const round = this.nextRound;
    this.nextRound += 10;
    return this.submitRound(round);
  }

  public async submitRound(round: number): Promise<ExecuteResult> {
    const beacon = localDataSource.get(round);
    assert(beacon, `No data source for round ${round} available`);

    const msg: DrandExecuteMsg = {
      add_round: {
        round: beacon.round,
        signature: beacon.signature,
      },
    };
    const res = await this.client.execute(this.address, this.drandAddress, msg, "auto");
    return res;
  }

  public async register(moniker: string): Promise<ExecuteResult> {
    const msg: DrandExecuteMsg = {
      register_bot: {
        moniker,
      },
    };
    return this.client.execute(this.address, this.drandAddress, msg, "auto");
  }
}

/**
 * Like Bot but submits pre-verified beacons to nois-gateway instead of
 * unverified beacons to nois-drand.
 */
export class MockBot {
  public static async connect(): Promise<MockBot> {
    const signer = await setupNoisClient();
    return new MockBot(signer.senderAddress, signer.sign);
  }

  public readonly address: string;
  private readonly client: SigningCosmWasmClient;
  private gatewayAddress: string | undefined;
  private nextRound = 800;

  private constructor(address: string, client: SigningCosmWasmClient) {
    this.address = address;
    this.client = client;
  }

  public setGatewayAddress(gatewayAddress: string) {
    this.gatewayAddress = gatewayAddress;
  }

  public async submitNext(): Promise<ExecuteResult> {
    const round = this.nextRound;
    this.nextRound += 10;
    return this.submitRound(round);
  }

  public async submitRound(round: number): Promise<ExecuteResult> {
    const beacon = localDataSource.get(round);
    assert(beacon, `No data source for round ${round} available`);

    const msg: GatewayExecuteMsg = {
      add_verified_round: {
        round: beacon.round,
        randomness: beacon.randomness,
        is_verifying_tx: true,
      },
    };
    assert(this.gatewayAddress);
    const res = await this.client.execute(this.address, this.gatewayAddress, msg, "auto");
    return res;
  }
}

export function ibcPacketsSent(resultLogs: readonly logs.Log[]): number {
  const allEvents = resultLogs.flatMap((log) => log.events);
  const packetsEvents = allEvents.filter((e) => e.type === "send_packet");
  const attributes = packetsEvents.flatMap((e) => e.attributes);
  const packetsSentCount = attributes.filter((a) => a.key === "packet_sequence").length;
  return packetsSentCount;
}
