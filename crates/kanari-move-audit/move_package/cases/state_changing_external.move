module 0x1::M {

    entry fun foo(signer: signer, registry: &mut Registry) {
        0x2::External::mutate(&mut registry.state);
    }

}

