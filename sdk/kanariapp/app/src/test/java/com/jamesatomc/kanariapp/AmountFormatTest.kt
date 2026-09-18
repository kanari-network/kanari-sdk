package com.jamesatomc.kanariapp.ui.components

import org.junit.Assert.*
import org.junit.Test

/**
 * Exact balance display: full precision, no Double, trailing zeros trimmed.
 * decimals = 9 (Mist) unless noted.
 */
class AmountFormatTest {

    @Test
    fun zero_shows_plain_zero() {
        assertEquals("0", formatAmountExact(0L, 9))
    }

    @Test
    fun dust_is_visible_not_zero() {
        assertEquals("0.000000001", formatAmountExact(1L, 9))
        assertEquals("0.0000001", formatAmountExact(100L, 9))
    }

    @Test
    fun round_amounts_trim_zeros() {
        assertEquals("1", formatAmountExact(1_000_000_000L, 9))
        assertEquals("10.5", formatAmountExact(10_500_000_000L, 9))
        assertEquals("10999989", formatAmountExact(10_999_989_000_000_000L, 9))
    }

    @Test
    fun huge_values_keep_full_precision() {
        // Long.MAX_VALUE Mist: a Double formatter silently drops Mist here.
        assertEquals("9223372036.854775807", formatAmountExact(Long.MAX_VALUE, 9))
    }

    @Test
    fun zero_decimals_shows_plain_int() {
        assertEquals("42", formatAmountExact(42L, 0))
    }

    @Test
    fun negative_is_defensive_not_crash() {
        assertEquals("-1", formatAmountExact(-1_000_000_000L, 9))
    }
}
