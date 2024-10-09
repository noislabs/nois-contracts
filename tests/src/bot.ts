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
  //   for r in {800..900}; do echo "    [$r, $(curl -sS https://api3.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/public/$r)],"; done

  // prettier-ignore
  [
    [800, {"round":800,"randomness":"a90a2443ae885159cac9b6081fc75661ab2004b37ddfe88b7fecdce33f1cd82e","signature":"af0cce28a9f95c24c8b9c337aa36ffabcc9d85e7a3fec4f562206dce3e44384aec8f18a1e3359df00ee4698beef9160b"}],
    [801, {"round":801,"randomness":"a4018181229052186a0df484e427ea9f53b8156031c1f702ac1c83ceb110edd8","signature":"b83761e5702777682c9e98031868f90a08d074914cf29422c815de8ffe75df9089be1bf1d200df7c6201bcede8561a13"}],
    [802, {"round":802,"randomness":"0d39bd76700384368939b7a019f75a6d127d4e6454bfe721a52fad004af23dd2","signature":"8b3d29f76f14b2d9ba9d4b55124ab1a7367aaa19306ab86f804b6282af540011ad9c710bf06180ad06165f9a319defa3"}],
    [803, {"round":803,"randomness":"c7565979a9c4592439bcdcea2d3173b51add6ece9beeaa51476ae091ddb7b3ef","signature":"98aeeca9a90dbbfafa839c9a44e1adb2791e8e89b7e1e43ec46cbfdd731cef96bc8e8b0ff0f1850af546b4ce497790e2"}],
    [804, {"round":804,"randomness":"4cc693d3cd792a955555a1607fc1207153780a7cf224d4761c59bb219b72beae","signature":"b9daf6c79ea0c9b57693f9f6a1e9746b8a4e67771b25000187dd72a9270d88803121ec6b8662443e61baea610f7c5d35"}],
    [805, {"round":805,"randomness":"a4bbaf83208dc7199d84081a7acced29daac2e120a5a3a57b7d8eec5cee30f6d","signature":"b1a3dcb7d2819123f6a140adb6aa5512da262f0bd34f4a244ce1b1958b61a5e27c5489f7e7fe74008bc0ea3bc0797d15"}],
    [806, {"round":806,"randomness":"9398313d98c3118c2913f26c0fd43a94326cf94dab20eebf763f1d565582894d","signature":"ac6045c9e30efde14c775875ac1150bcbbe1e0a8ab7434821fac04e34fffc36160d06a3cc44917e340b060fdff2d871f"}],
    [807, {"round":807,"randomness":"8199a798a49a1a51900090b938ace3f0f63313e3a20953c89eb9d9b3f08f7731","signature":"897aaab94257f5ec622d40a8f2bf1068788afad8e8f672fec38ad2232c9fc7564eb007de09ec9dcce1efcded30c4f52d"}],
    [808, {"round":808,"randomness":"04fda9b3a0364145564b7ce0e8a025714d90e9fb7384de8cf8cdb1d77f66146c","signature":"a0c3a881f0c9086bf045906ac34df80ebeb54212a55063c22558fb8a93730f7d22df0835304c4464ad43e8dfd7fd7dfb"}],
    [809, {"round":809,"randomness":"076f54706e3910bc2fdb9673bead443c5fa6262dc83e4dea7aadcc13d8f8b027","signature":"95d35ed0ececca2a1af11a4ec1db3c70be1b1f11d0e600030ee74202e5d61170c4cda1b318f83c426e81fb7333a6629c"}],
    [810, {"round":810,"randomness":"a3da4f4f5f4c2f5d901a3a731abb027707b3c839b8d2ccfde0343de2a324822a","signature":"af1e9c6f040086f614d48e814ee551b663de868e14492f56a9b5dbc264b51829b76b83ae1d2cf1dbe7ec6f3d9235d104"}],
    [811, {"round":811,"randomness":"529bcd02011d10dde4e5fd03ec8792e62ef0dbb6b0aae09ad6cf245c5ca5ce9c","signature":"86499591859f5bfdebd568b51e5dfb938fa0e12987333ee772c67c8d7c39ad0277fa7ca4eacba714c43929c91c131ae0"}],
    [812, {"round":812,"randomness":"24d84cb3c4a942b468a23a97e71baf0b35376e99c49e2398931e90c5223d0249","signature":"b77c1d5a42ef1737a7987ea66a235d4749e69ca2cbe89eacc647762655857ea73a3b37bcc71daa72aae6540345658f4d"}],
    [813, {"round":813,"randomness":"db0343cc6448b7e651ea8f9322addc50065157a28521ef433c3150699136a62e","signature":"af2cddf834ba1d22387713bd65c92b7e9b8c7012eec1497dbd204e6d05fa1f91a6491b4df489242c8ae4d1b9f727d964"}],
    [814, {"round":814,"randomness":"0f66c957c0b84668222bab82ec98d48a6d97c94ee3dc1f0d95cd19fc1c319679","signature":"aaaf530ca17153f45ff8a31068d2c0a02f5beff37ef08461771e08994a8d3a1616238f01399d0d85a8fe36472cfc384c"}],
    [815, {"round":815,"randomness":"53bedfa3f316bd1230e8d584033cb82e184c1cb03f0146ab87c8914326734c5a","signature":"86499b22c6bd54e71f4f681dd6eef948b24a467ce4e80353a5c9ec2cb7456864b2bb857f500ba93a4f3e3cb303d8100e"}],
    [816, {"round":816,"randomness":"071502377667f7265a36de24468d625f196ad50f0d9812348ff6f1219dcaf7fe","signature":"8ef689647f624fdd9944d91987183cf40d4a63c77da1da9adbd23a7e572491f0927bf59cfe52a59ea6d263a8e72af2e5"}],
    [817, {"round":817,"randomness":"a0b5477dc168c567f4e81de10f2389ed294b67e1a22b553fb7e7394c1954cf0f","signature":"8591a552bf23d000bf0aa464777809f4ea8d3d6f0a803725e16c22f1da358fa3ad29e82b23e1086f27e8974e71f8dd5b"}],
    [818, {"round":818,"randomness":"84b1f1f969b030202fdaaae706839c08b2eeec67fc1cf9042b3a8624664f3485","signature":"b3e0874d8010ac93e42f4719cb1a029d39a4b9f14dc062ee065fa05f6cd1aacaf61d6bd8ace11813d44855d831c86bb3"}],
    [819, {"round":819,"randomness":"f6252e9b826fa620b77bc9e1ca4ad214968f4309120adf3eda6f44173e7ea0e1","signature":"941761408c3adbc7c6a4a9c6a237125716761c292233720bec86606926ca3b978ffceae016e5850a575503b24ff6453a"}],
    [820, {"round":820,"randomness":"4c9b36d4a6eff819ed35052a277f9ea489ed71246edcb283a44507e1e26dfc08","signature":"8437d1a393441c666df19d3e72f1b656f465d334cf41788b03e104058d324f69b89fa8494e962e79aea83864c7015d15"}],
    [821, {"round":821,"randomness":"a316318d58839c09057a06064b36fd176f03575f9be854f35ea91b1419c11182","signature":"b83287095f53cbdeccea3658740160d7279c28c2aca674ee4c29e5d0bbfe7197aa7940ae97a495cdf7fc5f4a5a98aff2"}],
    [822, {"round":822,"randomness":"66f0da6d83adb2fcb5c5383c611bb081958af0cef820484c7e3c10cb5a01ff66","signature":"9202f0b4f64343da694ffbbdd4054e2616b03d49bdca7930c90e8aa071dc2896cc1ee8f6dd0e338ce302e711dfbf0dc4"}],
    [823, {"round":823,"randomness":"9ddbea770b08ca309ac39dc86a51abbfe67bc579d5be91618cc1ab6ab7178681","signature":"b9922268163660fd20247939a8ee6634de826e9a9b935d17e2053e3e97786c46caf11215dc22d1fc5671d1a53fbcbc2d"}],
    [824, {"round":824,"randomness":"5e980e95ab1e8912830a03fb747f7fb29463c50dffda39dada5ad3cadb2bf828","signature":"a85b4f764a2864cae1c6083867b809d9a8cd2bb66b910eb121b4bc8a99177dde04c962bfb898210301109a1ca686196b"}],
    [825, {"round":825,"randomness":"fd4acd3623da1daf9de08e005b4a9f8dbcc4c8684d7e89ed86c6250e7b6a1fc8","signature":"96c89d01f5ba3e7043349ee43670c5b6c36922bb82ad155178977178fb7a4fa692a48d25cd11e8155b465595e97cdd95"}],
    [826, {"round":826,"randomness":"5297d6b6a767f3671df4e50e6b7140ff2735a9936b1907b6ad6c533f3531d805","signature":"950a3054b8e490427397eb74574abc207de9784eac4e4110f0f17263fcda54b1052a8a27f36e5e9c83df906d085513b8"}],
    [827, {"round":827,"randomness":"e73ab37e92f4c87edbbb5baa0e106baaae188ac1637ece36b44ce20d84ee9eb2","signature":"a155be6d576dac77d6cf25a897ac85d8e1683c921b7355ddbf5ae363934b8b49779b1b68f3d07e504fb711930ad96727"}],
    [828, {"round":828,"randomness":"0ec41251f6b09b08b5b0827a6d5353ac1e3ef23ee731572efe7a1ffa53564b60","signature":"9033abc3ef7a06e8a834aab1a35d0bcc4a4e27bb91fb00c2dffecaeed4995258e12e62d392088c9ca9a2c80f09d1a133"}],
    [829, {"round":829,"randomness":"2b9a7b26e919320eb0a3e1a517b5c8839eba5535497d3d448a90dab40dcf84e1","signature":"ab06a7c322f39266fdfca0a7b6749d8359fc2c7db5ea0e51b51cd3af3235263c7a7ad0c92b22bebfe27f9dd269d410ae"}],
    [830, {"round":830,"randomness":"1c09cc0ea685da9b080264f6dcee983b09b3c1b0c1fbbf792b6f9beeac5d1b0b","signature":"a64c47e9f3a2c16ed5ef7b64347545d6c6888cbc62e1870289d87461d0d9a7d247fdc9131b4d794853bdc215a2c1c8eb"}],
    [831, {"round":831,"randomness":"d9ad7111e028639fda54853a36b3c0464e1c07e00422caa53798e1ed65930500","signature":"8db8665bf14029e3c8d32c895c19e0ad9ec32ef8c4ab9111d35a422259ee00c72a99641da0222ecc044528e4cbf65c99"}],
    [832, {"round":832,"randomness":"014a398ceac7310a4c20324826d6d55aaebbf59a59e86e07202bf9a9d5d8eb82","signature":"990bb217a8a098f43beea30e829da238831d264c256e12676caa4b2312c438c90a7dcf20a3400c6025d31f13a2889933"}],
    [833, {"round":833,"randomness":"cccfacfb0a3916baaeb8732db32479787ec16c39adb04e66235277d8cc2eb02a","signature":"934493eb8f3ec98bc7cd800df1fded0b432c7e6dbc2c15e0b5c5162529324eebdc7fb34923a99b6e9c5ff64e39eccc1c"}],
    [834, {"round":834,"randomness":"c7f8c37693216e733f6acad44b36e8fea8f7546dc44ef530bf3a9abd32a0094b","signature":"99fc97d0bcf6208fe517b3afc0e3084b9268785c9f510c49138943d8a6e971f23550f0eec253f275e75a071a7cdd5d15"}],
    [835, {"round":835,"randomness":"15c4654817b0e4078bff06fab8330ca0a813d8052ef0d057445b044a658b08a9","signature":"8e25e8cea929377ed7332a9ab5334e98a4d75c10bdb18ba6b12a7b86476901133d82ee926dff1d713922dfdd90c96000"}],
    [836, {"round":836,"randomness":"0ba73368f1286d4b661e1c39080722d3ec3641ecc2ae95c0010b277aa1f9c721","signature":"a16b93f5eb63bfa7d57663d82d51a9de4689a5652e24c2482bea0ecaa7c3bd7c2fd1b98c319ae46364e5bad21477527e"}],
    [837, {"round":837,"randomness":"1302b00b751e3bfaac2ea347b321de6b21ebd6223a468e0cf1ae9fd659ecf20b","signature":"a8b45d0e456fc5071940bf4d883ce2d769c351c56eff0caf3e9b9e941e60c6d40b0059d3b634939e3e064d01313e501f"}],
    [838, {"round":838,"randomness":"f3177a45e6a2359e8b8e87815f425510390a9bf3b1f14359f63544d654a4eeff","signature":"961a557b0ee3cebfb34b3d2f266150f1c22ba054b8144b831dc4349f0603d5fa97be0e8336d1c3aa9497abca6134c081"}],
    [839, {"round":839,"randomness":"f1783ccfd8babc1eabb493336014bc6e401aca7f23d822d68cce4538c8c625e9","signature":"857023a4bcd0ff5805c4a9cd92fd31b6ef28e858a58fd3da1743c57c9af4224fc4794968edd6f07144685ee5b6fa791f"}],
    [840, {"round":840,"randomness":"b14a7f1e5e66d32e60930c82b5ddd7981582824d6e810134e28941546231d106","signature":"ae40165b9be470899e18e128e8c583308807f13ed6652939caf0df15a2fe488cd636ca128a3bfe8d34fd4db283930de7"}],
    [841, {"round":841,"randomness":"460ce2c52b741f7bc94db906fad2401fc2def3e6fb26e2902f004c9c59e50ef8","signature":"82a6b31395c9d25fde9b68ac74f6585725f2228cfe1c0051852d692149b4412c46cac78b1a8d5cba766223c24854a230"}],
    [842, {"round":842,"randomness":"f2b2ad638a4efebbfb85fcfc20a8e1ee535e3256d9e4f680962ad65c1ab719bf","signature":"810f77ce65afcaa4e4cda4c92881b40a014989735afa2491ff5c2829f80527d74eba95183645ee6ad20fa8d7306541c4"}],
    [843, {"round":843,"randomness":"3d8fcb5214698889ed4ef00d3992bdc5585dd62701740244f4aa31a6c9494ca4","signature":"b1a83d8b0c405495a33dd22815043515bf87c8e90940890d0a050739718d750d8226d0063a2bc2e901819a882e8c56a2"}],
    [844, {"round":844,"randomness":"40b14487a7e1ca720c6054afb84a26ea6b4c5735b4438b957dcf09dd34b041ce","signature":"8a66626a3402209c31e9feb64bb026f17671655720f38c52f3ca5f8b676e3bea38e1dfa05df5a495ad2c12c1baa45bdd"}],
    [845, {"round":845,"randomness":"eafca834e5bbf98372eb474e155f400fd6a6654b1d5b2fbb559c9bcf5f2b8c56","signature":"a5f8854eb0fc3a8b74707d08dac588cc6cc0819191107fd4cf98feb704d637f575e3bb2568f4ae9acd055a50edc92527"}],
    [846, {"round":846,"randomness":"c99ca27bce6e6926959b9ba8e39f4cc2fb0ceee1d7d82a8bfdb9cfd6de19bb78","signature":"98201a05822f402f4ae78a7f776a1f7186efef42a65153a1765827451dc5705bc97831cbb852a025e2a39cc9e8fc35ee"}],
    [847, {"round":847,"randomness":"0efb161d02d324948121a2141e9d1973a05811c2d7ee38d62710f783f6c95b20","signature":"a1b03d321b5ba3e6534d93b71dcebca7bb80e3ef5eebc8d805d3cb9b7c99a2fe180f82942a51a910834d74a8708cb807"}],
    [848, {"round":848,"randomness":"7bcc7e394d54179c33286a848cd5f5d62564c4cf016378c2dc7ec877ede5f39c","signature":"a8a51fdb2fb6b58d75a6131cb91300e1e0fe326292946bb1ed1269898b6b2ced2ebc86b1ab6039902423a4f2e93c18b9"}],
    [849, {"round":849,"randomness":"219f28edc1abd42810e2779fdc373ce44004d4d392bd0f705cfb65ce90cd9435","signature":"88899d7f7bdd8cdd4216594694b4b540dd317dde1edef4d8590be790582664777597e1e58fc312131dc5f08daf38e6a0"}],
    [850, {"round":850,"randomness":"139ddb77e6cde9f262fd1b3c55ca72025957f4f5d1649d16068a8b155578a95f","signature":"aa8cbd5312feed1d77c35c3023d18f4cdb731677e22af4d729b311f1a28f0ed5883f17965a4bbe16f8cebb04470f747b"}],
    [851, {"round":851,"randomness":"9337241666edf539bed813edb67fc410383a08996485bbe3c48760f5836ab970","signature":"a13b764b77fe0610c751ea22e0f68bd6b4b40734c30c5c33bc5727380d274d601626472fdfc9c46a895fc00676b3cab0"}],
    [852, {"round":852,"randomness":"2cb88403c75809cf9ca63a50cb5d53bde821a85976b4021ca7f5cf2c4075dc9a","signature":"8d81cf8f3fbb6f92794ba9b479a3e9107eb03740094eebdc17bca2c795299d9f55adf614308fd9c4a14fe2d15fe0559b"}],
    [853, {"round":853,"randomness":"cebb873f73577150bd17126f5dc3b1e783b9519ec423606712ea9b53c0d196e9","signature":"8575b5e69396040b91c0c7053495c38ae9d70b5f1f820ee465d1bf3be4e1fb22a8f2dccbc1b2ee084d2b621402bff510"}],
    [854, {"round":854,"randomness":"fd8d62f5bcbadba011ee242272ba2d24d44ba8f3279d73ae9a9bd7b1f1caaa42","signature":"b8b7b5c0dbe937aaf976180c4ceb2b5d4c206fdc8af424abb7351dba31346a5efa57e10e7da0018bb3b36374f89a6fd9"}],
    [855, {"round":855,"randomness":"a8fa5c85ac017791661e54c3dd9bf54833229dd9f38696b9f0e25bd142ae9b4f","signature":"92f94941d508aeaa906a5e6a54c17a82f1539b694e36b43a4a92c2e4cbd1a798b4219397e2d0eb80e3a5dcf178d8f536"}],
    [856, {"round":856,"randomness":"24e4e268db79982329964036a891e0fe4c7f9a9fc010dd829e6207e3e79712db","signature":"a92b417d324a7f327f72e670b5542cbf9f5d15c989f673daf3c3ce412d454110616bd13305dd712ebe66d9cc06b32812"}],
    [857, {"round":857,"randomness":"03652dedd6a2b6eada2266433b15b1a4b9823ff3683530a7be54dd72153afc64","signature":"91271ebe70cc7c14ba0d6b157f1300cae5489c4ceb3fae68e2357f121c1ff69346d543263777c17e57631f5e1d020ef5"}],
    [858, {"round":858,"randomness":"dadc04fb09959d9952a0c582c05b84da331af74f42cb32f09427017c952a72ad","signature":"a59c00ba38d94bf83655256c3ef7f5a593e3a89a0c3547489e69e4625ee77498dc7e0720eeab4908b646e68369fd371a"}],
    [859, {"round":859,"randomness":"28cdd4eac6e6d86b0a3a6dbae0ff5095757904e85c99cbe50e32e613ccbe1c49","signature":"a2abb1133f9035d22586379687063bf341c62e6eb057b05bacfb9f7b94815f1ca9b825fd8fd577ac06c07f84b61f480f"}],
    [860, {"round":860,"randomness":"0aa978bf2023173e736510c41088d0150f61a38a3f4f0254b1790a28da09424e","signature":"aa11d7f1b09ffa0fc0b58589738ada11a0bc6f7f44997773f2fd891d12baf154c90c30fdd3e3b8acfb30d0f96808c364"}],
    [861, {"round":861,"randomness":"b5369d52561bdd5961dca501a853e384d081e71d507530455b6b4a9857227b34","signature":"b3bc55e5a962483e540c576ad15fc1202af6d39219836cc7395ff4958d8dbbcd7a741498a316e557e2bfa2f1064db251"}],
    [862, {"round":862,"randomness":"1d588b42b9ce414b20e48513e83d9669033ca8d7a7a1ab0ad68483b5284bddb8","signature":"a8da3f0ae38a1dc2175b2cfe8aa44cd3544024d08c599a544a05b0b6dab781d093908296a7d9ba6bee8b19faac1c8b15"}],
    [863, {"round":863,"randomness":"c8b46a919d54192ee5fdb28f3394fca47bc6a2407fd4ea00e28d0cbdf0aab7fe","signature":"b98d7474e03c1d611519fcbc60f32122cdfde74ab720999cc96cd86f1d1a34a0bc43d376164fb5951ee06e072710aa06"}],
    [864, {"round":864,"randomness":"be970715f31666b9331ce026ac01d49b42a44eb0ffc0aaffe1861bc28faccc00","signature":"90a167c64b56dcf47ee2b89bb547c27019ed12c06ee7f8e7562b42ab934eb770ad2b8f14b647aeb87576c3e97413aa8c"}],
    [865, {"round":865,"randomness":"d0b7767ef301ddffd3b4f99fd9483a51bc3dca03d2046b6f03430330ba08190e","signature":"aa2bf37f3891e19a0e6960f53e33be71619bfdff41fca375216d157113924c31943d30d5ec3d2b22b4ecbe2a661dcb7f"}],
    [866, {"round":866,"randomness":"ca668c7d4606582b2437df3f4bd4737800f6d6a84f739c0d9580ee56e2d1c236","signature":"829fd50efada52337ea1fa4aba20bd2d136e888488ecccde5e6382a5eae4f62bdafe0f33489c8cdf36b4206e23752cbe"}],
    [867, {"round":867,"randomness":"7b25f466797113ae39fa6fad1496ff3bd71fa1dd85972a6364f686a48d8303f6","signature":"adef5e0053b137511589e2d9ac62a22c6c7a4047fd19df804fc45cd9b090d0db3187da4e0bee056a7fa7f832dd5fc439"}],
    [868, {"round":868,"randomness":"2052471f1dc5c5d8c36a4cbdd2f3c5892b1cb4e3e8d645d7cf0b0e35bf6e551e","signature":"82959d7369c6b8e325a79df220eed67c3b2c455bcd0b3fab300a2f4674414c14b3e9df831bcf42d571a8e6e6aad3f8e7"}],
    [869, {"round":869,"randomness":"b7298aa21844876bf614d2f4baf8217ea9d939ae3e26032242ae7df5a3e06d75","signature":"98ca24585e605e6fc9dfbd006f4cfc997a3a762f77a00c65deedb1ee8bb887b19f59ce00fc42046bfa33fdade167c293"}],
    [870, {"round":870,"randomness":"fac6381edf2617cbeba29aed40b944242e4eb880287ec3dfef57e09aa7161943","signature":"889a6617d0818dd922695b6b93e19a2afc1d9f2c491082288c2eead183f4be7df0a7a55b66c2c5ce748223ceeb701db8"}],
    [871, {"round":871,"randomness":"70944d2cae4b3cfa78be78e64212117c0424c53144302c9cf67890b782877ed2","signature":"aa1d157aa382fc8280ba1cbd8406ce5bc5c64cd707a59ac6ffbc07aa874a984e51dea0e2fb1138e93992860b7c8fb82c"}],
    [872, {"round":872,"randomness":"4e89f40bf71b5045dffb8f82ca51b0e39c67ccb4c68e37f302788d332307fc0d","signature":"a4f0eea79d7b00404034a38a343f81839bea69d048f4c8780811af1623a4a2f5ffb5209c68b0716b2393c54977c22e4e"}],
    [873, {"round":873,"randomness":"47f9bf99c0282ef351b744abc3c007db895c7b5d8e94285daa2ab1ef12c3788f","signature":"859343335aea1140ed4d1b5e6cfa83cba8349a8a0969507a462a67b8aaba1efe1a8c6b2baaa433a097b74d470018aa07"}],
    [874, {"round":874,"randomness":"e616729f6da65e8aa69ec37e569abcad2b05cb9ffed6de99d14c6c5024befb27","signature":"83c7f029fd2f58b72cd450849efe15e5ff8b75251d08e192b3849e51d7a1ba36c94ec8f4ed9244e6c296d86111587302"}],
    [875, {"round":875,"randomness":"fe5690038ea8cbee069130a86076af06fb1cfb07b4eea8797fc7c0d8a0da81e3","signature":"8ed505c9578771871a6dd9ce7109ed0f1573e9759d68147ba8950ca5ab977de44e616d731b17716de6cc6b93c938fb2e"}],
    [876, {"round":876,"randomness":"4667ae1efcf521f8cd51d394b4b179bc7dacdb5a27ddb9a2d7742e128cec1129","signature":"a8073a6fdac5289172981075e573d313d19d1ddf6b80b74be572b064d8c17b449135013ec1f385a0989d27bc0caeeb46"}],
    [877, {"round":877,"randomness":"c2b428b997f201073e65346666375ecb8331b075a014429275bf7c3f0a8b74d0","signature":"8863e40fb79b9f4612c0e547619219ccedb217334951d24075b60cfec950336424eb298857899c2a2dec2e9ae9fab56d"}],
    [878, {"round":878,"randomness":"5786de1f9713a95b3b7e23551b889dfc8019b77226e5603b960e1f5369a6fbe2","signature":"83ecb167c4ac49c8f26a39d37dc8444b228c2973ac038a55abefc79c7f3f70c4e28f651669ba1e7b69339d516693310d"}],
    [879, {"round":879,"randomness":"a13f9fed75a838def4ae435be870650a7a7a7ace247ee84e5f7c053103994fb0","signature":"85e152e450fae65d0be48a8a785e0ecac607546c9492ba0aad903f37ccbbb6e7c0325cdafda5b16de6673c31960c83cd"}],
    [880, {"round":880,"randomness":"80a4d505b6cef02bd59b4bc1ccc36dc5d59762d07f3c99c5968ecffbef987cc8","signature":"91cd984596c94113d995fa6c13f0a1bf577a839023d4a64d2d299a97f9a9eedabd93e91c8b81b2148b39d7eefd224270"}],
    [881, {"round":881,"randomness":"ae9ee8df61d149888db6ca4b3dde82b0da1b3f0e46fabeb99fc984925338c122","signature":"b8a42a17370aaabeee01e9b04af7af8513413f5e291331be1061ee867456fd0e8416c18e13447a4547317b1d843f8cd1"}],
    [882, {"round":882,"randomness":"901be3f7764d4102e46308cdb46a47cc551585a6fbfc816989e322533f638cd4","signature":"91b7a5d6ba60329f9a4a7ed48f0c31fa32138339e843f1246ae6a5dc5e46ac23a9bcf13b260cf40125d416c1b1b2c108"}],
    [883, {"round":883,"randomness":"d3979922ba1670a72829c05810ec40b859324c75d5c5799d6b2ffecbfc3b887e","signature":"8071eb2cb3644aa9a82b1897030e3f190d0ec224bffca114650709270addd3c3dbddf49a8d2fe212e642a9f59345f20a"}],
    [884, {"round":884,"randomness":"8e331158a1e918ebffd5499364867343add6cfcd4398a32f14a7f5e395aedff5","signature":"86c095cfa4c4538569d67a74317bb7020ca49935a6a38b378a93ac0ab610f46c088a66b9ce3df39a9ecd73459ead30c7"}],
    [885, {"round":885,"randomness":"42fb073e8970f610038269068a3d146092b6406692aae3b457ff4fb51566f793","signature":"82ab8099804f287cbf6d1536f98326788e52a03e1591c97fd9c4594ec59074c2c66902c23f8cf165abb60cf6779d6824"}],
    [886, {"round":886,"randomness":"7f0fcda9f6376439a9e1c4ae6b45c642fa312da7e8eff76a71686c85c1c11d06","signature":"8ceb187f14883378e156e6b7fc39c1b88847040a9fef94bfbf7d081e3b161bff250c3c7ab8b46c5ce6a322ae45b7d75e"}],
    [887, {"round":887,"randomness":"01b9b91979734caa9885393a5d8495119e8de1edc672b85d8102c6675a5d6806","signature":"b400d47367b701f05a61a12e1fe22df9eece12df05c64da3653eb178f19782dc430fa7f6a9a8f4f1d5220b92f2ade416"}],
    [888, {"round":888,"randomness":"2638a6acaa970799c27d575031928fdc2d468bc10df87099729980ab0c0cf40a","signature":"abfefe6ca3f09248d4ddbb900543dac4f49886b88b3924085cc57d78b29dc638c480b0eb8a105894fc3139141577a2be"}],
    [889, {"round":889,"randomness":"710cc65d10097c10264aa74d196fd3661d4b8ff5a7407388b5fdd14892ba315d","signature":"a8b9d25bd7c58414de008796cfdeb79870261005e83dedc60b50cc6b04e919e3dad088da59d4f190c84037bad8e5ffa5"}],
    [890, {"round":890,"randomness":"9a55f502407883a076599118ec2873174b91c74a9b634f5d212fbf69c42521b9","signature":"a9c69e6fe9c601b7fc183c870e37ccee58c1a358229a3d6973939015d9006dd8d614e6dced6dacbba8718dcc36727196"}],
    [891, {"round":891,"randomness":"0f7c0ab1dd3d2b9f59d2214ec5b70653cd052e6b8fd7959b6036ea9877349e68","signature":"b7b718112b6876d3aed3424965af1167eb8e6f2c28635325b8b387ef9cde17e9855301668809bde8fc34d901a237ce38"}],
    [892, {"round":892,"randomness":"906517f7df537626f3a5e0faa32b345d6f1affdd608e1a489b2ff36a7dd29a30","signature":"8cf5e537ac986ea83d7affe87b52a6c13083bd3c581992034c8072cb486adaf7f18f0dda59fe7f0bcc4508513230fa27"}],
    [893, {"round":893,"randomness":"97232b1a25287625dae4331bf1e74b79662258120906307b715090d5cd70f47e","signature":"afa053baf3cda395efe48f5fdd135f1041313c5a80b748410e29f2fe13794cbf6cfc897c6fea020f30403cf4de47d1ee"}],
    [894, {"round":894,"randomness":"c54ada1b837c27e81f16bff0b8e3760ee3e316a56b659c92a1897d38ebb209c7","signature":"ae38c3e9ca300f7b7de531e79f421ea148d1c8efc3fc86dc738558fab1aaed298f976b3f7b013c00060d5d8c7c96861a"}],
    [895, {"round":895,"randomness":"a3c65b7283b1665da69f8673324a19e3ff0043de30c85d3e70b2e4cd305b267d","signature":"a271be2a0eb56b42e18c1edca6a66613b963d5412290af19d8648bf053270ad7ecd5dec1817f3ac5721456b8af448596"}],
    [896, {"round":896,"randomness":"52b5df39bea4430cc1ff26c3dfa0bbc116a456a68aa504d151e8f5615582477c","signature":"86164e526a744f8a942219f2bffe98b866be062fbdc164d2727442f0b7178367b1b18ece2b960dae2655f68363629d69"}],
    [897, {"round":897,"randomness":"409b4e0716a1709bc2e2e3bf179cbc733ec3de618e1da423bc6bac7900c7313f","signature":"b0bf098afce260fb82982df2cfbb3b7bac55a9983a7c60577e143ffc61dea73f1e51fa0e7eff7e1051ff0f12cbe657f1"}],
    [898, {"round":898,"randomness":"ef451c04ca7592c12045f5438d0cf72a074ee24c7673e46c2b240b94fae9f117","signature":"911437015b7011f8190d79e2c531c59bb3ed2bb4c82ff69bb19deec55f7d8bba3d57442aa8ae60f264aaab8ca2f75c78"}],
    [899, {"round":899,"randomness":"7a48216916a90548e0b06a88bb31903f04667404400a4a6c1a2aa0fa19aed4b1","signature":"a09e3a7c344fb9777b72bb617f7f37a4aab226ff9ff944fb933f8a317e464e179b16358d75ac5b0959ecc4f2d7aa3bc2"}],
    [900, {"round":900,"randomness":"1f5aff1110bb1430e52251231bf8f72dab894f79b9684e468346907370fa0c09","signature":"b4e95f436a6a94a9766923168de9e6120d6e407a9858fbe94e42d67b170d77014d83fd40931a1735127e25c78db14483"}],
  ],

  // Publish times (https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=4d0d6d84cdb9b48260594e3b74aa86ae)
  // Publish time of #800: 1692805764000000000
  // Publish time of #801: 1692805767000000000
  // Publish time of #802: 1692805770000000000
  // Publish time of #803: 1692805773000000000
  // Publish time of #804: 1692805776000000000
  // Publish time of #805: 1692805779000000000
  // Publish time of #806: 1692805782000000000
  // Publish time of #807: 1692805785000000000
  // Publish time of #808: 1692805788000000000
  // Publish time of #809: 1692805791000000000
  // Publish time of #810: 1692805794000000000
  // Publish time of #811: 1692805797000000000
  // Publish time of #812: 1692805800000000000
  // Publish time of #813: 1692805803000000000
  // Publish time of #814: 1692805806000000000
  // Publish time of #815: 1692805809000000000
  // Publish time of #816: 1692805812000000000
  // Publish time of #817: 1692805815000000000
  // Publish time of #818: 1692805818000000000
  // Publish time of #819: 1692805821000000000
  // Publish time of #820: 1692805824000000000
  // Publish time of #821: 1692805827000000000
  // Publish time of #822: 1692805830000000000
  // Publish time of #823: 1692805833000000000
  // Publish time of #824: 1692805836000000000
  // Publish time of #825: 1692805839000000000
  // Publish time of #826: 1692805842000000000
  // Publish time of #827: 1692805845000000000
  // Publish time of #828: 1692805848000000000
  // Publish time of #829: 1692805851000000000
  // Publish time of #830: 1692805854000000000
  // Publish time of #831: 1692805857000000000
  // Publish time of #832: 1692805860000000000
  // Publish time of #833: 1692805863000000000
  // Publish time of #834: 1692805866000000000
  // Publish time of #835: 1692805869000000000
  // Publish time of #836: 1692805872000000000
  // Publish time of #837: 1692805875000000000
  // Publish time of #838: 1692805878000000000
  // Publish time of #839: 1692805881000000000
  // Publish time of #840: 1692805884000000000
  // Publish time of #841: 1692805887000000000
  // Publish time of #842: 1692805890000000000
  // Publish time of #843: 1692805893000000000
  // Publish time of #844: 1692805896000000000
  // Publish time of #845: 1692805899000000000
  // Publish time of #846: 1692805902000000000
  // Publish time of #847: 1692805905000000000
  // Publish time of #848: 1692805908000000000
  // Publish time of #849: 1692805911000000000
  // Publish time of #850: 1692805914000000000
  // Publish time of #851: 1692805917000000000
  // Publish time of #852: 1692805920000000000
  // Publish time of #853: 1692805923000000000
  // Publish time of #854: 1692805926000000000
  // Publish time of #855: 1692805929000000000
  // Publish time of #856: 1692805932000000000
  // Publish time of #857: 1692805935000000000
  // Publish time of #858: 1692805938000000000
  // Publish time of #859: 1692805941000000000
  // Publish time of #860: 1692805944000000000
  // Publish time of #861: 1692805947000000000
  // Publish time of #862: 1692805950000000000
  // Publish time of #863: 1692805953000000000
  // Publish time of #864: 1692805956000000000
  // Publish time of #865: 1692805959000000000
  // Publish time of #866: 1692805962000000000
  // Publish time of #867: 1692805965000000000
  // Publish time of #868: 1692805968000000000
  // Publish time of #869: 1692805971000000000
  // Publish time of #870: 1692805974000000000
  // Publish time of #871: 1692805977000000000
  // Publish time of #872: 1692805980000000000
  // Publish time of #873: 1692805983000000000
  // Publish time of #874: 1692805986000000000
  // Publish time of #875: 1692805989000000000
  // Publish time of #876: 1692805992000000000
  // Publish time of #877: 1692805995000000000
  // Publish time of #878: 1692805998000000000
  // Publish time of #879: 1692806001000000000
  // Publish time of #880: 1692806004000000000
  // Publish time of #881: 1692806007000000000
  // Publish time of #882: 1692806010000000000
  // Publish time of #883: 1692806013000000000
  // Publish time of #884: 1692806016000000000
  // Publish time of #885: 1692806019000000000
  // Publish time of #886: 1692806022000000000
  // Publish time of #887: 1692806025000000000
  // Publish time of #888: 1692806028000000000
  // Publish time of #889: 1692806031000000000
  // Publish time of #890: 1692806034000000000
  // Publish time of #891: 1692806037000000000
  // Publish time of #892: 1692806040000000000
  // Publish time of #893: 1692806043000000000
  // Publish time of #894: 1692806046000000000
  // Publish time of #895: 1692806049000000000
  // Publish time of #896: 1692806052000000000
  // Publish time of #897: 1692806055000000000
  // Publish time of #898: 1692806058000000000
  // Publish time of #899: 1692806061000000000
  // Publish time of #900: 1692806064000000000
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
