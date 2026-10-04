//! ระบบ types หลัก: address / coin denomination / gas math

use kanari_types::address::Address;
use kanari_types::gas_coin::GasModule;

#[test]
fn system_addresses_parse() {
    for addr in [
        Address::DEV_ADDRESS,
        Address::DAO_ADDRESS,
        Address::STD_ADDRESS,
        Address::KANARI_SYSTEM_ADDRESS,
    ] {
        Address::parse_to_account_address(addr)
            .unwrap_or_else(|_| panic!("system address ต้อง parse ได้: {addr}"));
    }
}

#[test]
fn invalid_address_is_rejected() {
    assert!(Address::parse_to_account_address("not-an-address").is_err());
    assert!(Address::parse_to_account_address("0xZZZ").is_err());
}

#[test]
fn kanari_to_mist_exact_math() {
    assert_eq!(GasModule::parse_kanari_to_mist("1").unwrap(), 1_000_000_000);
    assert_eq!(
        GasModule::parse_kanari_to_mist("12.5").unwrap(),
        12_500_000_000
    );
    assert_eq!(GasModule::parse_kanari_to_mist("0.000000001").unwrap(), 1);
    // เกิน 9 ทศนิยมต้อง reject (ห้าม float math แอบปัด)
    assert!(GasModule::parse_kanari_to_mist("1.0000000001").is_err());
    assert!(GasModule::parse_kanari_to_mist("-1").is_err());
    assert!(GasModule::parse_kanari_to_mist("").is_err());
}

#[test]
fn mist_format_roundtrip() {
    assert_eq!(GasModule::format_mist_to_kanari(12_500_000_000), "12.5");
    assert_eq!(GasModule::format_mist_to_kanari(1_000_000_000), "1");
    let mist = GasModule::parse_kanari_to_mist("42.123456789").unwrap();
    assert_eq!(GasModule::format_mist_to_kanari(mist), "42.123456789");
}

#[test]
fn genesis_supply_constants_sane() {
    assert_eq!(GasModule::TOTAL_SUPPLY_MIST, 11_000_000_000_000_000);
    assert_eq!(GasModule::gas_to_mist(1), GasModule::MIST_PER_GAS);
    assert_eq!(GasModule::mist_to_gas(GasModule::MIST_PER_GAS), 1);
}
