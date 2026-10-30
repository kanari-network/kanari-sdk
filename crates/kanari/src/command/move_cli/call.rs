// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::command::common::{
    build_blocking_client, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{
    render_transaction_submission, require_rpc_result, sign_call_function_request,
    submit_blocking_rpc,
};
use anyhow::{Context, Result, bail};
use clap::*;
use kanari_rpc_api::{BuildCallFunctionRequest, CallFunctionRequest, methods};
use kanari_types::{GasEstimate, GasOperation};
use move_core_types::{
    account_address::AccountAddress, parser, runtime_value::MoveValue, u256::U256,
};

/// Call a Move function on the blockchain
#[derive(Parser)]
#[clap(
    name = "call",
    about = "Call a Move function on the blockchain",
    after_help = "Examples:\n  \
        kanari call --package 0x1 --module coin --function transfer --args 0x2 1000\n  \
        kanari call --package 0x1 --module nft --function mutate --args 0xabc \"new name\"\n  \
        kanari call --package 0x1 --module dex --function place --args @0x403 100u8 [1u64,2u64]\n\n\
        Argument syntax (first match wins):\n  \
        1. Typed prefix (fail-fast):  u8:100  u64:1_000  u256:0xff  bool:true  string:hi\n  \
                                       address:0x403  hex:0xdeadbeef  raw:hi\n  \
                                       vector<address>:[0x1,0x2]  vector<u8>:0xdeadbeef\n  \
        2. Bool:                      true / false\n  \
        3. Move byte strings:         b\"raw bytes\"  x\"0badc0de\"\n  \
        4. Address literal:           @0x403\n  \
        5. Quoted string:             \"hello\"  (→ vector<u8>)\n  \
        6. Vector literal:            [1,2,3]  [1u8,2u8]  [\"a\",\"b\"]  [[1,2],[3,4]]  [0x1,0x2]\n  \
        7. Number with suffix:        100u8  100u16  100u32  100u64  100u128  100u256  0x10u8\n  \
        8. Address / object id:       0x1  0x403  0xabc...64hex  (auto-padded to 32 bytes)\n  \
        9. Plain integer:             1000  (u64; underscores allowed: 1_000)\n  \
        10. String fallback:          anything else → UTF-8 bytes (warned)"
)]
pub struct Call {
    #[clap(long = "package")]
    pub package: String,
    #[clap(long = "module", value_name = "MODULE")]
    pub module: String,
    #[clap(long = "function")]
    pub function: String,
    #[clap(long = "type-args")]
    pub type_args: Vec<String>,
    #[clap(long = "args", num_args = 0..)]
    pub args: Vec<String>,
    #[clap(long = "gas-limit")]
    pub gas_limit: Option<u64>,
    #[clap(long = "gas-price")]
    pub gas_price: Option<u64>,
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Call {
    pub fn execute(self) -> Result<()> {
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let (gas_limit, gas_price) = resolve_transaction_gas(self.gas_limit, self.gas_price);

        if self.package.contains("::") {
            anyhow::bail!(
                "Combined form `address::module` is not supported. Use `--package <ADDRESS> --module <MODULE>` instead."
            );
        }

        eprintln!("Preparing function call...");
        let sender_normalized = resolve_sender(None)?;
        let package_normalized = normalize_addr(&self.package)
            .with_context(|| format!("Invalid package address: {}", self.package))?;
        let client = build_blocking_client(30)?;

        eprintln!("Call Details:");
        eprintln!("   Package: {}::{}", package_normalized, self.module);
        eprintln!("   Function: {}", self.function);
        eprintln!("   Sender: {}", sender_normalized);
        eprintln!("   Gas Limit: {}", gas_limit);
        eprintln!("   Gas Price: {}", gas_price);

        let wallet = load_wallet_for(&sender_normalized, None)?;
        eprintln!(
            "   Wallet loaded: {} (curve: {})",
            sender_normalized, wallet.curve_type
        );
        let sender_for_tx = get_sender_for_tx(&wallet, &sender_normalized)?;

        if !self.type_args.is_empty() {
            for type_arg in &self.type_args {
                parser::parse_type_tag(type_arg.trim())
                    .with_context(|| format!("Invalid type argument: {}", type_arg))?;
            }
            eprintln!("   Type Args: {}", self.type_args.join(", "));
        }

        let parsed_args: Vec<Vec<u8>> = self
            .args
            .iter()
            .map(|arg| self.parse_arg_flexible(arg))
            .collect::<Result<_>>()?;

        if !parsed_args.is_empty() {
            eprintln!("   Arguments: {} args provided", parsed_args.len());
        }

        let estimate =
            GasEstimate::from_operation(GasOperation::ExecuteFunction { complexity: 1 }, gas_price);
        eprintln!("Gas estimation:");
        eprintln!("   Estimated: {} units", estimate.gas_units);
        eprintln!("   Limit: {} units", gas_limit);
        eprintln!(
            "   Total Cost: {} Mist ({:.9} KANARI)",
            estimate.total_cost_mist, estimate.total_cost_kanari
        );

        eprintln!("Creating transaction via API...");
        let prepared = submit_blocking_rpc(
            &client,
            &rpc,
            methods::BUILD_CALL_FUNCTION,
            serde_json::to_value(BuildCallFunctionRequest {
                sender: sender_for_tx.clone(),
                package: package_normalized,
                module: self.module.clone(),
                function: self.function.clone(),
                type_args: self.type_args.clone(),
                args: parsed_args,
                object_inputs: None,
                gas_limit,
                gas_price,
                nonce: None,
                execute_immediate: Some(true),
            })
            .context("Failed to serialize build call request")?,
        )?;

        let prepared: CallFunctionRequest = serde_json::from_value(require_rpc_result(
            prepared,
            "RPC did not return prepared call request",
        )?)
        .context("Failed to decode prepared call request")?;

        if let Some(object_inputs) = &prepared.object_inputs
            && !object_inputs.is_empty()
        {
            eprintln!(
                "   Object Inputs: {} API-resolved object ref(s)",
                object_inputs.len()
            );
        }
        if let Some(gas_payment) = &prepared.gas_payment
            && let Some(gas_object) = gas_payment.payment_objects.first()
        {
            eprintln!("   Gas payment object: {}", gas_object.object_id);
        }

        let signed = sign_call_function_request(prepared, &wallet)?;
        eprintln!("   Transaction signed (curve: {})", wallet.curve_type);

        eprintln!("Sending RPC request to {}...", rpc);
        let rpc_response = submit_blocking_rpc(
            &client,
            &rpc,
            methods::CALL_FUNCTION,
            serde_json::to_value(signed).context("Failed to serialize call request")?,
        )?;
        render_transaction_submission(&client, &rpc, rpc_response, "", true)?;

        Ok(())
    }

    // =========================================================================
    // Argument parsing — Sui-style: every pure Move value has a CLI syntax.
    // =========================================================================
    //
    // Priority order (first match wins):
    //   0. Explicit typed prefix    → fail-fast: `u8:100`, `vector<address>:[0x1,0x2]`
    //   1. Bool                     → `true` / `false`
    //   2. Move byte-string literal → `b"raw bytes"`, `x"0badc0de"` → vector<u8>
    //   3. Address literal          → `@0x403`
    //   4. Quoted string            → `"hello"` → vector<u8>
    //   5. Vector literal           → `[1,2,3]` (CSV, nested, quoted, typed elements)
    //   6. Number with suffix       → `100u8` … `100u256`, `1_000u64`, `0x10u8`
    //   7. Address / object id      → `0x…` auto-padded to 32 bytes
    //   8. Plain integer            → `1000` → u64 (`1_000` allowed)
    //   9. String fallback          → UTF-8 bytes (warned, so the guess is visible)
    //
    // Typed prefixes: bool u8 u16 u32 u64 u128 u256 address signer string hex
    //                 raw vector<T>. `hex:`/`raw:` emit unframed bytes; every
    //                 other form emits canonical BCS for the Move value it
    //                 denotes — one encoding path, no per-shape special cases.
    //
    // Rejected on purpose (reported as errors, never silently degraded):
    //   - `0xaddr::module::func` inside --args: that's a function id, not a value
    //   - unbalanced `[`, `0x` without hex digits, addresses over 32 bytes
    //   - JSON object/array without type info: would silently produce wrong BCS
    // =========================================================================

    fn parse_arg_flexible(&self, arg: &str) -> Result<Vec<u8>> {
        parse_arg(arg, true)?.into_bytes()
    }
}

// =============================================================================
// ParsedArg — separates BCS-framed Move values from raw byte blobs
// =============================================================================

/// A parsed CLI argument.
///
/// Every form that denotes a Move value is BCS-encoded exactly once, in
/// [`ParsedArg::into_bytes`]. `hex:` / `raw:` are the exception: they exist to
/// emit verbatim bytes for parameters that are not Move values (a blob the
/// callee decodes itself), so they bypass BCS framing.
enum ParsedArg {
    Value(MoveValue),
    Raw(Vec<u8>),
}

impl ParsedArg {
    fn into_bytes(self) -> Result<Vec<u8>> {
        match self {
            ParsedArg::Raw(bytes) => Ok(bytes),
            ParsedArg::Value(value) => value
                .simple_serialize()
                .context("Failed to BCS-serialize argument"),
        }
    }

