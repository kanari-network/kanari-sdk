/* Block comment header.
   Spans two lines. */
module test::block_comments {
    use test::other; /* inline block note */

    /* Leading block for the struct. */
    struct WithBlock has drop {
        value: u64,
    }
}
