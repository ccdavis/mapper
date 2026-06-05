use rand::Rng;

use super::TerrainGenerator;

impl TerrainGenerator {
    pub(super) fn generate_ocean_name(&mut self, _index: usize) -> String {
        let prefixes = [
            "Azure", "Cerulean", "Sapphire", "Mystic", "Crystal", "Eternal", "Whispering",
        ];
        let suffixes = ["Sea", "Ocean", "Deep", "Abyss", "Waters", "Expanse", "Bay"];
        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
        let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];
        format!("{} {}", prefix, suffix)
    }

    pub(super) fn generate_mountain_name(&mut self, index: usize) -> String {
        let prefixes = ["Mount", "Mt.", "Peak"];
        let first_parts = [
            "Storm", "Iron", "Snow", "Thunder", "Eagle", "Wolf", "Dragon", "Crystal", "Shadow",
            "Silver", "Golden", "Frost", "Wind", "Cloud", "Stone", "Red",
        ];
        let second_parts = [
            "horn", "crest", "spire", "ridge", "tooth", "peak", "crown", "fang", "head", "point",
            "top", "summit", "needle", "wall",
        ];
        let suffixes = ["Mountains", "Range", "Peaks", "Heights", "Alps", "Highlands"];

        // Ensure variety by using index to influence selection
        let prefix_idx = (index + self.rng.gen_range(0..3)) % prefixes.len();
        let first_idx = (index * 7 + self.rng.gen_range(0..4)) % first_parts.len();
        let second_idx = (index * 5 + self.rng.gen_range(0..3)) % second_parts.len();

        if self.rng.gen_bool(0.4) {
            // Sometimes just use a suffix for the range
            let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];
            format!(
                "The {}{} {}",
                first_parts[first_idx], second_parts[second_idx], suffix
            )
        } else {
            format!(
                "{} {}{}",
                prefixes[prefix_idx], first_parts[first_idx], second_parts[second_idx]
            )
        }
    }

    pub(super) fn generate_forest_name(&mut self, _index: usize) -> String {
        let adjectives = [
            "Whispering", "Ancient", "Enchanted", "Dark", "Silver", "Golden", "Misty",
        ];
        let nouns = [
            "Woods", "Forest", "Grove", "Thicket", "Woodland", "Glade", "Copse",
        ];
        let adj = adjectives[self.rng.gen_range(0..adjectives.len())];
        let noun = nouns[self.rng.gen_range(0..nouns.len())];
        format!("{} {}", adj, noun)
    }

    pub(super) fn generate_swamp_name(&mut self, _index: usize) -> String {
        let adjectives = [
            "Murky", "Fetid", "Misty", "Black", "Forgotten", "Cursed", "Silent",
        ];
        let nouns = ["Marsh", "Swamp", "Bog", "Fen", "Mire", "Wetlands", "Quagmire"];
        let adj = adjectives[self.rng.gen_range(0..adjectives.len())];
        let noun = nouns[self.rng.gen_range(0..nouns.len())];
        format!("{} {}", adj, noun)
    }

    pub(super) fn generate_city_name(&mut self, index: usize) -> String {
        let prefixes = [
            "New", "Port", "Fort", "Saint", "North", "South", "East", "West", "Old", "",
        ];
        let first_parts = [
            "Oak", "River", "Lake", "Hill", "Green", "White", "Black", "Gold", "Silver", "Spring",
            "Summer", "Winter", "Mill", "Fair", "Clear", "Bright",
        ];
        let second_parts = [
            "haven", "bridge", "vale", "crest", "shore", "field", "gate", "wells", "cross", "wood",
            "meadow", "ridge", "view", "hill", "brook",
        ];
        let city_suffixes = [
            "ton", "ville", "burg", "shire", "ford", "mouth", "stead", "ham", "thorpe",
        ];
        let city_types = [" City", " Town", "", "", ""]; // Sometimes add City/Town

        // Use index to ensure variety
        let prefix_chance = self.rng.gen_bool(0.4);
        let first_idx = (index * 3 + self.rng.gen_range(0..4)) % first_parts.len();
        let second_idx = (index * 5 + self.rng.gen_range(0..3)) % second_parts.len();

        let base_name = if self.rng.gen_bool(0.6) {
            // Compound name with suffix
            let suffix = city_suffixes[(index * 7 + self.rng.gen_range(0..2)) % city_suffixes.len()];
            format!(
                "{}{}{}",
                first_parts[first_idx], second_parts[second_idx], suffix
            )
        } else {
            // Two-part name
            format!("{}{}", first_parts[first_idx], second_parts[second_idx])
        };

        let with_prefix = if prefix_chance {
            let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
            if prefix.is_empty() {
                base_name
            } else {
                format!("{} {}", prefix, base_name)
            }
        } else {
            base_name
        };

        // Add City/Town suffix for clarity
        let city_type = city_types[self.rng.gen_range(0..city_types.len())];
        format!("{}{}", with_prefix, city_type)
    }

    pub(super) fn generate_road_name(&mut self, index: usize) -> String {
        let descriptors = [
            "King's",
            "Queen's",
            "Merchant's",
            "Old",
            "Ancient",
            "Royal",
            "Imperial",
            "Trade",
            "Coastal",
            "Mountain",
            "Forest",
            "Valley",
            "Pioneer",
            "Settler's",
            "Hunter's",
            "Pilgrim's",
        ];
        // Use index to ensure variety
        let desc_idx = (index * 3 + self.rng.gen_range(0..4)) % descriptors.len();
        descriptors[desc_idx].to_string()
    }

    pub(super) fn generate_river_name(&mut self, _index: usize) -> String {
        let prefixes = ["River", "The"];
        let names = [
            "Silverflow",
            "Clearwater",
            "Rushing",
            "Serpent",
            "Crystal",
            "Moonwater",
            "Swift",
        ];
        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
        let name = names[self.rng.gen_range(0..names.len())];

        if prefix == "The" {
            format!("{} {} River", prefix, name)
        } else {
            format!("{} {}", name, prefix)
        }
    }

    pub(super) fn generate_bridge_name(&mut self, index: usize) -> String {
        let prefixes = [
            "Old", "New", "Great", "High", "Stone", "Iron", "Wooden", "Ancient",
        ];
        let middles = [
            "River", "Creek", "Valley", "Canyon", "Gorge", "Falls", "Rapids", "Mill",
        ];

        // Always make it clear it's a bridge
        let prefix_idx = (index * 5 + self.rng.gen_range(0..3)) % prefixes.len();
        let middle_idx = (index * 3 + self.rng.gen_range(0..2)) % middles.len();

        format!("{} {} Bridge", prefixes[prefix_idx], middles[middle_idx])
    }
}