    /// Nested positions (vector elements) only accept real Move values.
    fn require_value(self) -> Result<MoveValue> {
        match self {
            ParsedArg::Value(value) => Ok(value),
            ParsedArg::Raw(_) => bail!(
                "`hex:`/`raw:` produce unframed bytes and cannot be nested; \
                 use `vector<u8>:0x...` to build a byte vector instead"
            ),
        }
    }
}

// =============================================================================
// Top-level parser — priority-ordered, first match wins
// =============================================================================

/// Parse one CLI argument into a Move value (or raw bytes). Recurses into
/// vector literals, so `[1u8, 2u8]`, `[u8:1, u8:2]` and a top-level `1u8` all
/// go through the same code path.
///
/// `warn_fallback` is set only for the top-level argument: nested elements
/// fall back silently, so `[a,b,c]` prints one line instead of three.
fn parse_arg(arg: &str, warn_fallback: bool) -> Result<ParsedArg> {
    let s = arg.trim();
    if s.is_empty() {
        bail!("Empty argument is not allowed");
    }

    // 0. Explicit typed prefix — fail-fast, no guessing. Tags are matched
    //    case-insensitively (`U8:100`), and a prefix that is *meant* to be a
    //    type tag but is not one (`u65:`, `i64:`, `bytes:`) errors out instead
    //    of silently sending the whole thing as a string.
    if let Some((ty, value)) = s.split_once(':') {
        let tag = ty.to_ascii_lowercase();
        if is_known_arg_type_tag(&tag) {
            return parse_typed_arg(&tag, value.trim())
                .with_context(|| format!("Failed to parse typed argument `{}`", s));
        }
        if looks_like_type_tag(ty) {
            bail!(
                "`{}` is not a supported type tag — Move has no signed, floating \
                 point, or option types here. Valid tags: bool, u8, u16, u32, u64, \
                 u128, u256, address, string, hex, raw, vector<T> (or drop the \
                 prefix to pass the value as a string)",
                ty
            );
        }
    }

    // 1. Bool
    if s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("false") {
        return Ok(ParsedArg::Value(MoveValue::Bool(parse_bool(s)?)));
    }

    // 2. Move byte-string literals → vector<u8>: `b"raw bytes"`, `x"0badc0de"`
    if let Some(value) = parse_byte_string(s)? {
        return Ok(ParsedArg::Value(value));
    }

    // 3. Move address literal `@0x403`. Anything else after `@` (`@alice`) is
    //    just a string and falls through.
    if let Some(rest) = s.strip_prefix('@')
        && has_hex_prefix(rest)
    {
        let address =
            parse_address_value(rest).with_context(|| format!("Invalid `@{}` address", rest))?;
        return Ok(ParsedArg::Value(MoveValue::Address(address)));
    }

    // 4. Quoted string → vector<u8>
    if let Some(inner) = strip_quotes(s) {
        return Ok(ParsedArg::Value(MoveValue::vector_u8(
            inner.as_bytes().to_vec(),
        )));
    }

    // 5. Vector literal: [a, b, c] — recursive, CSV-aware, typed elements.
    if s.starts_with('[') {
        if !s.ends_with(']') || !brackets_balanced(s) {
            bail!("Unbalanced `[` in argument `{}`", s);
        }
        let elements = parse_vector_literal(&s[1..s.len() - 1])
            .with_context(|| format!("Failed to parse vector `{}`", s))?;
        return Ok(ParsedArg::Value(MoveValue::Vector(elements)));
    }

    // 6. Number with suffix. Checked before the address branch: hex digits can
    //    never contain the letter `u`, so `0x10u8` cannot be a valid address.
    if let Some(value) = parse_numeric_suffix(s)? {
        return Ok(ParsedArg::Value(value));
    }

    // 7. Address / object id — `0x...` only, auto-padded to the full 32 bytes.
    if !s.contains("::") && has_hex_prefix(s) {
        let address = parse_address_value(s)?;
        return Ok(ParsedArg::Value(MoveValue::Address(address)));
    }

    // 8. Plain integer → u64 (leading zeros rejected: `007` is a typo or a string)
    if let Some(value) = parse_plain_u64(s) {
        return Ok(ParsedArg::Value(MoveValue::U64(value)));
    }

    // 9. Fallback: UTF-8 string bytes — warn so the guess is visible
    if warn_fallback {
        eprintln!(
            "[CLI] warning: argument `{}` did not match any known type, treating as UTF-8 \
             string (quote it, or use `string:`/`hex:`/`raw:` to be explicit)",
            s
        );
    }
    Ok(ParsedArg::Value(MoveValue::vector_u8(
        s.as_bytes().to_vec(),
    )))
}

// =============================================================================
// Type-tag helpers
// =============================================================================

/// Recognized type tags for the explicit `type:value` prefix.
/// Keeping this list short & exact avoids false positives on strings that
/// happen to contain `:` (e.g. URLs, timestamps). An unknown prefix is *not*
/// an error — it is a string — which is why the list is exact rather than
/// prefix-guessy.
fn is_known_arg_type_tag(ty: &str) -> bool {
    matches!(
        ty,
        "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "u256"
            | "address"
            | "signer"
            | "string"
            | "hex"
            | "raw"
    ) || ty.starts_with("vector<")
}

fn is_vector_type(ty: &str) -> bool {
    ty.starts_with("vector<") && ty.ends_with('>')
}

/// Prefixes that are almost-but-not-quite type tags. Hitting one means the
/// user intended a typed value, so an error listing the valid tags beats
/// silently sending `u65:5` as a string and failing later inside the VM.
fn looks_like_type_tag(ty: &str) -> bool {
    let t = ty.to_ascii_lowercase();
    let numeric_suffix = |prefix: char| {
        let rest = t.strip_prefix(prefix).unwrap_or("");
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    };
    // u65 / i64 / f64 — widths and kinds Move does not have.
    numeric_suffix('u')
        || numeric_suffix('i')
        || numeric_suffix('f')
        || matches!(
            t.as_str(),
            "byte"
                | "bytes"
                | "str"
                | "text"
                | "utf8"
                | "utf-8"
                | "addr"
                | "vec"
                | "vector"
                | "array"
                | "list"
                | "number"
                | "int"
                | "uint"
                | "char"
        )
        || t.starts_with("vec<")
}

// =============================================================================
// Primitive parsers
// =============================================================================

/// `true` / `false` / `1` / `0`, case-insensitive.
fn parse_bool(v: &str) -> Result<bool> {
    let text = v.trim();
    match text.to_ascii_lowercase().as_str() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => bail!("expected true/false, got `{}`", text),
    }
}

