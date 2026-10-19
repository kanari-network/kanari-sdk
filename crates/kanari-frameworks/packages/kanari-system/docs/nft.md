
<a name="0x2_nft"></a>

# Module `0x2::nft`

Minimal generic NFT, mirroring <code>sui::nft</code>.

Deliberately minimal: <code>id</code>, <code>name</code>, <code>description</code>, <code>image_url</code>, and
per-token <code>attributes</code>. No supply counter, no collection pointer, no royalty
field. That is not laziness — it is the same reason Sui's is minimal:

- **Supply** belongs to the issuer's own module, gated by a capability the
framework cannot tie to an arbitrary package's type (see
<code>kanari_system::package</code>). <code>kanari_system::collection</code> offers a shared
<code>NftCap</code> counter for those who want it.
- **Display metadata** belongs on <code>kanari_system::display::Display&lt;T&gt;</code>,
keyed by type, so a wallet can render any type without per-token fields.
Putting <code>name</code>/<code>image_url</code> on every token object here would be a second
source of truth that disagrees with the display.
- **Royalties** belong on a <code><a href="transfer_policy.md#0x2_transfer_policy">transfer_policy</a></code> rule, so the owner can change
them and the change applies to every instance at once.

**Attributes are per-token, and that is the one deliberate departure from
the display split.** <code>Display&lt;T&gt;</code> answers "what *kind* of thing is this" and
is shared by every token of the type. Attributes answer "what is *this
particular* one" — a level, a serial, a trait — and are exactly the data
that cannot be hoisted to the type. So they stay on the token while
name/description/image_url do not.

Stored as <code>VecMap&lt;String, String&gt;</code>, matching <code>NFT_STANDARDS.md</code>, not as the
parallel <code><a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;</code> / <code><a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;</code> pair the legacy <code>james::KariKid</code>
uses. A parallel pair has no structural link between the two sides: a caller
can pass 3 keys and 5 values, and every later read has to guess which
alignment it meant. A map cannot be built into that state at all.


