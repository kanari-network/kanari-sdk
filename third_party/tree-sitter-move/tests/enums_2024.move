module test::enums_2024 {
    public enum Color has drop {
        Red,
        Green { intensity: u8 },
        Blue(u8, u8, u8),
    }

    public fun is_red(c: &Color): bool {
        match (c) {
            Color::Red => true,
            _ => false,
        }
    }
}
