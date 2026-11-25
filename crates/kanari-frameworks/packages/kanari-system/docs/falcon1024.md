
<a name="0x2_falcon1024"></a>

# Module `0x2::falcon1024`

FIPS 206 FN-DSA-1024 verification.


-  [Constants](#@Constants_0)
-  [Function `public_key_length`](#0x2_falcon1024_public_key_length)
-  [Function `max_signature_length`](#0x2_falcon1024_max_signature_length)
-  [Function `verify`](#0x2_falcon1024_verify)
-  [Function `rust_vector_message`](#0x2_falcon1024_rust_vector_message)
-  [Function `rust_vector_wrong_message`](#0x2_falcon1024_rust_vector_wrong_message)
-  [Function `rust_vector_public_key`](#0x2_falcon1024_rust_vector_public_key)
-  [Function `rust_vector_signature`](#0x2_falcon1024_rust_vector_signature)


<pre><code></code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_falcon1024_PUBLIC_KEY_LENGTH"></a>



<pre><code><b>const</b> <a href="falcon1024.md#0x2_falcon1024_PUBLIC_KEY_LENGTH">PUBLIC_KEY_LENGTH</a>: u64 = 1793;
</code></pre>



<a name="0x2_falcon1024_MAX_SIGNATURE_LENGTH"></a>



<pre><code><b>const</b> <a href="falcon1024.md#0x2_falcon1024_MAX_SIGNATURE_LENGTH">MAX_SIGNATURE_LENGTH</a>: u64 = 2048;
</code></pre>



<a name="0x2_falcon1024_public_key_length"></a>

## Function `public_key_length`

Returns the public key length for Falcon-1024.


<pre><code><b>public</b> <b>fun</b> <a href="falcon1024.md#0x2_falcon1024_public_key_length">public_key_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="falcon1024.md#0x2_falcon1024_public_key_length">public_key_length</a>(): u64 {
    <a href="falcon1024.md#0x2_falcon1024_PUBLIC_KEY_LENGTH">PUBLIC_KEY_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_falcon1024_max_signature_length"></a>

## Function `max_signature_length`

Returns the maximum signature length for Falcon-1024.


<pre><code><b>public</b> <b>fun</b> <a href="falcon1024.md#0x2_falcon1024_max_signature_length">max_signature_length</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="falcon1024.md#0x2_falcon1024_max_signature_length">max_signature_length</a>(): u64 {
    <a href="falcon1024.md#0x2_falcon1024_MAX_SIGNATURE_LENGTH">MAX_SIGNATURE_LENGTH</a>
}
</code></pre>



</details>

<a name="0x2_falcon1024_verify"></a>

## Function `verify`



<pre><code><b>public</b> <b>fun</b> <a href="falcon1024.md#0x2_falcon1024_verify">verify</a>(signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="falcon1024.md#0x2_falcon1024_verify">verify</a>(
    signature: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    public_key: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    message: &<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool;
</code></pre>



</details>

<a name="0x2_falcon1024_rust_vector_message"></a>

## Function `rust_vector_message`



<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_message">rust_vector_message</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_message">rust_vector_message</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"6b616e617269206d6f76652070716320706f73697469766520766563746f72"
}
</code></pre>



</details>

<a name="0x2_falcon1024_rust_vector_wrong_message"></a>

## Function `rust_vector_wrong_message`



<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_wrong_message">rust_vector_wrong_message</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_wrong_message">rust_vector_wrong_message</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"6b616e617269206d6f76652070716320706f73697469766520766563746f7200"
}
</code></pre>



</details>

<a name="0x2_falcon1024_rust_vector_public_key"></a>

## Function `rust_vector_public_key`



<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_public_key">rust_vector_public_key</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_public_key">rust_vector_public_key</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"0aec0e83740e584b4ce01600b06e8414c54240fbc25441863dcb04acaec5077b8b98d9040765d37837804cd7487721264c354858f3d75dd808ad9016f8ca782899d4fed1dec0a0bc2cfb8125a559784a19656f3a8e945a6a43f7a9bfcca2ee68b604535bcdb1aade5c064ac595a0b5202693a989b13b958d28140e16f4f11d2a8ae0f1d1c59ec04b2ce26ba27e5e8cd8ea10bda6996ce7a29646a125c4bf5813c22b6ce9378632a65bc26457f24eba2efc834550914d05051ab5f15e95b15456eb50106255c027a5ba5a4c92175a5de6e56a02e55ffb85e6a42c74e8635302ce4bdad454c6139654422d58c84ee09ed2987fc9b39d07ca6eb3e320d2ae476440db148e858cc70ba636e4ba9c61487a279c6276e71988080d083a0421d7baa14f7cd019b5ba59a2ee85dac836de2fbe5e537adc16435064c7dbe5184c654b00a6aca40770c79d63d19e1f90e92f363ad55e4699d034805d6ebac1b868f04460da6a09123416b6ac62874268d87a024970682311be989eb8ebb5bd40c08d32b29e726fb2a5d7a57632183fd8103b989e3e3c8b6415ef7674e3d809f9f1e2256d99540906ea5ff3521e8b61c83c35680b01051c2109e2b2d704d048b24b6cf252a2a883c55e1bc6fdb675d92315f7892089062029a96690b68fbb363228932014fd40dc5667910e862923c69e75ab7317cc12668b64d4f5efd13a5869769a2ce4342dd02b4188dc72ce9dae7bda42aa2396b7a303c1a0d225c8f7ae1d70055e30e187d5a7a0a363533eedb70c4ad3c0b59a93208f7575ca72284a1593e6fa35232be1de74f92252ea4b8a68527882425d71820719b8112fc269ca513e2289808808393b77e3ee86e9ce62a286006510f652f09800c4ff2097ee124684e79a456a25df8afc167db954bfe1a1f2a2ec9200387e2d9c02ca47db233e5cb01c746bf87a7ffdad6852eb1c16d0892c368571653f0db2676af87834412b9bc1449758abcccba101bcf789c7c19a756fc6abe2a8dea8b657188190893060fd1ee5012b3e923f1d2fa1c3114fd9423bf84c1c9f2fd2f435ae9042771ffcf57ab5495d69581ac01d3324ee76ca301a15a1d79788295c8eab2aac3828ae504e298a07f96e030c930bba322c9105a51608c1546b1760b4809ca1ac064027308800af09a1a069c001316adde64276ec63c3813db728ce1c84d290bbc546018a671840b0de4936c7ad1aea998c548f0ca7de6a932d587b59e9684127181dc1bd2c25055e2989209a7f6c92e2281f90a7abe7d221d1391c625f6732607541a7ad7497926a760861612a835c5795634468e59994d466d3743ea46016ae95f3222c8ad91424a2744e46f10d079821fd256ee999388268645e67f0a4d8a1a34c638a2216c6554d085ff9e2cd5eb758039b508d6905c38016293923a229eaea141e925686af44d351541e278c915d6a183f092080983a632d5952ab3de56fe7e30bcb320c0a5f2860f1a47a3cae133ad89559ba750bedc597ea4e394ad5e39f413f656a0d7755a669b756be82459473b5623fcad0d95019a7ba96b3c2596b9d78749f781b61590010ea2d74f0e0f900be0c85da1a3035ddbcb55aec7e179a9405e3947f9bbd64458910e08b1252421ecf66398ef517f31025fb58445da42fd34050a05123022e106ca098fc84ac5137aa5e791756b69a4500d98ee549208aa81a2ac295ab192bf54fa976be8d60c76e5277a9bc8efb0298411f96745b08b3478f3846dddbba4368da4681e2f39ac21e05dadc9faa6615ab561c5eb4042dac6095b455e8aa07d18a4e41372513572c02da05089a66e704a292c029d7a86ffbc5458075936c342d526390a7ec823fd16e72cbea62aae957e81381091cf934f2eb1d4054420013e1a3a7e4187c5a5920028856a78f4b59647d0c9bd4a487e63fba131c154cc29f149350028b39ad49772b2378c7dcf08667ca0e7bfa1c2154653344cdac471bc78da7085a72af5a9c949efd9e7c8adc8553dd81dc48fdda9ffe40a7abad92c932b24d641af38e84bdd0acaaf0ceb86614808f00a8e6325e59e2db191daaabc4846ab7e00b6613c0918c78adf7a8cc60d82a6979e2207b69190a090a66470264ba2218233bb89067cd83616557bd49530d1e5325d92bd6ddccd1bc81630f9dc40724681f4d003a29bffab85d2a6ec9266996349fb64772ec0f26550a347e59910d1cd77691f213a185c331fd6e0df3406008b8b125532621471a7e6d59cd02971f0a0e3b92711595b93a31ac7c40e8ac96f9194ad6b905b08398c525e09a22b8c87614b5f2a92560ce96766e7cd8c73561895d5555289203ad8557ce99a6c86b7473ac65a2080d383548c8ff0ab360b83386ff443af51f1ee3d6872cc657de9791495a7db2acac8e4b6ba55b6fa251245b40b316d54ce265cc3f5f979d63e7de63728f09c00cd1aa35a1159868c573de91cb124cb00232889b45418e6a736beb032a06355a6ea007d4755a27ade4b0771d6448e07996e90c44e8994af8ceb412efe1e3c8345880"
}
</code></pre>



</details>

<a name="0x2_falcon1024_rust_vector_signature"></a>

## Function `rust_vector_signature`



<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_signature">rust_vector_signature</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="falcon1024.md#0x2_falcon1024_rust_vector_signature">rust_vector_signature</a>(): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"3a9d31fbba45f31d4bb9664960e3520a3d6416a1fdcb61c1254311c2d4066398ef519141fdbde060630f85158dbbe28c01a2155fc3ee3343b676715306bce314a350fa135fdc8c54d36c054411222893e82d6d0d97d6d5cb1e2071af79f7ba5068463aea4e7c273914b879b453a4b74b8ac6ae6189aad5266c59b6e38d3f05dd53fdabce194744be6eec065b4dd815c48b3d3b08456f9d1c14c1fba345575094dddd9bd86ac70eb57a9db2c83ccdc434b30cd9bfbfb2d6f1db7f6ed0c802d61eb6acef47b638204cfc1864170c3b4a0e8d4f757d5326c02d9ed337593144da18cf35ddd328a08743d34779572872d32feaa61bb44478af96191cb7c44b8ed223dec0da8bd49b386b9a0be1b6dcebbdfc80d8f4c4774224b8e4887fad1a244af101612ab7dfc4ccc3dde5bb21563c6e8c9937842636b8fce4b293865964844f8ccbe495bc0f1d354b5a49acd78299bcf91764e35f8f74f1669589effc4b62751925e8c07dd13cf0bb674475be28dd73c252fa75957ecffb821e2942fb5b5e757077d3199bfcf6441461bf6ba9e0ab1131892217ed263452ada50fff076e19b2f6d35d0b4b1823863fef0bbfea73cdbfbe4fada7be8fc9f4a19465b4964d13065fb23ee3420708c69f0cf7a6592cc4eb5b42487833bb3e66941adec1a415b9e4d4b8c8cd320f7c80587a2eb5e3c037b573d7c1d73e5c4c3c89f0916e79f9b02edb57eacc32fb98a0cfad1dad06b83735d4c7ccd9a63e7bdbebefb1cf0a71f3e5098f7ce50841949c5ea870911f6bbd504ababb596d366f94173cdefd43260d51afd1acea973b95e998ba45bfc68776562d968260f30ef17d0a6a53ef8ce35979be3b77da5cdc3fb6415b1ad731411eca5de1b923a249618a4df21ff5289e0236d4ef5d871cb6ca54ea78b8f36bcd702ad7ba1dd1c268452d2a22959295946df78170235dfe6be40c2f05538c44e7fada9941b9e90a955e39afbc784be2471c21a60ab4cddbe22d8ffb148e66503ab12bef49f6e54e3daacb11b9afca2732dea39b1a0566fbebe5f51ff69d1663abd4f7c27f4a7a5da63c27a35c59fd6f6a05be98e00d66df013bab7e09a911990929d546133491c3a7b961167695499a175ea71cd5a98026db7e31c7790504a4ad06e49ab56b50e7363165114b3c5d477c35ad8a99c670c76bab983741987e77e7756714b6c44c1d83731586ecffa1c9d0d47bab3257e41445a5d74144723892e08ba7810ea858d3c4c9fe3d5d1dbc88dd5ffc6ee258910a45a4f323e7a7e345447993f44e24f564082afae7e1cbc2bc42df2b1436020b7af28f1e0335f83201fa72574477c47ac19c386aab26e6f11a84e0149a58fcaa85ebeff7d65af9c60a9a98fb4915d685d4addd6091de728b39035f74ddb6b25e06622cd818558acf824a2bdc1ec22d2ce75ab889bf7a58297258433028a3022b991447560f8c3632e51a259c6ffd66dd815e13de349f4c1e2b24331201c8bd58e158ed5e5d1661ceb0b01041a72156ea1c4909b6e66306cb2c322c872abf022652db9d283385194fffb751f122ab08cc6fdf62815cb6d735880edcf0b3710901aafe52450bd4b4f1d355dc4194abbdf60ac1089686963f9c34477e24d038d979efe2be8e7ec94fddbf6f2670bf134382da9a9123c4775657d9e6fbd3da907d936429a4d782309aba525702ebda49a5b10ad816439403d3ad78153d5d2d92f2578d66944a97c5b2af6cfdcd054e3792247bda50ef0a9045faaa7fa61a112cef0a937d3c93c6771de875462137a8cb8d127d4a923f601db7030000000000"
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/kanari-network/kanari-sdk
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