-  [Resource `Nft`](#0x2_nft_Nft)
-  [Constants](#@Constants_0)
-  [Function `create`](#0x2_nft_create)
-  [Function `create_with_attributes`](#0x2_nft_create_with_attributes)
-  [Function `from_vec_map`](#0x2_nft_from_vec_map)
-  [Function `transfer`](#0x2_nft_transfer)
-  [Function `update`](#0x2_nft_update)
-  [Function `burn`](#0x2_nft_burn)
-  [Function `id`](#0x2_nft_id)
-  [Function `nft_address`](#0x2_nft_nft_address)
-  [Function `name`](#0x2_nft_name)
-  [Function `description`](#0x2_nft_description)
-  [Function `image_url`](#0x2_nft_image_url)
-  [Function `attribute`](#0x2_nft_attribute)
-  [Function `set_attribute`](#0x2_nft_set_attribute)
-  [Function `remove_attribute`](#0x2_nft_remove_attribute)
-  [Function `set_attributes`](#0x2_nft_set_attributes)
-  [Function `has_attribute`](#0x2_nft_has_attribute)
-  [Function `attribute_count`](#0x2_nft_attribute_count)
-  [Function `attribute_keys`](#0x2_nft_attribute_keys)
-  [Function `all_attributes`](#0x2_nft_all_attributes)


<pre><code><b>use</b> <a href="dependencies/move-stdlib/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="dependencies/move-stdlib/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="object.md#0x2_object">0x2::object</a>;
<b>use</b> <a href="transfer.md#0x2_transfer">0x2::transfer</a>;
<b>use</b> <a href="tx_context.md#0x2_tx_context">0x2::tx_context</a>;
<b>use</b> <a href="url.md#0x2_url">0x2::url</a>;
<b>use</b> <a href="vec_map.md#0x2_vec_map">0x2::vec_map</a>;
</code></pre>



<a name="0x2_nft_Nft"></a>

## Resource `Nft`

A single NFT. The <code>UID</code> is the token id; there is no separate integer.


<pre><code><b>struct</b> <a href="nft.md#0x2_nft_Nft">Nft</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="object.md#0x2_object_UID">object::UID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>image_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>attributes: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>
 Per-token traits, keyed by name. Insertion order is preserved so a
 renderer shows them in the order the issuer chose.
</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x2_nft_E_ATTRIBUTE_LENGTH_MISMATCH"></a>

<code>keys</code> and <code>values</code> were different lengths, so the attributes could not
be paired up.


<pre><code><b>const</b> <a href="nft.md#0x2_nft_E_ATTRIBUTE_LENGTH_MISMATCH">E_ATTRIBUTE_LENGTH_MISMATCH</a>: u64 = 0;
</code></pre>



<a name="0x2_nft_E_TOO_MANY_ATTRIBUTES"></a>

<code>attributes</code> holds more entries than <code><a href="nft.md#0x2_nft_MAX_ATTRIBUTES">MAX_ATTRIBUTES</a></code>.


<pre><code><b>const</b> <a href="nft.md#0x2_nft_E_TOO_MANY_ATTRIBUTES">E_TOO_MANY_ATTRIBUTES</a>: u64 = 1;
</code></pre>



<a name="0x2_nft_MAX_ATTRIBUTES"></a>

Upper bound on attribute count per token. Not a storage limit — it is a
bound on how much arbitrary text one token can carry, which matters
because anyone holding the object can rewrite its attributes and every
rewrite costs the chain.


<pre><code><b>const</b> <a href="nft.md#0x2_nft_MAX_ATTRIBUTES">MAX_ATTRIBUTES</a>: u64 = 64;
</code></pre>



<a name="0x2_nft_create"></a>

## Function `create`

Mint an NFT with no attributes.

Open, unlike a supply-capped mint. Wrapping a cap or counter around this
is the issuer's job — see the module docs for why the framework cannot.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_create">create</a>(name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, description: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, image_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="nft.md#0x2_nft_Nft">nft::Nft</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_create">create</a>(
    name: String,
    description: Option&lt;String&gt;,
    image_url: Option&lt;Url&gt;,
    ctx: &<b>mut</b> TxContext
): <a href="nft.md#0x2_nft_Nft">Nft</a> {
    <a href="nft.md#0x2_nft_create_with_attributes">create_with_attributes</a>(
        name,
        description,
        image_url,
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>(),
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>(),
        ctx
    )
}
</code></pre>



</details>

<a name="0x2_nft_create_with_attributes"></a>

## Function `create_with_attributes`

Mint an NFT carrying <code>attributes</code>, given as parallel key and value
vectors.

Parallel vectors are the input shape because that is what the legacy
mint entry points and the CLI already take, so callers do not have to
build a map themselves. They are converted immediately and never stored:
<code>from_keys_values</code> aborts <code><a href="nft.md#0x2_nft_E_ATTRIBUTE_LENGTH_MISMATCH">E_ATTRIBUTE_LENGTH_MISMATCH</a></code> on a length
mismatch, which is the whole reason the map is the storage form. For new
code that already holds a <code>VecMap</code>, use <code>from_vec_map</code> instead.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_create_with_attributes">create_with_attributes</a>(name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, description: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, image_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, attribute_keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, attribute_values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="nft.md#0x2_nft_Nft">nft::Nft</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_create_with_attributes">create_with_attributes</a>(
    name: String,
    description: Option&lt;String&gt;,
    image_url: Option&lt;Url&gt;,
    attribute_keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    attribute_values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    ctx: &<b>mut</b> TxContext
): <a href="nft.md#0x2_nft_Nft">Nft</a> {
    <b>let</b> attribute_keys = attribute_keys;
    <b>let</b> attribute_values = attribute_values;
    <b>assert</b>!(
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&attribute_keys) == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&attribute_values),
        <a href="nft.md#0x2_nft_E_ATTRIBUTE_LENGTH_MISMATCH">E_ATTRIBUTE_LENGTH_MISMATCH</a>
    );
    <b>assert</b>!(
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&attribute_keys) &lt;= <a href="nft.md#0x2_nft_MAX_ATTRIBUTES">MAX_ATTRIBUTES</a>,
        <a href="nft.md#0x2_nft_E_TOO_MANY_ATTRIBUTES">E_TOO_MANY_ATTRIBUTES</a>
    );
    <b>let</b> attributes = <a href="vec_map.md#0x2_vec_map_from_keys_values">vec_map::from_keys_values</a>(attribute_keys, attribute_values);
    <a href="nft.md#0x2_nft_Nft">Nft</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        name,
        description,
        image_url,
        attributes
    }
}
</code></pre>



</details>

<a name="0x2_nft_from_vec_map"></a>

## Function `from_vec_map`

Mint an NFT whose attributes are already a <code>VecMap</code>.

This is the form to prefer when the caller has a map in hand — it skips
the unpack/repack round trip and cannot hit a length mismatch because
there is nothing to mismatch.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_from_vec_map">from_vec_map</a>(name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, description: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, image_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;, attributes: <a href="vec_map.md#0x2_vec_map_VecMap">vec_map::VecMap</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, ctx: &<b>mut</b> <a href="tx_context.md#0x2_tx_context_TxContext">tx_context::TxContext</a>): <a href="nft.md#0x2_nft_Nft">nft::Nft</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_from_vec_map">from_vec_map</a>(
    name: String,
    description: Option&lt;String&gt;,
    image_url: Option&lt;Url&gt;,
    attributes: VecMap&lt;String, String&gt;,
    ctx: &<b>mut</b> TxContext
): <a href="nft.md#0x2_nft_Nft">Nft</a> {
    <b>assert</b>!(<a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&attributes) &lt;= <a href="nft.md#0x2_nft_MAX_ATTRIBUTES">MAX_ATTRIBUTES</a>, <a href="nft.md#0x2_nft_E_TOO_MANY_ATTRIBUTES">E_TOO_MANY_ATTRIBUTES</a>);
    <a href="nft.md#0x2_nft_Nft">Nft</a> {
        id: <a href="object.md#0x2_object_new">object::new</a>(ctx),
        name,
        description,
        image_url,
        attributes
    }
}
</code></pre>



</details>

<a name="0x2_nft_transfer"></a>

## Function `transfer`

Transfer ownership. This is the whole ERC-721 <code>transferFrom</code> analogue:
moving the object *is* the transfer, so there is no separate approval
surface to get wrong.


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(<a href="nft.md#0x2_nft">nft</a>: <a href="nft.md#0x2_nft_Nft">nft::Nft</a>, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transfer.md#0x2_transfer">transfer</a>(<a href="nft.md#0x2_nft">nft</a>: <a href="nft.md#0x2_nft_Nft">Nft</a>, recipient: <b>address</b>) {
    <a href="transfer.md#0x2_transfer_public_transfer">transfer::public_transfer</a>(<a href="nft.md#0x2_nft">nft</a>, recipient);
}
</code></pre>



</details>

<a name="0x2_nft_update"></a>

## Function `update`

Update the mutable descriptive fields.

Deliberately does **not** touch <code>attributes</code>. They are a separate
operation with separate entry points (<code>set_attribute</code> and friends), so a
caller renaming a token cannot silently drop its traits, and a caller
editing traits does not have to restate the name.


<pre><code><b>public</b> <b>fun</b> <b>update</b>(<a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">nft::Nft</a>, name: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, description: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, image_url: <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>update</b>(
    <a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">Nft</a>,
    name: String,
    description: Option&lt;String&gt;,
    image_url: Option&lt;Url&gt;
) {
    <a href="nft.md#0x2_nft">nft</a>.name = name;
    <a href="nft.md#0x2_nft">nft</a>.description = description;
    <a href="nft.md#0x2_nft">nft</a>.image_url = image_url;
}
</code></pre>



</details>

<a name="0x2_nft_burn"></a>

## Function `burn`

Destroy the NFT.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_burn">burn</a>(<a href="nft.md#0x2_nft">nft</a>: <a href="nft.md#0x2_nft_Nft">nft::Nft</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_burn">burn</a>(<a href="nft.md#0x2_nft">nft</a>: <a href="nft.md#0x2_nft_Nft">Nft</a>) {
    <b>let</b> <a href="nft.md#0x2_nft_Nft">Nft</a> {
        id,
        name: _,
        description: _,
        image_url: _,
        attributes: _
    } = <a href="nft.md#0x2_nft">nft</a>;
    <a href="object.md#0x2_object_delete">object::delete</a>(id);
}
</code></pre>



</details>

<a name="0x2_nft_id"></a>

## Function `id`

The token id.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_id">id</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): <a href="object.md#0x2_object_ID">object::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_id">id</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): ID {
    <a href="object.md#0x2_object_uid_to_inner">object::uid_to_inner</a>(&<a href="nft.md#0x2_nft">nft</a>.id)
}
</code></pre>



</details>

<a name="0x2_nft_nft_address"></a>

## Function `nft_address`

The NFT's object address.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_nft_address">nft_address</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_nft_address">nft_address</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): <b>address</b> {
    <a href="object.md#0x2_object_uid_address">object::uid_address</a>(&<a href="nft.md#0x2_nft">nft</a>.id)
}
</code></pre>



</details>

<a name="0x2_nft_name"></a>

## Function `name`

The name field.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_name">name</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_name">name</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): String {
    <a href="nft.md#0x2_nft">nft</a>.name
}
</code></pre>



</details>

<a name="0x2_nft_description"></a>

## Function `description`

The description field.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_description">description</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_description">description</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): Option&lt;String&gt; {
    <a href="nft.md#0x2_nft">nft</a>.description
}
</code></pre>



</details>

<a name="0x2_nft_image_url"></a>

## Function `image_url`

The image URL field.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_image_url">image_url</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="url.md#0x2_url_Url">url::Url</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_image_url">image_url</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): Option&lt;Url&gt; {
    <a href="nft.md#0x2_nft">nft</a>.image_url
}
</code></pre>



</details>

<a name="0x2_nft_attribute"></a>

## Function `attribute`

The value stored under <code>key</code>, or <code>none</code>.

This is the ERC-721 <code>tokenURI</code>-style attribute read the design doc
specifies: a keyed lookup rather than positional indexing, so a missing
trait is absent rather than shifted onto the wrong one.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_attribute">attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>, key: &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>): <a href="dependencies/move-stdlib/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_attribute">attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>, key: &String): Option&lt;String&gt; {
    <a href="vec_map.md#0x2_vec_map_try_get">vec_map::try_get</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes, key)
}
</code></pre>



</details>

<a name="0x2_nft_set_attribute"></a>

## Function `set_attribute`

Insert or overwrite <code>key</code>. Upsert rather than insert-only, because the
common case is "this trait's value changed", and forcing callers to
remove-then-insert would let a read-modify-write race leave the map
empty in between.

Overwriting an existing key does not consume another slot, so the count
check is skipped on that path — otherwise a legitimate update at the
limit would be refused.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_set_attribute">set_attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">nft::Nft</a>, key: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>, value: <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_set_attribute">set_attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">Nft</a>, key: String, value: String) {
    <b>if</b> (<a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes, &key)) {
        <b>let</b> (_removed_key, _removed_value) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(
            &<b>mut</b> <a href="nft.md#0x2_nft">nft</a>.attributes, &key
        );
    } <b>else</b> {
        // Only a genuinely new key adds an entry, so this is the only place
        // the bound <b>has</b> <b>to</b> be enforced. Checking it here rather than at
        // construction is what stops an incremental caller from walking
        // past the limit one `set_attribute` at a time.
        <b>assert</b>!(
            <a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes) &lt; <a href="nft.md#0x2_nft_MAX_ATTRIBUTES">MAX_ATTRIBUTES</a>,
            <a href="nft.md#0x2_nft_E_TOO_MANY_ATTRIBUTES">E_TOO_MANY_ATTRIBUTES</a>
        );
    };
    <a href="vec_map.md#0x2_vec_map_insert">vec_map::insert</a>(&<b>mut</b> <a href="nft.md#0x2_nft">nft</a>.attributes, key, value);
}
</code></pre>



</details>

<a name="0x2_nft_remove_attribute"></a>

## Function `remove_attribute`

Remove <code>key</code> and return its value. Aborts if absent, so a caller cannot
believe it cleared a trait that was never there.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_remove_attribute">remove_attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">nft::Nft</a>, key: &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>): <a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_remove_attribute">remove_attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">Nft</a>, key: &String): String {
    <b>let</b> (_removed_key, value) = <a href="vec_map.md#0x2_vec_map_remove">vec_map::remove</a>(&<b>mut</b> <a href="nft.md#0x2_nft">nft</a>.attributes, key);
    value
}
</code></pre>