/// Move byte-string literals: `b"raw bytes"` and `x"deadbeef"` → `vector<u8>`.
/// Returns `Ok(None)` when `s` is not such a literal; an `x"..."` that *is*
/// shaped like a literal but is not valid hex fails fast instead of degrading
/// to a string.
fn parse_byte_string(s: &str) -> Result<Option<MoveValue>> {
    if s.len() < 3 || !s.ends_with('"') {
        return Ok(None);
    }
    let kind = s.as_bytes()[0];
    if kind != b'b' && kind != b'x' {
        return Ok(None);
    }
    let Some(inner) = strip_quotes(&s[1..]) else {
        return Ok(None);
    };
    if kind == b'b' {
        return Ok(Some(MoveValue::vector_u8(inner.as_bytes().to_vec())));
    }
    // x"..." — empty is allowed (zero-length byte vector)
    if inner.is_empty() {
        return Ok(Some(MoveValue::vector_u8(vec![])));
    }
    Ok(Some(MoveValue::vector_u8(hex_decode(inner).with_context(
        || format!("`{}` is not a valid hex byte-string literal", s),
    )?)))
}

/// Strip one layer of matching surrounding quotes. `None` unless the whole
/// value is quoted, so a value that merely *starts* with a quote stays intact.
fn strip_quotes(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let quote = bytes[0];
    if (quote == b'"' || quote == b'\'') && bytes[bytes.len() - 1] == quote {
        return Some(&s[1..bytes.len() - 1]);
    }
    None
}

