use anyhow::{Error, Result, anyhow};
use memprocfs::{FLAG_NOCACHE};
use serde::{Serialize};
use crate::{constants::{game_offsets, player_offsets, unity_offsets}, utils::{Encoding, Vector2}, vmm_wrapper::TarkovVmmProcess};

#[derive(Debug)]
pub struct Player {
    pub ptr: u64,
    pub faction: Faction,
    pub human: bool,
    pub player_type: PlayerType,

    pub health_addr: u64,
    pub rota_addr: u64
}

#[derive(Debug, Serialize)]
pub struct PopulatedPlayer {
    pub faction: Faction,
    pub human: bool,
    pub player_type: PlayerType,
    pub health_status: HealthStatus,
    pub rotation: Vector2
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
pub enum Faction {
    USEC,
    BEAR,
    SCAV
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum PlayerType {
    ClientPlayer,
    MainPlayer,
    NetworkedPlayer
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum HealthStatus {
    UNKNOWN,
    FULL,
    HIGH,
    MEDIUM,
    LOW,
    SPECIAL
}

impl PlayerType {
    fn player_type_from_bytes(p_type_bytes: &[u8]) -> Self {
        match p_type_bytes {
            bytes if bytes.starts_with(b"ClientPlayer") => Self::ClientPlayer,
            bytes if bytes.starts_with(b"LocalPlayer") => Self::MainPlayer,
            _ => Self::NetworkedPlayer
        }
    }
}

impl TryFrom<i32> for Faction {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, anyhow::Error> {
        match value {
            1 => anyhow::Ok(Faction::USEC),
            2 => anyhow::Ok(Faction::BEAR),
            4 => anyhow::Ok(Faction::SCAV),
            _ => Err(anyhow!("Invalid player value {}", value)),
        }
    }
}

impl TryFrom<i32> for HealthStatus {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, anyhow::Error> {
        match value {
            0 | -1 => anyhow::Ok(HealthStatus::UNKNOWN),
            1024 => anyhow::Ok(HealthStatus::FULL),
            2048 => anyhow::Ok(HealthStatus::HIGH),
            4096 => anyhow::Ok(HealthStatus::MEDIUM),
            8192 => anyhow::Ok(HealthStatus::LOW),
            _ => anyhow::Ok(HealthStatus::SPECIAL)
        }
    }
}

impl TarkovVmmProcess<'_> {
    pub fn get_players(&self, game_world_ptr: u64) -> Result<Vec<Player>> {
        //Make this a scatter read since local player and all players don't rely on each other (add benchmarking to see if scatters with just x reads is worth the added over head of the scatter struct)
        let local_player_ptr = self.vmm.mem_read_as::<u64>(game_world_ptr + game_offsets::MAIN_PLAYER, FLAG_NOCACHE)?;
        let main_player = self.get_main_player(local_player_ptr)?;
        let mut all_players = self.get_all_players(game_world_ptr, local_player_ptr)?;
        all_players.push(main_player);

        return Ok(all_players)
    }

    fn get_all_players(&self, game_world_ptr: u64, player_to_ignore: u64) -> Result<Vec<Player>> {
        let players_address = self.vmm.mem_read_as::<u64>(game_world_ptr + game_offsets::ALL_PLAYERS, FLAG_NOCACHE)?;
        let player_count = self.vmm.mem_read_as::<i32>(players_address + unity_offsets::ARRAY_COUNT_OFFSET, 0)?;
        let vec_ptr = self.vmm.mem_read_as::<u64>(players_address + unity_offsets::ARRAY_OFFSET, 0)? + unity_offsets::ARRAY_START;
        let mut player_ptr_vec = self.mem_read_array_into_buffer(vec_ptr, player_count as usize)?;
        player_ptr_vec.retain(|&x| x != 0x0 && x != player_to_ignore);
        
        let mut player_vec = Vec::with_capacity(player_ptr_vec.len());
        for player_ptr in player_ptr_vec {
            let player_bytes = self.get_object_bytes(player_ptr, 64)?;
            let p_type = PlayerType::player_type_from_bytes(&player_bytes);
            
            if let Ok(player) = self.get_player_details(player_ptr, &p_type) {
                player_vec.push(player);
            } else {
                println!("Get Player Details Failed: {:?} {}", p_type, player_ptr)
            }
        }

        return Ok(player_vec);
    }

    fn get_main_player(&self, player_ptr: u64) -> Result<Player> {
        return Ok(self.get_player_details(player_ptr, &PlayerType::MainPlayer)?);
    }

    fn get_player_details(&self, player_ptr: u64, p_type: &PlayerType) -> Result<Player> {
        match p_type {
            PlayerType::ClientPlayer | PlayerType::MainPlayer => {
                let profile_ptr = self.vmm.mem_read_as::<u64>(player_ptr + player_offsets::PROFILE, 0)?;
                let info_ptr = self.vmm.mem_read_as::<u64>(profile_ptr + player_offsets::INFO, 0)?;
                let faction_value = self.vmm.mem_read_as::<i32>(info_ptr + player_offsets::FACTION, 0)?;

                // let group_id_ptr = process.vmm.mem_read_as::<u64>(info_ptr + player_offsets::GROUP_ID, 0)?;
                // let group_id = process.mem_read_string(group_id_ptr, 128, Encoding::UFT8)?;

                let move_context_ptr = self.vmm.mem_read_as::<u64>(player_ptr + player_offsets::MOVEMENT_CONTEXT, 0)?;
                let rotation_addr = move_context_ptr + player_offsets::ROTATION;
                self.scatter.prepare_as::<Vector2>(rotation_addr)?;

                return Ok(Player { ptr: player_ptr, faction: Faction::try_from(faction_value)?, human: true, player_type: PlayerType::ClientPlayer, health_addr: 0, rota_addr: rotation_addr });
            },
            PlayerType::NetworkedPlayer => {
                //Can ignore profle stuff since that is just an API call and I can make web client handle that
                let player_controller_ptr = self.vmm.mem_read_as::<u64>(player_ptr + player_offsets::NETWORKED_PLAYER_CONTROLLER, 0)?;
                let health_ptr = self.vmm.mem_read_as::<u64>(player_controller_ptr + player_offsets::NETWORKED_HEALTH_CONTROLLER, 0)?;
                let move_context_ptr = self.mem_read_chain(player_controller_ptr, player_offsets::NETWORKED_MOVEMENT_CHAIN)?;
                let is_bot = self.vmm.mem_read_as::<bool>(player_ptr + player_offsets::IS_BOT, 0)?;
                
                if !is_bot {
                    let group_id_ptr = self.vmm.mem_read_as::<u64>(player_ptr + player_offsets::NETWORKED_GROUP_ID, 0)?;
                    let group_id = self.mem_read_string(group_id_ptr + unity_offsets::UNITY_UTF8, 128, Encoding::UNICODE)?; //Still needs tested
                }
                
                let faction_value = self.vmm.mem_read_as::<i32>(player_ptr + player_offsets::NETWORKED_FACTION, 0)?;
                let faction = Faction::try_from(faction_value)?;
                
                // Works but the SCAV_{num} isn't unique. Probably fine.
                if faction == Faction::SCAV{
                    let voice_ptr = self.vmm.mem_read_as::<u64>(player_ptr + player_offsets::NETWORKED_VOICE, 0)?;
                    let voice = self.mem_read_string(voice_ptr + unity_offsets::UNITY_UTF8, 128, Encoding::UNICODE)?;
                }
                
                let rotation_addr = move_context_ptr + player_offsets::NETWORKED_ROTATION;
                let health_addr = health_ptr + player_offsets::NETWORKED_HEALTH_VALUE;
                self.scatter.prepare_as::<i32>(health_addr)?;
                self.scatter.prepare_as::<Vector2>(rotation_addr)?;
                
                return Ok(Player { ptr: player_ptr, faction, human: !is_bot, player_type: PlayerType::NetworkedPlayer, health_addr: health_addr, rota_addr: rotation_addr });
            },
        }
    }

    //Hot path
    pub fn populate_player(&self, player: &Player) -> Result<PopulatedPlayer> {
        let health_value = self.scatter.read_as::<i32>(player.health_addr)?;
        let rotation_value = self.scatter.read_as::<Vector2>(player.rota_addr)?;
        let status = HealthStatus::try_from(health_value)?; 

        return Ok(PopulatedPlayer { faction: player.faction, human: player.human, player_type: player.player_type, health_status: status, rotation: rotation_value })
    }
}
