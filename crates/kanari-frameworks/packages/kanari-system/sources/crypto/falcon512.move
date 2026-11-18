// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// FIPS 206 FN-DSA-512 verification.
module kanari_system::falcon512 {

    const PUBLIC_KEY_LENGTH: u64 = 897;
    const MAX_SIGNATURE_LENGTH: u64 = 1024;

    /// Returns the public key length for Falcon-512.
    public fun public_key_length(): u64 {
        PUBLIC_KEY_LENGTH
    }

    /// Returns the maximum signature length for Falcon-512.
    public fun max_signature_length(): u64 {
        MAX_SIGNATURE_LENGTH
    }

    public native fun verify(
        signature: &vector<u8>,
        public_key: &vector<u8>,
        message: &vector<u8>
    ): bool;

    #[test]
    fun test_sizes() {
        assert!(public_key_length() == 897, 0);
        assert!(max_signature_length() == 1024, 1);
    }

    #[test]
    fun test_rejects_malformed_input() {
        assert!(!verify(&x"00", &x"00", &x"6b616e617269"), 0);
    }

    #[test]
    fun test_rejects_empty_input() {
        assert!(!verify(&x"", &x"", &x""), 0);
    }

    #[test]
    fun test_rejects_truncated_signature() {
        assert!(
            !verify(&x"000102", &x"00", &x"6b616e617269"),
            0
        );
    }

    #[allow(unused_function)]
    fun rust_vector_message(): vector<u8> {
        x"6b616e617269206d6f76652070716320706f73697469766520766563746f72"
    }

    #[allow(unused_function)]
    fun rust_vector_wrong_message(): vector<u8> {
        x"6b616e617269206d6f76652070716320706f73697469766520766563746f7200"
    }

    #[allow(unused_function)]
    fun rust_vector_public_key(): vector<u8> {
        x"09a8866e2a843901f74aadc412b17186ea72d4ded116022e506665a2be63d1885656a04eb316e473429e4641c86d011b3ea6f58019a2acbe4f2e9ba187e2484749e952938ce90dd4004618ea198c9545c2d10f6e1f0e140e25faba68a2a50ae55581ba8402f143bfec8e0ed511a026847f1b500c925ca1646aae32b595f4b917ec484a92629fbf0229f5b1727b8784849110b1984d4afb0b5150b9790d75c7056e9b7eeb5b8570265a1d1c0ddac0e0abc10dd533c1fa89fe897ed2c9564e2eed8e96c07c580c2bba12805233471ee754fe7e29f9ae8aeaa2e22c9700cfd7380e16388378388a5eb9bea689713f4197ce466b8b66950c46250fc55e4615cb0e97244dfd4ca29493288306a559119de892211456abbf81ea91b3b345cb2b9f14724060fd91e6bd2c71c955231ade571510fba0331e79e0a0c685916c644e04ad01b1045b8480a878901c6ac60f9f3841f855f48f450bc97ab219d11af14609b52a6141934e9dbd339fa044eae63b30021eda40e98b2e80a0abb4d49eed502ea9f0062bf752f9a091da2be1d74e16e1fd9a5c45082761321135a8fc12b0a10c2e97b56023720a5120a20aa6e08b84d39a0a1af687162f04352565adad096ef4071984a56ffee4cf9d12ed25188303688c9158feb1af8482c99cd4a909350e82698f71f9081bba184e5bfbfe1435ab3f55c08c2544d72a9132394b012715a346112112d463fb82bab8f44bdec22bfa0375c260e475bc1540184e48529697f15f90e0ee483f930e4b43cb2e85f52be656ec2a5daac1cf7989166c6ddf02c393de75a5426d05fd5ebfa7e6e3a808f01b7b238a834d4902e562b1b255f6510b16aaf2530559544c36434ac45d7deac2b96035271b1bb5206aca649a294880c1c114c1ec22f95ce682ea9f523356de8c0e908f6863ed82f10d898ac8ba3351c46f639e797532dfb8f1d7329256ca4cd208f595f468e1cb3f283a38a6b0b08db5b85c470c40fb0685111cf9d919bd894bd92b874cd6aa5989f0b1f552027f605bfa1546928c4b8bd3118458a86779e4bd1209f711d5e1bf3db9069497e45b6272c3c6e8cb63e525528d8bf6b0fd020d0e6532a14817ce63cb20dbee86b614d954b8fc1e9a87d64b34ccb1c21096b25ff905f981b5629596246e9c3ed7ce8518de5761d48038049b8ac6a00a0d8551c7d3e4aa640579eb4548ae56ba3c525f23b53c222164b807cf48430a2964a4cc489e84ab568a41f64f869c013572a04c73851e170dedac"
    }

    #[allow(unused_function)]
    fun rust_vector_signature(): vector<u8> {
        x"397f216aad26d4f7376a3855cccda4b22d80cc9330301ec38c101259c934e9b8d4098598d29606c35f956f5e4918efdb7f1af1bc38130e17b654ee7d7302645569940f094feb16b01d3e545c217fa6e701b4d4fd488716893eb612f9fb57bf8617889ef1e97a895c9864002dfa549431b3e0d97cbf38723f156f3b5346d00d5f8ed2290453243365b7874b55f1e0013e28a617312c1cc5a71a4208558173ff3551d7dab802cc53266695283b0bd06766cfb34ed072023a3ae4df00aafd3e36ecf1ac1424cc614801ad1b4b66993e259c4dd9225ee2642d22cd3b5298da6115248ca8c97e148af4ffd517534ce81cec862626f40f698245477c38a5f5b0936b92bd303d8009f90f4bfd47d0813e9826a1f465729774909b89b20a74c2cd646e0aa7f16b02d50e5fe7bb7375fd0db523928d8122b6c2929abe5077054276154f0b698b4f8076afb37b1ee3afadcef99488f0bec632cad3910ebece16b7152d5debf28307a97eb0f34215db29d7c103360f3cc2af878ff76973d4a746370bd0da960c135896a8d73bea17d139a8a6627a7b4515645e92ce93eb056238211ad3c856759c6f54b796dadaa6aef785567f0778e41569621ab1590025c20bb4403579ac7e9060c3a2e75d3376c9433e671709f33f9b09fe4ac39de7f94ef773d464e7ac63b8723ed910c7b67d9873eb4ff917795878ecb8faaf222690d5e411f6ab2e7edc428e48936b91f52a9151b12915999ada8327c5cd9cfa231b7d89d34a9144a146f06783b2a6fd224883e9df16d9704e846a230db9452fc6f44810c1d703193cb5783cd1e6987b413665bc7f7739572d3fad7cade1f842a69a91f64c8e3d56b963eca0dd1f3b47e31ae2bfa0af4a04f7541ae565a0d367db031451b16e2175bc6ce0473423923860bd4d9caa12fb4f0000000000000000000000"
    }

    #[test]
    fun test_verify_accepts_rust_vector() {
        let signature = rust_vector_signature();
        let public_key = rust_vector_public_key();
        let message = rust_vector_message();
        let wrong_message = rust_vector_wrong_message();
        assert!(verify(&signature, &public_key, &message), 0);
        assert!(
            !verify(&signature, &public_key, &wrong_message),
            1
        );
    }

}
