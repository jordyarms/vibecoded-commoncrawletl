/// Toronto postal code prefixes (FSA - Forward Sortation Area).
/// All Toronto postal codes start with M.
pub const TORONTO_POSTAL_PREFIXES: &[&str] = &[
    "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9",
];

/// GTA postal code prefixes by region (L prefix codes).
pub const GTA_POSTAL_PREFIXES: &[&str] = &[
    // Mississauga
    "L4T", "L4W", "L4X", "L4Y", "L4Z", "L5A", "L5B", "L5C", "L5E", "L5G",
    "L5H", "L5J", "L5K", "L5L", "L5M", "L5N", "L5R", "L5S", "L5T", "L5V", "L5W",
    // Brampton
    "L6P", "L6R", "L6S", "L6T", "L6V", "L6W", "L6X", "L6Y", "L6Z", "L7A",
    // Markham
    "L3P", "L3R", "L3S", "L3T", "L6B", "L6C", "L6E",
    // Vaughan
    "L4H", "L4J", "L4K", "L4L", "L6A",
    // Richmond Hill
    "L4B", "L4C", "L4E", "L4S",
    // Oakville
    "L6H", "L6J", "L6K", "L6L", "L6M",
    // Burlington
    "L7L", "L7M", "L7N", "L7P", "L7R", "L7S", "L7T",
    // Pickering
    "L1V", "L1W", "L1X", "L1Y",
    // Ajax
    "L1S", "L1T", "L1Z",
    // Whitby
    "L1M", "L1N", "L1P", "L1R",
    // Oshawa
    "L1G", "L1H", "L1J", "L1K", "L1L",
    // Newmarket/Aurora
    "L3X", "L3Y", "L4G",
];

/// Bounding box: [south_lat, north_lat, west_lon, east_lon]
/// Note: Toronto longitudes are negative (west), we store absolute values.
pub struct BoundingBox {
    pub south: f64,
    pub north: f64,
    pub west: f64,
    pub east: f64,
}

impl BoundingBox {
    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        lat >= self.south && lat <= self.north && lon >= self.west && lon <= self.east
    }
}

/// Toronto core bounding box.
pub const TORONTO_BBOX: BoundingBox = BoundingBox {
    south: 43.58,
    north: 43.86,
    west: -79.64,
    east: -79.10,
};

/// Greater Toronto Area bounding box.
pub const GTA_BBOX: BoundingBox = BoundingBox {
    south: 43.40,
    north: 44.30,
    west: -80.20,
    east: -78.80,
};

/// Locality names for Toronto neighborhoods and GTA cities.
pub const LOCALITY_NAMES: &[(&str, f64)] = &[
    // Toronto proper (high confidence)
    ("toronto", 0.90),
    ("north york", 0.90),
    ("scarborough", 0.90),
    ("etobicoke", 0.90),
    ("east york", 0.90),
    // "york" removed — matches New York, York UK, Yorkshire, etc.
    ("downtown toronto", 0.95),
    ("midtown toronto", 0.95),
    // Toronto neighborhoods
    ("the annex", 0.85),
    ("kensington market", 0.90),
    ("queen west", 0.85),
    ("king west", 0.85),
    ("liberty village", 0.85),
    ("parkdale", 0.85),
    ("leslieville", 0.85),
    ("the beaches", 0.85),
    ("the beach", 0.85),
    ("riverdale", 0.80),
    ("cabbagetown", 0.85),
    ("rosedale", 0.80),
    ("yorkville", 0.80),
    ("the junction", 0.80),
    ("high park", 0.85),
    ("bloor west village", 0.85),
    ("roncesvalles", 0.85),
    ("danforth", 0.85),
    ("greektown toronto", 0.85),
    // "little italy", "chinatown" removed — exist in many cities
    ("little portugal", 0.80),
    ("distillery district", 0.90),
    ("st lawrence market", 0.90),
    ("harbourfront", 0.85),
    // "waterfront", "entertainment district", "financial district" removed — too generic
    ("bay street toronto", 0.75),
    ("dundas square", 0.90),
    ("yonge-dundas", 0.90),
    ("cne grounds", 0.90),
    ("exhibition place", 0.90),
    ("nathan phillips square", 0.95),
    ("mel lastman square", 0.95),
    // GTA cities (lower confidence — could be other cities with same name)
    ("mississauga", 0.85),
    ("brampton", 0.85),
    ("markham", 0.85),
    ("vaughan", 0.85),
    ("richmond hill", 0.80),
    ("oakville", 0.85),
    ("burlington", 0.80),
    ("hamilton ontario", 0.80),
    ("pickering", 0.75),
    ("ajax ontario", 0.80),
    ("whitby", 0.80),
    ("oshawa", 0.85),
    ("newmarket", 0.80),
    ("aurora ontario", 0.80),
    ("king city", 0.80),
    ("caledon", 0.80),
    ("milton", 0.80),
    ("halton hills", 0.80),
    ("georgetown ontario", 0.80),
    ("stouffville", 0.85),
    ("woodbridge", 0.80),
    ("maple ontario", 0.75),
    ("thornhill", 0.85),
    ("unionville", 0.85),
    ("port credit", 0.85),
    ("streetsville", 0.85),
    ("meadowvale", 0.80),
    ("malton", 0.80),
    ("bramalea", 0.85),
];