</details>

<a name="0x2_nft_set_attributes"></a>

## Function `set_attributes`

Replace every attribute at once, from parallel vectors.

The wholesale form, matching how a legacy caller that stored
<code>attribute_keys</code> / <code>attribute_values</code> updates them. Length and count are
checked **before** anything is mutated, so a bad call leaves the token's
existing attributes intact rather than half-applied — the replacement is
built first and only then swapped in.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_set_attributes">set_attributes</a>(<a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">nft::Nft</a>, attribute_keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, attribute_values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_set_attributes">set_attributes</a>(
    <a href="nft.md#0x2_nft">nft</a>: &<b>mut</b> <a href="nft.md#0x2_nft_Nft">Nft</a>,
    attribute_keys: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    attribute_values: <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;
) {
    <b>let</b> attribute_keys = attribute_keys;
    <b>let</b> attribute_values = attribute_values;
    <b>assert</b>!(
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&attribute_keys) == <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&attribute_values),
        <a href="nft.md#0x2_nft_E_ATTRIBUTE_LENGTH_MISMATCH">E_ATTRIBUTE_LENGTH_MISMATCH</a>
    );
    <b>assert</b>!(
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&attribute_keys) &lt;= <a href="nft.md#0x2_nft_MAX_ATTRIBUTES">MAX_ATTRIBUTES</a>,
        <a href="nft.md#0x2_nft_E_TOO_MANY_ATTRIBUTES">E_TOO_MANY_ATTRIBUTES</a>
    );
    // `String` <b>has</b> `drop`, so `VecMap&lt;String, String&gt;` does too and the
    // superseded map is released here rather than needing a drain <b>loop</b>.
    <a href="nft.md#0x2_nft">nft</a>.attributes = <a href="vec_map.md#0x2_vec_map_from_keys_values">vec_map::from_keys_values</a>(attribute_keys, attribute_values);
}
</code></pre>



