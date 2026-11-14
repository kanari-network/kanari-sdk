module 0x1::M {

    entry fun foo(signer: signer, r: &mut R) {
        r.x = 1;
        0x2::X::bar(&mut r.y);
    }

}