fn strip_hex_prefix(s: &str) -> Option<&str> {
    s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
}

fn has_hex_prefix(s: &str) -> bool {
    strip_hex_prefix(s).is_some()
}

// =============================================================================
// Typed-arg dispatch
// =============================================================================

/// Parse an argument with an explicit type prefix.
/// Fail-fast: a value that does not match its declared type is an error, never
/// a silent fallback to another interpretation.
fn parse_typed_arg(ty: &str, value: &str) -> Result<ParsedArg> {
    let v = value.trim();
    match ty {
        "bool" => Ok(ParsedArg::Value(MoveValue::Bool(
            parse_bool(v).with_context(|| format!("bool: invalid value `{}`", v))?,
        ))),
        "u8" => Ok(ParsedArg::Value(MoveValue::U8(parse_int_literal(v, v)?))),
        "u16" => Ok(ParsedArg::Value(MoveValue::U16(parse_int_literal(v, v)?))),
        "u32" => Ok(ParsedArg::Value(MoveValue::U32(parse_int_literal(v, v)?))),
        "u64" => Ok(ParsedArg::Value(MoveValue::U64(parse_int_literal(v, v)?))),
        "u128" => Ok(ParsedArg::Value(MoveValue::U128(parse_int_literal(v, v)?))),
        "u256" => Ok(ParsedArg::Value(MoveValue::U256(parse_int_literal(v, v)?))),
        "address" => Ok(ParsedArg::Value(MoveValue::Address(
            parse_address_value(v).with_context(|| format!("address: invalid value `{}`", v))?,
        ))),
        "signer" => bail!("signer cannot be passed as a CLI argument"),
        "string" => {
            // Quoted or bare; BCS of a Move `String` is the BCS of its bytes.
            let text = strip_quotes(v).unwrap_or(v);
            Ok(ParsedArg::Value(MoveValue::vector_u8(
                text.as_bytes().to_vec(),
            )))
        }
        "hex" => Ok(ParsedArg::Raw(
            hex_decode(v).with_context(|| format!("hex: invalid value `{}`", v))?,
        )),
        "raw" => Ok(ParsedArg::Raw(v.as_bytes().to_vec())),
        _ if is_vector_type(ty) => {
            let inner_ty = &ty["vector<".len()..ty.len() - 1];
            if inner_ty.is_empty() {
                bail!("`{}`: missing element type", ty);
            }
            let elements = parse_typed_vector(v, inner_ty)
                .with_context(|| format!("{}: failed to parse `{}`", ty, v))?;
            Ok(ParsedArg::Value(MoveValue::Vector(elements)))
        }
        _ => bail!("Unsupported type tag `{}`", ty),
    }
}

/// Elements of a typed vector. Accepts `[a, b]`, bare CSV (`1,2,3`) and — for
/// `vector<u8>` — a hex blob (`0xdeadbeef`), matching how callers reach for
/// byte vectors in practice.
///
/// Every element is built as a `MoveValue` and serialized together, so
/// `vector<u8>` gets a single ULEB length prefix instead of one per element.
fn parse_typed_vector(value: &str, inner_ty: &str) -> Result<Vec<MoveValue>> {
    let v = value.trim();

    // `vector<u8>:0xdeadbeef` — hex straight into the vector's bytes.
    if inner_ty == "u8" && !v.starts_with('[') && has_hex_prefix(v) {
        return Ok(hex_decode(v)?.into_iter().map(MoveValue::U8).collect());
    }

    let body = if v.starts_with('[') {
        if !v.ends_with(']') || !brackets_balanced(v) {
            bail!("unbalanced `[` in `{}`", v);
        }
        &v[1..v.len() - 1]
    } else {
        v
    };
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }

    split_csv(body)
        .into_iter()
        .map(|part| parse_typed_arg(inner_ty, part.trim())?.require_value())
        .collect()
}

