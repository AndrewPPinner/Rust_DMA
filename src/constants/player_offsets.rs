const LOCATION: u64 = 0x870;
const PROFILE: u64 = 0x900;
const INFO: u64 = 0x48;
const FACTION: u64 = 0x48;
const GROUP_ID: u64 = 0x50;
const MOVEMENT_CONTEXT: u64 = 0x60;
const ROTATION: u64 = 0xC8;

const NETWORKED_IS_BOT: u64 = 0xA0;
const NETWORKED_PLAYER_CONTROLLER: u64 = 0x28;
const NETWORKED_HEALTH_CONTROLLER: u64 = 0xE8;
const NETWORKED_HEALTH_VALUE: u64 = 0x10;
const NETWORKED_MOVEMENT_CHAIN: [u64; 2] = [0xD8, 0x98];
const NETWORKED_ROTATION: u64 = 0x20;
const NETWORKED_FACTION: u64 = 0x94;
const NETWORKED_GROUP_ID: u64 = 0x80;
const NETWORKED_VOICE: u64 = 0x40;

pub struct PlayerOffsets {
    #[cfg_attr(any(), csharp_struct = "Player", csharp_field = "Location")]
    pub location: u64,
    #[cfg_attr(any(), csharp_struct = "Player", csharp_field = "Profile")]
    pub profile: u64,
    #[cfg_attr(any(), csharp_struct = "Player", csharp_field = "MovementContext")]
    pub movement_context: u64,
    #[cfg_attr(any(), csharp_struct = "Profile", csharp_field = "Info")]
    pub info: u64,
    #[cfg_attr(any(), csharp_struct = "PlayerInfo", csharp_field = "Side")]
    pub faction: u64,
    #[cfg_attr(any(), csharp_struct = "PlayerInfo", csharp_field = "GroupId")]
    pub group_id: u64,
    #[cfg_attr(any(), csharp_struct = "MovementContext", csharp_field = "_rotation")]
    pub rotation: u64,
    
    #[cfg_attr(any(), csharp_struct = "ObservedPlayerView", csharp_field = "IsAI")]
    pub networked_is_bot: u64,
    #[cfg_attr(any(), csharp_struct = "ObservedPlayerView", csharp_field = "ObservedPlayerController")]
    pub networked_player_controller: u64,
    #[cfg_attr(any(), csharp_struct = "ObservedPlayerController", csharp_field = "HealthController")]
    pub networked_health_controller: u64,
    #[cfg_attr(any(), csharp_struct = "ObservedHealthController", csharp_field = "HealthStatus")]
    pub networked_health_value: u64,
    #[cfg_attr(any(), csharp_struct = "ObservedPlayerStateContext", csharp_field = "Rotation")]
    pub networked_rotation: u64,
    #[cfg_attr(any(), csharp_struct = "ObservedPlayerView", csharp_field = "Side")]
    pub networked_faction: u64,
    #[cfg_attr(any(), csharp_struct = "ObservedPlayerView", csharp_field = "Voice")]
    pub networked_voice: u64,
    
    #[cfg_attr(any(), is_chain = true, csharp_struct = "ObservedPlayerController | ObservedPlayerMovementController", csharp_field = "MovementController | ObservedPlayerStateContext")]
    pub networked_movement_chain: [u64; 2],
    pub networked_group_id: u64
}

pub const PLAYER_OFFSETS: PlayerOffsets = PlayerOffsets {
    
    location: LOCATION,
    profile: PROFILE,
    info: INFO,
    faction: FACTION,
    group_id: GROUP_ID,
    movement_context: MOVEMENT_CONTEXT,
    rotation: ROTATION,
    
    networked_is_bot: NETWORKED_IS_BOT,
    networked_player_controller: NETWORKED_PLAYER_CONTROLLER,
    networked_health_controller: NETWORKED_HEALTH_CONTROLLER,
    networked_health_value: NETWORKED_HEALTH_VALUE,
    networked_movement_chain: NETWORKED_MOVEMENT_CHAIN,
    networked_rotation: NETWORKED_ROTATION,
    networked_faction: NETWORKED_FACTION,
    networked_group_id: NETWORKED_GROUP_ID,
    networked_voice: NETWORKED_VOICE,
};

//Create procmacro that builds these via a json unispect dump pulls from lones C# file