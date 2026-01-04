pub const LOCATION: u64 = 0x870;
pub const PROFILE: u64 = 0x900;
pub const INFO: u64 = 0x48;
pub const FACTION: u64 = 0x48;
pub const IS_BOT: u64 = 0xA0;
pub const GROUP_ID: u64 = 0x50;
pub const MOVEMENT_CONTEXT: u64 = 0x60;
pub const ROTATION: u64 = 0xC8;

pub const NETWORKED_PLAYER_CONTROLLER: u64 = 0x28;
pub const NETWORKED_HEALTH_CONTROLLER: u64 = 0xE8;
pub const NETWORKED_HEALTH_VALUE: u64 = 0x10;
pub const NETWORKED_MOVEMENT_CHAIN: [u64; 2] = [0xD8, 0x98];
pub const NETWORKED_ROTATION: u64 = 0x20;
pub const NETWORKED_FACTION: u64 = 0x94;
pub const NETWORKED_GROUP_ID: u64 = 0x80;
pub const NETWORKED_VOICE: u64 = 0x40;

//Create procmacro that builds these via a json unispect dump pulls from lones C# file