// =============================================================================
// Numeric literal parsing
// =============================================================================

/// Peel a numeric literal into (digits, radix), or `None` when `s` is not one.
/// `_` digit separators are accepted (`1_000`, `0xdead_beef`), Move-style.
fn int_literal(s: &str) -> Option<(&str, u32)> {
    if let Some(hex) = strip_hex_prefix(s) {
        if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit() || c == '_') {
            return Some((hex, 16));
        }
        return None;
    }
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '_') {
        return Some((s, 10));
    }
    None
}

/// Parse a numeric literal (`100`, `1_000`, `0xff`) as `T`.
/// `display` is the whole argument, so error messages show what the user typed.
fn parse_int_literal<T: ParseIntLiteral>(text: &str, display: &str) -> Result<T> {
    let Some((digits, radix)) = int_literal(text) else {
        bail!("`{}` is not a number", display);
    };
    let cleaned: String = digits.chars().filter(|c| *c != '_').collect();
    if cleaned.is_empty() {
        bail!("`{}` is not a number", display);
    }
    T::parse_radix(&cleaned, radix).with_context(|| format!("invalid number `{}`", display))
}

/// Decimal/hex parsing for every integer width the CLI accepts, so `u8` …
/// `u256` share one code path instead of one `.parse()` per arm.
trait ParseIntLiteral: Sized {
    fn parse_radix(digits: &str, radix: u32) -> Result<Self>;
}

macro_rules! impl_parse_int_literal {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ParseIntLiteral for $ty {
                fn parse_radix(digits: &str, radix: u32) -> Result<Self> {
                    <$ty>::from_str_radix(digits, radix).with_context(|| {
                        format!("`{}` is out of range for {}", digits, stringify!($ty))
                    })
                }
            }
        )+
    };
}
impl_parse_int_literal!(u8, u16, u32, u64, u128);

impl ParseIntLiteral for U256 {
    fn parse_radix(digits: &str, radix: u32) -> Result<Self> {
        U256::from_str_radix(digits, radix)
            .with_context(|| format!("`{}` is out of range for u256", digits))
    }
}

/// Try to parse a numeric literal with a Move-style suffix (`100u8`, `1_000u128`).
/// Returns `Ok(Some(value))` on success, `Ok(None)` if no suffix matched.
fn parse_numeric_suffix(s: &str) -> Result<Option<MoveValue>> {
    // Longest suffix first so `100u128` is never read as `100u12` + `8`… and so
    // `u256` is matched before any shorter prefix of it.
    const SUFFIXES: &[&str] = &["u128", "u256", "u64", "u32", "u16", "u8"];
    for suffix in SUFFIXES {
        let Some(digits) = s.strip_suffix(suffix) else {
            continue;
        };
        // A string that merely ends in `u8` (`myu8`) is not a number: only take
        // the suffix when what precedes it looks like one.
        if int_literal(digits).is_none() {
            continue;
        }
        return Ok(Some(match *suffix {
            "u8" => MoveValue::U8(parse_int_literal(digits, s)?),
            "u16" => MoveValue::U16(parse_int_literal(digits, s)?),
            "u32" => MoveValue::U32(parse_int_literal(digits, s)?),
            "u64" => MoveValue::U64(parse_int_literal(digits, s)?),
            "u128" => MoveValue::U128(parse_int_literal(digits, s)?),
            "u256" => MoveValue::U256(parse_int_literal(digits, s)?),
            _ => unreachable!("SUFFIXES and the match arms must stay in sync"),
        }));
    }
    Ok(None)
}

/// Plain decimal integer → `u64`. Leading zeros are rejected so `007` surfaces
/// as what it is (a typo or a string) rather than silently becoming the number 7.
fn parse_plain_u64(s: &str) -> Option<u64> {
    let digits: String = s.chars().filter(|c| *c != '_').collect();
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if digits.starts_with('0') && digits != "0" {
        return None;
    }
    digits.parse::<u64>().ok()
}

// =============================================================================
// Address parsing
// =============================================================================

/// Parse an address value (used by `address:0x…`, `@0x…` and the heuristic path).
/// Accepts `0x…` only and auto-pads the short form to a full 32-byte address;
/// anything that *looks* like an attempt at an address but is not one is an
/// error with a hint, never a silent string.
fn parse_address_value(s: &str) -> Result<AccountAddress> {
    let s = s.trim();
    let hex = strip_hex_prefix(s)
        .with_context(|| format!("address must start with `0x`, got `{}`", s))?;
    if hex.is_empty() {
        bail!("`{}` has no digits after `0x`", s);
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!(
            "`{}` is not a hex address (use `string:\"{}\"` if that is the literal string)",
            s,
            s
        );
    }
    if hex.len() > AccountAddress::LENGTH * 2 {
        bail!(
            "`{}` has {} hex digits; an address holds at most {} — use `hex:{}` for raw bytes \
             or `vector<u8>:0x{}` for a byte vector",
            s,
            hex.len(),
            AccountAddress::LENGTH * 2,
            s,
            hex
        );
    }
    let padded = format!("{:0>width$}", hex, width = AccountAddress::LENGTH * 2);
    // Make the padding visible: short ids are silently widened to 32 bytes, and
    // that widening is exactly what is worth eyeballing before a transaction.
    eprintln!("[CLI] address argument: 0x{}", padded);
    AccountAddress::from_hex_literal(&format!("0x{}", padded))
        .with_context(|| format!("invalid address `{}`", s))
}

