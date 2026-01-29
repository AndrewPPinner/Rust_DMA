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
    pub networked_movement_chain: [u64; 2]
}

include!("generated_player_offsets.rs");
//Create procmacro that builds these via a json unispect dump pulls from lones C# file