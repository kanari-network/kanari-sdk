module 0x1::M {

    entry fun foo(signer: signer) {
        let addr = signer::address_of(&signer);
    }

}