// =============================================================================
// Vector literal parsing (CSV-aware, nested-bracket aware)
// =============================================================================

/// Parse a vector literal `[a, b, c]` into a `Vec<MoveValue>`.
///
/// Each element goes through the full top-level parser (`parse_arg`) minus the
/// fallback warning, so everything works nested: quoted strings with commas,
/// nested vectors, typed prefixes (`[u8:1, u8:2]`), suffixed numbers, and
/// addresses (`[0x1,0x2]` → `vector<address>`).
fn parse_vector_literal(inner: &str) -> Result<Vec<MoveValue>> {
    let inner = inner.trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    split_csv(inner)
        .into_iter()
        .map(|part| parse_arg(&part, false)?.require_value())
        .collect()
}

/// Bracket balance for a vector literal, ignoring brackets that live inside a
/// quoted region (`["a]"]` is balanced). Catches `[[1,2],[3,4]` before it is
/// silently reinterpreted as a vector of two vectors.
fn brackets_balanced(s: &str) -> bool {
    let mut depth: i64 = 0;
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            },
        }
    }
    depth == 0
}

/// Split a CSV-style string, respecting double- and single-quoted regions and
/// nested `[...]` groups — so `[[1,2],[3,4]]` splits into two elements, not
/// four fragments. Quotes and brackets are preserved in the output (the caller
/// decides what to do with them).
/// Empty trailing segments are kept so `[a,]` parses as `["a", ""]` — matches
/// user expectation and surfaces typos early.
fn split_csv(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0usize;

    for c in s.chars() {
        match quote {
            Some(q) if c == q => {
                cur.push(c);
                quote = None;
            }
            Some(_) => cur.push(c),
            None => match c {
                '"' | '\'' => {
                    quote = Some(c);
                    cur.push(c);
                }
                '[' => {
                    depth += 1;
                    cur.push(c);
                }
                ']' => {
                    depth = depth.saturating_sub(1);
                    cur.push(c);
                }
                ',' if depth == 0 => {
                    out.push(cur.clone());
                    cur.clear();
                }
                _ => cur.push(c),
            },
        }
    }
    out.push(cur);
    out
}

// =============================================================================
// Hex decoding
// =============================================================================

