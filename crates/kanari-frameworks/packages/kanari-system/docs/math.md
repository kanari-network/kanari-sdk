
<a name="0x2_math"></a>

# Module `0x2::math`



-  [Constants](#@Constants_0)
-  [Function `sqrt_u128`](#0x2_math_sqrt_u128)
-  [Function `sqrt_u64`](#0x2_math_sqrt_u64)
-  [Function `pow_u64`](#0x2_math_pow_u64)
-  [Function `mul_div_u128`](#0x2_math_mul_div_u128)
-  [Function `mul_div_u64`](#0x2_math_mul_div_u64)
-  [Function `min_u128`](#0x2_math_min_u128)
-  [Function `max_u128`](#0x2_math_max_u128)
-  [Function `min_u64`](#0x2_math_min_u64)
-  [Function `max_u64`](#0x2_math_max_u64)
-  [Function `mul_div_round_up_u64`](#0x2_math_mul_div_round_up_u64)
-  [Function `diff_u128`](#0x2_math_diff_u128)
-  [Function `divide_and_round_up`](#0x2_math_divide_and_round_up)
-  [Function `divide_and_round_up_u128`](#0x2_math_divide_and_round_up_u128)
-  [Function `diff_u64`](#0x2_math_diff_u64)


<pre><code></code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x2_math_E_DIVIDE_BY_ZERO"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>: u64 = 2;
</code></pre>



<a name="0x2_math_E_OVERFLOW"></a>



<pre><code><b>const</b> <a href="math.md#0x2_math_E_OVERFLOW">E_OVERFLOW</a>: u64 = 1;
</code></pre>



<a name="0x2_math_sqrt_u128"></a>

## Function `sqrt_u128`

Calculate the square root of a u128


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u128">sqrt_u128</a>(x: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u128">sqrt_u128</a>(x: u128): u128;
</code></pre>



</details>

<a name="0x2_math_sqrt_u64"></a>

## Function `sqrt_u64`

Calculate the square root of a u64


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u64">sqrt_u64</a>(x: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_sqrt_u64">sqrt_u64</a>(x: u64): u64;
</code></pre>



</details>

<a name="0x2_math_pow_u64"></a>

## Function `pow_u64`

Calculate power (base ^ exponent)


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_pow_u64">pow_u64</a>(base: u64, exponent: u8): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_pow_u64">pow_u64</a>(base: u64, exponent: u8): u64;
</code></pre>



</details>

<a name="0x2_math_mul_div_u128"></a>

## Function `mul_div_u128`

Safely calculate (x * y) / z for u128
Most commonly used in AMMs to prevent overflow issues during multiplication


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>(x: u128, y: u128, z: u128): u128;
</code></pre>



</details>

<a name="0x2_math_mul_div_u64"></a>

## Function `mul_div_u64`

Helper: Calculate (x * y) / z for u64
Converts to u128 for Native calculation to prevent Overflow, then converts back to u64


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u64">mul_div_u64</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_u64">mul_div_u64</a>(x: u64, y: u64, z: u64): u64 {
    <b>let</b> result = <a href="math.md#0x2_math_mul_div_u128">mul_div_u128</a>((x <b>as</b> u128), (y <b>as</b> u128), (z <b>as</b> u128));
    (result <b>as</b> u64)
}
</code></pre>



</details>

<a name="0x2_math_min_u128"></a>

## Function `min_u128`

Calculate the minimum value for u128


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u128">min_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u128">min_u128</a>(x: u128, y: u128): u128 {
    <b>if</b> (x &lt; y) { x } <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_max_u128"></a>

## Function `max_u128`

Calculate the maximum value for u128


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u128">max_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u128">max_u128</a>(x: u128, y: u128): u128 {
    <b>if</b> (x &gt; y) { x } <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_min_u64"></a>

## Function `min_u64`

Calculate the minimum value for u64 (Commonly used in DEXs when comparing Liquidity)


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u64">min_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_min_u64">min_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x &lt; y) { x } <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_max_u64"></a>

## Function `max_u64`

Calculate the maximum value for u64


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u64">max_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_max_u64">max_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x &gt; y) { x } <b>else</b> { y }
}
</code></pre>



</details>

<a name="0x2_math_mul_div_round_up_u64"></a>

## Function `mul_div_round_up_u64`

Multiply and divide with round up for u64
(Commonly used when calculating the Amount In a user must pay into the Pool to get the desired Amount Out)


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u64">mul_div_round_up_u64</a>(x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_mul_div_round_up_u64">mul_div_round_up_u64</a>(x: u64, y: u64, z: u64): u64 {
    <b>let</b> x_128 = (x <b>as</b> u128);
    <b>let</b> y_128 = (y <b>as</b> u128);
    <b>let</b> z_128 = (z <b>as</b> u128);
    <b>assert</b>!(z_128 &gt; 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);

    <b>let</b> prod = x_128 * y_128;
    <b>let</b> res = prod / z_128;

    // If there is a remainder, immediately round up by +1
    <b>if</b> (prod % z_128 == 0) {
        (res <b>as</b> u64)
    } <b>else</b> {
        ((res + 1) <b>as</b> u64)
    }
}
</code></pre>



</details>

<a name="0x2_math_diff_u128"></a>

## Function `diff_u128`

Calculate the absolute difference for u128 (Paired with u64)


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u128">diff_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u128">diff_u128</a>(x: u128, y: u128): u128 {
    <b>if</b> (x &gt; y) { x - y } <b>else</b> { y - x }
}
</code></pre>



</details>

<a name="0x2_math_divide_and_round_up"></a>

## Function `divide_and_round_up`

Divide and round up
Used when calculating the number of coins a user must pay into the Pool
to prevent the Pool from losing value due to fractional amounts (dust)


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up">divide_and_round_up</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up">divide_and_round_up</a>(x: u64, y: u64): u64 {
    <b>assert</b>!(y &gt; 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>if</b> (x == 0) {
        0
    } <b>else</b> {
        // Popular formula: (x - 1) / y + 1
        ((x - 1) / y) + 1
    }
}
</code></pre>



</details>

<a name="0x2_math_divide_and_round_up_u128"></a>

## Function `divide_and_round_up_u128`

Divide and round up for u128


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up_u128">divide_and_round_up_u128</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_divide_and_round_up_u128">divide_and_round_up_u128</a>(x: u128, y: u128): u128 {
    <b>assert</b>!(y &gt; 0, <a href="math.md#0x2_math_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>if</b> (x == 0) {
        0
    } <b>else</b> {
        ((x - 1) / y) + 1
    }
}
</code></pre>



</details>

<a name="0x2_math_diff_u64"></a>

## Function `diff_u64`

Calculate the absolute difference
Frequently used to check if price fluctuation (Slippage) exceeds a specified limit


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u64">diff_u64</a>(x: u64, y: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math.md#0x2_math_diff_u64">diff_u64</a>(x: u64, y: u64): u64 {
    <b>if</b> (x &gt; y) { x - y } <b>else</b> { y - x }
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
