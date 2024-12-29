#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, Memory, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

// Define memory type
type MemoryType = VirtualMemory<DefaultMemoryImpl>;

// Character Stats
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Stats {
    strength: u32,
    agility: u32,
    intelligence: u32,
    vitality: u32,
    luck: u32,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
enum Stat {
    Strength,
    Agility,
    Intelligence,
    Vitality,
    Luck,
}

// Character Skills
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Skill {
    id: u32,
    name: String,
    damage: u32,
    cooldown: u32,
    element: Element,
    mastery_level: u32,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
enum Element {
    Fire,
    Water,
    Earth,
    Air,
    Light,
    Dark,
}

// Payloads for Character Creation and Training
#[derive(candid::CandidType, Serialize, Deserialize)]
struct CreateCharacterPayload {
    name: String,
    class: CharacterClass,
}

#[derive(candid::CandidType, Serialize, Deserialize)]
struct TrainCharacterPayload {
    character_id: u64,
    stat: Stat,
}

// Payload for Market Listing
#[derive(candid::CandidType, Serialize, Deserialize)]
struct ListCharacterPayload {
    character_id: u64,
    price: u64,
}

#[derive(candid::CandidType, Serialize, Deserialize)]
struct PurchaseCharacterPayload {
    listing_id: u64,
}

// Character Equipment
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Equipment {
    weapon: Option<Item>,
    armor: Option<Item>,
    accessory: Option<Item>,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Item {
    id: u32,
    name: String,
    rarity: Rarity,
    stat_bonus: Stats,
    required_level: u32,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

// Main Character struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Character {
    id: u64,
    owner: String,
    name: String,
    level: u32,
    experience: u64,
    class: CharacterClass,
    stats: Stats,
    skills: Vec<Skill>,
    equipment: Equipment,
    training_history: Vec<TrainingSession>,
    creation_date: u64,
    last_training: u64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
enum CharacterClass {
    Warrior,
    Mage,
    Rogue,
    Cleric,
    Ranger,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct TrainingSession {
    timestamp: u64,
    stat_trained: Stat,
    gain: u32,
}

// Market listing
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct MarketListing {
    id: u64,
    character_id: u64,
    seller: String,
    price: u64,
    listing_date: u64,
    status: ListingStatus,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, PartialEq)]
enum ListingStatus {
    Active,
    Sold,
    Cancelled,
}

// Storage implementation
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static CHARACTER_STORAGE: RefCell<StableBTreeMap<u64, Character, MemoryType>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))
        )
    );

    static MARKET_STORAGE: RefCell<StableBTreeMap<u64, MarketListing, MemoryType>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        )
    );

    static ID_COUNTER: RefCell<Cell<u64, MemoryType>> = RefCell::new(
        Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))), 0)
            .expect("Cannot create counter")
    );
}

// Implementation for Character
impl Storable for Character {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Character {
    const MAX_SIZE: u32 = 2048; // 2KB max size
    const IS_FIXED_SIZE: bool = false;
}

// Implementation for MarketListing
impl Storable for MarketListing {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for MarketListing {
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

// Create a new character using payload
#[ic_cdk::update]
fn create_character(payload: CreateCharacterPayload) -> Result<Character, String> {
    let CreateCharacterPayload { name, class } = payload;
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    let owner = ic_cdk::caller().to_string();
    let current_time = time();

    let base_stats = match class {
        CharacterClass::Warrior => Stats {
            strength: 10,
            agility: 5,
            intelligence: 3,
            vitality: 8,
            luck: 5,
        },
        CharacterClass::Mage => Stats {
            strength: 3,
            agility: 5,
            intelligence: 10,
            vitality: 4,
            luck: 6,
        },
        _ => Stats {
            strength: 5,
            agility: 5,
            intelligence: 5,
            vitality: 5,
            luck: 5,
        },
    };

    let character_id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter
            .borrow_mut()
            .set(current_value + 1)
            .expect("Failed to increment counter");
        current_value
    });

    let character = Character {
        id: character_id,
        owner,
        name,
        level: 1,
        experience: 0,
        class,
        stats: base_stats,
        skills: Vec::new(),
        equipment: Equipment {
            weapon: None,
            armor: None,
            accessory: None,
        },
        training_history: Vec::new(),
        creation_date: current_time,
        last_training: current_time,
    };