/// Decode a hex string into bytes. The `0x` prefix and `_` separators are
/// optional (`dead_beef`, `0xdeadbeef`), which is how Move spells byte strings.
fn hex_decode(s: &str) -> Result<Vec<u8>> {
    let hex: String = strip_hex_prefix(s)
        .unwrap_or(s)
        .chars()
        .filter(|c| *c != '_')
        .collect();
    if hex.is_empty() {
        bail!("expected hex digits, got `{}`", s);
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("invalid hex string `{}`", s);
    }
    if !hex.len().is_multiple_of(2) {
        bail!("hex string has odd length: `{}`", s);
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .with_context(|| format!("invalid hex byte at position {}", i))
        })
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Tiny stub so we can unit-test the pure helpers without constructing `Call`.
    fn parse(arg: &str) -> Result<Vec<u8>> {
        // Build a dummy Call; parse_arg_flexible doesn't read `self`.
        let dummy = Call {
            package: String::new(),
            module: String::new(),
            function: String::new(),
            type_args: vec![],
            args: vec![],
            gas_limit: None,
            gas_price: None,
            rpc_endpoint: None,
        };
        dummy.parse_arg_flexible(arg)
    }

    #[test]
    fn bool_true_false() {
        assert_eq!(parse("true").unwrap(), bcs::to_bytes(&true).unwrap());
        assert_eq!(parse("FALSE").unwrap(), bcs::to_bytes(&false).unwrap());
    }

    #[test]
    fn numeric_suffix() {
        assert_eq!(parse("100u8").unwrap(), bcs::to_bytes(&100u8).unwrap());
        assert_eq!(parse("1000u16").unwrap(), bcs::to_bytes(&1000u16).unwrap());
        assert_eq!(parse("1000u64").unwrap(), bcs::to_bytes(&1000u64).unwrap());
        assert_eq!(
            parse("340282366920938463463374607431768211455u128").unwrap(),
            bcs::to_bytes(&u128::MAX).unwrap()
        );
    }

    #[test]
    fn plain_integer_is_u64() {
        assert_eq!(parse("1000").unwrap(), bcs::to_bytes(&1000u64).unwrap());
        assert_eq!(parse("0").unwrap(), bcs::to_bytes(&0u64).unwrap());
    }

    #[test]
    fn address_short_form_auto_pads() {
        let bytes = parse("0x1").unwrap();
        assert_eq!(bytes.len(), AccountAddress::LENGTH);
        assert_eq!(bytes[bytes.len() - 1], 1);
    }

    #[test]
    fn x_prefix_is_treated_as_string() {
        // `x1234` is NOT an address — falls through to string fallback.
        let bytes = parse("x1234").unwrap();
        assert_eq!(bytes, bcs::to_bytes(&b"x1234".to_vec()).unwrap());
    }

    #[test]
    fn vector_of_integers() {
        let bytes = parse("[1,2,3]").unwrap();
        let expected = MoveValue::Vector(vec![
            MoveValue::U64(1),
            MoveValue::U64(2),
            MoveValue::U64(3),
        ])
        .simple_serialize()
        .unwrap();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn vector_with_type_suffix() {
        let bytes = parse("[1u8,2u8,3u8]").unwrap();
        let expected =
            MoveValue::Vector(vec![MoveValue::U8(1), MoveValue::U8(2), MoveValue::U8(3)])
                .simple_serialize()
                .unwrap();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn vector_with_comma_inside_quotes() {
        let bytes = parse(r#"["hello, world", "foo"]"#).unwrap();
        let expected = MoveValue::Vector(vec![
            MoveValue::Vector(b"hello, world".iter().map(|&b| MoveValue::U8(b)).collect()),
            MoveValue::Vector(b"foo".iter().map(|&b| MoveValue::U8(b)).collect()),
        ])
        .simple_serialize()
        .unwrap();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn nested_vector() {
        let bytes = parse("[[1,2],[3,4]]").unwrap();
        let expected = MoveValue::Vector(vec![
            MoveValue::Vector(vec![MoveValue::U64(1), MoveValue::U64(2)]),
            MoveValue::Vector(vec![MoveValue::U64(3), MoveValue::U64(4)]),
        ])
        .simple_serialize()
        .unwrap();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn typed_prefix_fail_fast() {
        // Wrong value for declared type → error, not silent fallback
        assert!(parse("u8:999").is_err());
        assert!(parse("bool:maybe").is_err());
        assert!(parse("address:hello").is_err());
    }

    #[test]
    fn typed_prefix_happy_path() {
        assert_eq!(parse("u8:42").unwrap(), bcs::to_bytes(&42u8).unwrap());
        assert_eq!(parse("bool:true").unwrap(), bcs::to_bytes(&true).unwrap());
        // BCS of `vector<u8>` is one ULEB length prefix plus the raw bytes —
        // never a length prefix per element.
        assert_eq!(
            parse("vector<u8>:1,2,3").unwrap(),
            bcs::to_bytes(&vec![1u8, 2u8, 3u8]).unwrap()
        );
        // Bare CSV and the bracketed form are the same value.
        assert_eq!(
            parse("vector<u8>:1,2,3").unwrap(),
            parse("[1u8,2u8,3u8]").unwrap()
        );
    }

    #[test]
    fn string_fallback_warns() {
        // Anything unrecognized → UTF-8 bytes (with warning printed to stderr)
        let bytes = parse("hello world").unwrap();
        assert_eq!(bytes, bcs::to_bytes(&b"hello world".to_vec()).unwrap());
    }

    #[test]
    fn quoted_string_strips_quotes() {
        assert_eq!(
            parse("\"hello\"").unwrap(),
            bcs::to_bytes(&b"hello".to_vec()).unwrap()
        );
        assert_eq!(
            parse("'hi'").unwrap(),
            bcs::to_bytes(&b"hi".to_vec()).unwrap()
        );
        // A quote inside a value does not strip anything.
        assert_eq!(
            parse("say \"hi\"").unwrap(),
            bcs::to_bytes(&b"say \"hi\"".to_vec()).unwrap()
        );
    }

    #[test]
    fn move_byte_string_literals() {
        assert_eq!(
            parse("b\"hi\"").unwrap(),
            bcs::to_bytes(&b"hi".to_vec()).unwrap()
        );
        assert_eq!(
            parse("x\"0badc0de\"").unwrap(),
            bcs::to_bytes(&vec![0x0bu8, 0xadu8, 0xc0u8, 0xdeu8]).unwrap()
        );
        // Underscores are allowed, Move-style.
        assert_eq!(
            parse("x\"0bad_c0de\"").unwrap(),
            bcs::to_bytes(&vec![0x0bu8, 0xadu8, 0xc0u8, 0xdeu8]).unwrap()
        );
        // Not a literal at all → ordinary string.
        assert_eq!(
            parse("brr").unwrap(),
            bcs::to_bytes(&b"brr".to_vec()).unwrap()
        );
    }

    #[test]
    fn at_address_literal() {
        let bytes = parse("@0x403").unwrap();
        assert_eq!(bytes.len(), AccountAddress::LENGTH);
        assert_eq!(&bytes[AccountAddress::LENGTH - 2..], &[0x04u8, 0x03u8]);
        // `@` followed by a non-address is just a string, not an error.
        assert_eq!(
            parse("@alice").unwrap(),
            bcs::to_bytes(&b"@alice".to_vec()).unwrap()
        );
    }

    #[test]
    fn vector_of_addresses() {
        let bytes = parse("[0x1,0x2]").unwrap();
        assert_eq!(bytes.len(), 1 + 2 * AccountAddress::LENGTH);
        assert_eq!(bytes[0], 2); // ULEB element count
        assert_eq!(bytes[1 + AccountAddress::LENGTH - 1], 1);
        assert_eq!(bytes[1 + 2 * AccountAddress::LENGTH - 1], 2);
    }

    #[test]
    fn vector_u8_hex_shorthand() {
        let typed = parse("vector<u8>:0x010203").unwrap();
        assert_eq!(typed, bcs::to_bytes(&vec![1u8, 2u8, 3u8]).unwrap());
        assert_eq!(typed, parse("[1u8,2u8,3u8]").unwrap());
        // Empty vector is one ULEB zero byte.
        assert_eq!(parse("vector<u8>:").unwrap(), vec![0u8]);
        assert_eq!(parse("[]").unwrap(), vec![0u8]);
    }

    #[test]
    fn typed_vector_of_strings() {
        let bytes = parse("vector<string>:[\"a\",\"bb\"]").unwrap();
        let expected = vec![
            MoveValue::vector_u8(b"a".to_vec()),
            MoveValue::vector_u8(b"bb".to_vec()),
        ];
        assert_eq!(
            bytes,
            MoveValue::Vector(expected).simple_serialize().unwrap()
        );
    }

    #[test]
    fn u256_suffix_and_typed_prefix() {
        let mut expected = vec![0u8; 32];
        expected[0] = 255;
        assert_eq!(parse("255u256").unwrap(), expected);
        assert_eq!(parse("u256:0xff").unwrap(), expected);
        // Widest value: 2^256 - 1 → 32 bytes of 0xff.
        assert_eq!(
            parse(
                "115792089237316195423570985008687907853269984665640564039457584007913129639935u256"
            )
            .unwrap(),
            vec![0xffu8; 32]
        );
        // One past the max → rejected, not truncated.
        assert!(
            parse(
                "115792089237316195423570985008687907853269984665640564039457584007913129639936u256"
            )
            .is_err()
        );
    }

    #[test]
    fn hex_and_raw_typed_are_unframed() {
        assert_eq!(
            parse("hex:0xdeadbeef").unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
        assert_eq!(
            parse("hex:dead_beef").unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
        assert_eq!(parse("raw:hi").unwrap(), b"hi".to_vec());
        assert!(parse("hex:0xdeadbee").is_err()); // odd length
        assert!(parse("hex:nope").is_err());
        // Raw bytes cannot be nested inside a value.
        assert!(parse("[hex:00]").is_err());
    }

    #[test]
    fn numeric_edge_cases() {
        // Underscore separators, decimal and hex.
        assert_eq!(parse("1_000").unwrap(), bcs::to_bytes(&1000u64).unwrap());
        assert_eq!(parse("1_000u64").unwrap(), bcs::to_bytes(&1000u64).unwrap());
        assert_eq!(parse("u8:0x10").unwrap(), bcs::to_bytes(&16u8).unwrap());
        assert_eq!(parse("16u8").unwrap(), bcs::to_bytes(&16u8).unwrap());
        // Leading zeros are not silently accepted as numbers.
        assert_eq!(
            parse("007").unwrap(),
            bcs::to_bytes(&b"007".to_vec()).unwrap()
        );
        // A word ending in a type suffix is a string, not a number.
        assert_eq!(
            parse("zou8").unwrap(),
            bcs::to_bytes(&b"zou8".to_vec()).unwrap()
        );
        // Out of range for the declared width.
        assert!(parse("256u8").is_err());
    }

    #[test]
    fn invalid_hex_address_is_an_error() {
        assert!(parse("0xnothex").is_err());
        assert!(parse("0x").is_err());
        // Longer than 32 bytes: an error with a hint, never a truncation.
        assert!(parse(&format!("0x{}", "ab".repeat(33))).is_err());
        // A `0x…::module::Type` tag is not a pure value; it falls through to the
        // string path with a warning (type tags belong in `--type-args`).
        assert_eq!(
            parse("0x1::coin::COIN").unwrap(),
            bcs::to_bytes(&b"0x1::coin::COIN".to_vec()).unwrap()
        );
    }

    #[test]
    fn unbalanced_vector_is_an_error() {
        assert!(parse("[1,2").is_err());
        assert!(parse("[1,2,3").is_err());
        assert!(parse("[[1,2],[3,4]").is_err());
    }

    #[test]
    fn type_tags_are_case_insensitive() {
        assert_eq!(parse("U8:42").unwrap(), bcs::to_bytes(&42u8).unwrap());
        assert_eq!(parse("Bool:TRUE").unwrap(), bcs::to_bytes(&true).unwrap());
        assert_eq!(
            parse("Vector<u8>:1,2").unwrap(),
            bcs::to_bytes(&vec![1u8, 2u8]).unwrap()
        );
        assert_eq!(
            parse("Address:0x403").unwrap(),
            parse("address:0x403").unwrap()
        );
    }

    #[test]
    fn near_miss_type_tags_are_rejected() {
        // Move has no signed/floating types: these are typos, not strings.
        assert!(parse("u65:5").is_err());
        assert!(parse("i64:5").is_err());
        assert!(parse("f64:3.5").is_err());
        assert!(parse("bytes:00ff").is_err());
        assert!(parse("vec<u8>:1,2").is_err());
    }

    #[test]
    fn non_tag_colon_values_stay_strings() {
        // Anything that is not a type tag (URLs, `key:value` data) is a string.
        assert_eq!(
            parse("https://example.com").unwrap(),
            bcs::to_bytes(&b"https://example.com".to_vec()).unwrap()
        );
        assert_eq!(
            parse("k:v").unwrap(),
            bcs::to_bytes(&b"k:v".to_vec()).unwrap()
        );
    }
}
