module 0x1::M {

    entry fun place_item(
        kiosk: &mut Kiosk,
        cap: &KioskOwnerCap<Coin>,
        item_id: ID,
        item: Coin
    ) {}
    entry fun burn(
        treasury_cap: &mut TreasuryCap<Coin>,
        coin: Coin
    ) {}

}

