// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module kanari_system::math_tests {
    use kanari_system::math;

    // =================================================================
    // Tests: Min / Max
    // =================================================================
    #[test]
    fun test_min_max_u64() {
        assert!(math::min_u64(10, 20) == 10, 0);
        assert!(math::min_u64(20, 10) == 10, 1);
        assert!(math::min_u64(15, 15) == 15, 2);

        assert!(math::max_u64(10, 20) == 20, 3);
        assert!(math::max_u64(20, 10) == 20, 4);
        assert!(math::max_u64(15, 15) == 15, 5);
    }

    // =================================================================
    // Tests: Rounding (หัวใจสำคัญของ DeFi ป้องกัน Pool ขาดทุน)
    // =================================================================
    #[test]
    fun test_divide_and_round_up() {
        // หารลงตัว ต้องไม่ปัดเพิ่ม
        assert!(math::divide_and_round_up(10, 2) == 5, 0); 
        assert!(math::divide_and_round_up(9, 3) == 3, 1);

        // หารไม่ลงตัว ต้องปัดขึ้นเสมอ
        assert!(math::divide_and_round_up(10, 3) == 4, 2); // 3.333 -> 4
        assert!(math::divide_and_round_up(11, 3) == 4, 3); // 3.666 -> 4
        assert!(math::divide_and_round_up(1, 100) == 1, 4); // 0.01 -> 1
    }

    #[test]
    #[expected_failure(abort_code = 2)] // คาดหวังว่าจะพังด้วย E_DIVIDE_BY_ZERO
    fun test_divide_and_round_up_zero_denominator() {
        math::divide_and_round_up(10, 0);
    }

    // =================================================================
    // Tests: Native Functions (ทศสอบว่า Rust VM รันคณิตศาสตร์ผ่านไหม)
    // =================================================================
    #[test]
    fun test_native_sqrt() {
        assert!(math::sqrt_u64(100) == 10, 0);
        assert!(math::sqrt_u64(144) == 12, 1);
        assert!(math::sqrt_u64(2) == 1, 2); // รูท 2 ปัดเศษลงเหลือ 1
    }

    #[test]
    fun test_native_pow() {
        assert!(math::pow_u64(2, 3) == 8, 0);
        assert!(math::pow_u64(10, 4) == 10000, 1);
        assert!(math::pow_u64(5, 0) == 1, 2); // ยกกำลัง 0 ต้องได้ 1
    }

    #[test]
    fun test_native_mul_div() {
        // (10 * 20) / 4 = 50
        assert!(math::mul_div_u64(10, 20, 4) == 50, 0);
    }
}