</details>

<a name="0x2_nft_has_attribute"></a>

## Function `has_attribute`

True iff <code>key</code> is present.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_has_attribute">has_attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>, key: &<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_has_attribute">has_attribute</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>, key: &String): bool {
    <a href="vec_map.md#0x2_vec_map_contains">vec_map::contains</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes, key)
}
</code></pre>



</details>

<a name="0x2_nft_attribute_count"></a>

## Function `attribute_count`

How many attributes the token carries.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_attribute_count">attribute_count</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_attribute_count">attribute_count</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): u64 {
    <a href="vec_map.md#0x2_vec_map_size">vec_map::size</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes)
}
</code></pre>



</details>

<a name="0x2_nft_attribute_keys"></a>

## Function `attribute_keys`

Attribute keys in insertion order. Not sorted — do not rely on ordering.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_attribute_keys">attribute_keys</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_attribute_keys">attribute_keys</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt; {
    <a href="vec_map.md#0x2_vec_map_keys">vec_map::keys</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes)
}
</code></pre>



</details>

<a name="0x2_nft_all_attributes"></a>

## Function `all_attributes`

Every <code>(key, value)</code> pair in insertion order, for a renderer that wants
the whole set in one call. Keys and values are index-aligned, so element
<code>i</code> of each is one attribute.


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_all_attributes">all_attributes</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">nft::Nft</a>): (<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;, <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;<a href="dependencies/move-stdlib/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="nft.md#0x2_nft_all_attributes">all_attributes</a>(<a href="nft.md#0x2_nft">nft</a>: &<a href="nft.md#0x2_nft_Nft">Nft</a>): (<a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;, <a href="dependencies/move-stdlib/vector.md#0x1_vector">vector</a>&lt;String&gt;) {
    <b>let</b> keys = <a href="vec_map.md#0x2_vec_map_keys">vec_map::keys</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes);
    <b>let</b> values = <a href="dependencies/move-stdlib/vector.md#0x1_vector_empty">vector::empty</a>&lt;String&gt;();
    <b>let</b> i = 0;
    <b>let</b> n = <a href="dependencies/move-stdlib/vector.md#0x1_vector_length">vector::length</a>(&keys);
    <b>while</b> (i &lt; n) {
        <b>let</b> key = *<a href="dependencies/move-stdlib/vector.md#0x1_vector_borrow">vector::borrow</a>(&keys, i);
        <a href="dependencies/move-stdlib/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> values, *<a href="vec_map.md#0x2_vec_map_get">vec_map::get</a>(&<a href="nft.md#0x2_nft">nft</a>.attributes, &key));
        i = i + 1;
    };
    (keys, values)
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
