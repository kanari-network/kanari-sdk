// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

package com.jamesatomc.kanariapp.network

/**
 * Helpers for comparing Move type tags.
 *
 * The node is free to render an address with or without leading zeros
 * (`0x2` vs `0x000...002`), so plain string equality is not reliable when
 * matching an object's `type` against a token type the app knows.
 */

private val LEADING_ZEROS = Regex("0x0+")

/** Lowercase [raw] and drop leading zeros from every `0x...` address in it. */
internal fun normalizeTypeTag(raw: String): String =
    raw.trim().lowercase().replace(LEADING_ZEROS, "0x")

/** True when [a] and [b] denote the same type tag regardless of address padding. */
internal fun typeTagsEqual(a: String, b: String): Boolean =
    normalizeTypeTag(a) == normalizeTypeTag(b)

/**
 * Inner `T` of a `Coin<T>` object type, or null when [objectType] is not a
 * coin type (for example `0x2::coin::Coin<0x2::usd::USD>` -> `0x2::usd::USD`).
 */
internal fun coinInnerType(objectType: String): String? {
    if (!objectType.endsWith(">") || !objectType.contains('<')) return null
    return objectType.substringAfter('<').substringBeforeLast('>')
}

/** True when [objectType] is a `Coin<T>` and `T` is [typeTag]. */
internal fun isCoinOfType(objectType: String, typeTag: String): Boolean =
    coinInnerType(objectType)?.let { typeTagsEqual(it, typeTag) } == true

/** Extract symbol from type tag (last segment after `::`), e.g. `0x2::kanari::KANARI` -> `KANARI` */
internal fun extractSymbol(typeTag: String): String =
    typeTag.trim().substringAfterLast("::").takeIf { it.isNotEmpty() } ?: typeTag