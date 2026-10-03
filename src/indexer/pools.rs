/// Identifies a mining pool from the coinbase script data (hex encoded).
/// Returns the pool name or None if unknown.
pub fn identify_pool(coinbase_hex: &str) -> Option<String> {
    // Decode hex to get the raw coinbase script bytes
    let bytes = hex::decode(coinbase_hex).unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).to_lowercase();

    // Known pool signatures — matched against coinbase script content
    let signatures: &[(&str, &str)] = &[
        ("foundry usa",         "Foundry USA"),
        ("antpool",             "AntPool"),
        ("f2pool",              "F2Pool"),
        ("viabtc",              "ViaBTC"),
        ("binance",             "Binance Pool"),
        ("braiins",             "Braiins Pool"),
        ("slush",               "Braiins Pool"),
        ("luxor",               "Luxor"),
        ("mara pool",           "MARA Pool"),
        ("marathon",            "MARA Pool"),
        ("riot",                "Riot Blockchain"),
        ("cleanspark",          "CleanSpark"),
        ("spiderpool",          "SpiderPool"),
        ("poolin",              "Poolin"),
        ("btc.com",             "BTC.com"),
        ("btccom",              "BTC.com"),
        ("1thash",              "1THash"),
        ("ultimus",             "Ultimus Pool"),
        ("sbicrypto",           "SBI Crypto"),
        ("sbi crypto",          "SBI Crypto"),
        ("emcd",                "EMCD Pool"),
        ("secpool",             "SecPool"),
        ("rawpool",             "RawPool"),
        ("sigmapool",           "SigmaPool"),
        ("titanpool",           "Titan Pool"),
        ("ocean",               "OCEAN"),
        ("public pool",         "Public Pool"),
        ("demand",              "Demand Pool"),
    ];

    for (pattern, name) in signatures {
        if text.contains(pattern) {
            return Some(name.to_string());
        }
    }

    None
}
