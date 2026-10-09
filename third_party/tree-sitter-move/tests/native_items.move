module test::native_items {
    native struct Handle has key, store;

    public native fun empty<Element>(): vector<Element>;

    native fun length<Element>(v: &vector<Element>): u64;

    public native fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
}
