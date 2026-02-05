pub struct GameOffsets {
    #[cfg_attr(any(), csharp_struct = "GameWorld", csharp_field = "LocationId")]
    pub location_id: u64,

    #[cfg_attr(any(), csharp_struct = "GameWorld", csharp_field = "MainPlayer")]
    pub main_player: u64,

    #[cfg_attr(any(), csharp_struct = "GameWorld", csharp_field = "RegisteredPlayers")]
    pub all_players: u64,
}

include!("generated_game_offsets.rs");
