module test::method_style {
    use std::string::{Self, String};

    public fun greeting(name: &String): String {
        let base = string::utf8(b"hello, ");
        base.append(name);
        base
    }

    public fun total(prices: &vector<u64>): u64 {
        let sum = 0;
        let i = 0;
        while (i < prices.length()) {
            sum = sum + prices[i];
            i = i + 1;
        };
        sum
    }
}