    CHARACTER_STORAGE.with(|storage| {
        storage.borrow_mut().insert(character_id, character.clone());
    });

    Ok(character)
}

// Train character stats using payload
#[ic_cdk::update]
fn train_character(payload: TrainCharacterPayload) -> Result<Character, String> {
    let TrainCharacterPayload { character_id, stat } = payload;
    let caller = ic_cdk::caller().to_string();
    let current_time = time();

    CHARACTER_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if let Some(mut character) = storage.get(&character_id) {
            if character.owner != caller {
                return Err("Not the character owner".to_string());
            }

            if current_time - character.last_training < 21600 {
                return Err("Training cooldown not finished".to_string());
            }

            let base_gain = ((current_time ^ character_id) % 3 + 1) as u32;
            let luck_bonus = (character.stats.luck as f32 * 0.1) as u32;
            let total_gain = base_gain + luck_bonus;

            match stat {
                Stat::Strength => character.stats.strength += total_gain,
                Stat::Agility => character.stats.agility += total_gain,
                Stat::Intelligence => character.stats.intelligence += total_gain,
                Stat::Vitality => character.stats.vitality += total_gain,
                Stat::Luck => character.stats.luck += total_gain,
            }

            character.training_history.push(TrainingSession {
                timestamp: current_time,
                stat_trained: stat,
                gain: total_gain,
            });

            character.last_training = current_time;
            storage.insert(character_id, character.clone());
            Ok(character)
        } else {
            Err("Character not found".to_string())
        }
    })
}

// List character on market using payload
#[ic_cdk::update]
fn list_character(payload: ListCharacterPayload) -> Result<MarketListing, String> {
    let ListCharacterPayload { character_id, price } = payload;
    let caller = ic_cdk::caller().to_string();
    let current_time = time();

    CHARACTER_STORAGE.with(|storage| {
        if let Some(character) = storage.borrow().get(&character_id) {
            if character.owner != caller {
                return Err("Not the character owner".to_string());
            }

            let listing_id = ID_COUNTER.with(|counter| {
                let current_value = *counter.borrow().get();
                counter
                    .borrow_mut()
                    .set(current_value + 1)
                    .expect("Failed to increment counter");
                current_value
            });

            let listing = MarketListing {
                id: listing_id,
                character_id,
                seller: caller,
                price,
                listing_date: current_time,
                status: ListingStatus::Active,
            };

            MARKET_STORAGE.with(|storage| {
                storage.borrow_mut().insert(listing_id, listing.clone());
            });

            Ok(listing)
        } else {
            Err("Character not found".to_string())
        }
    })
}

// Purchase character from market using payload
#[ic_cdk::update]
fn purchase_character(payload: PurchaseCharacterPayload) -> Result<Character, String> {
    let PurchaseCharacterPayload { listing_id } = payload;
    let buyer = ic_cdk::caller().to_string();
    let current_time = time();

    MARKET_STORAGE.with(|market_storage| {
        let mut market_storage = market_storage.borrow_mut();
        if let Some(mut listing) = market_storage.get(&listing_id) {
            if listing.status != ListingStatus::Active {
                return Err("Listing is not active".to_string());
            }

            CHARACTER_STORAGE.with(|char_storage| {
                let mut char_storage = char_storage.borrow_mut();
                if let Some(mut character) = char_storage.get(&listing.character_id) {
                    character.owner = buyer;
                    listing.status = ListingStatus::Sold;

                    char_storage.insert(listing.character_id, character.clone());
                    market_storage.insert(listing_id, listing);

                    Ok(character)
                } else {
                    Err("Character not found".to_string())
                }
            })
        } else {
            Err("Listing not found".to_string())
        }
    })
}

// Get character details
#[ic_cdk::query]
fn get_character(character_id: u64) -> Result<Character, String> {
    CHARACTER_STORAGE.with(|storage| {
        storage
            .borrow()
            .get(&character_id)
            .ok_or_else(|| "Character not found".to_string())
    })
}

// Get active market listings
#[ic_cdk::query]
fn get_market_listings() -> Vec<MarketListing> {
    MARKET_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, listing)| matches!(listing.status, ListingStatus::Active))
            .map(|(_, listing)| listing)
            .collect()
    })
}

// Export Candid interface
ic_cdk::export_candid!();
