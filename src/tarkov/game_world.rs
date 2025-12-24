use std::{sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, thread};

use anyhow::{Result, anyhow};
use memprocfs::{FLAG_NOCACHE};
use crate::{constants::{game_offsets, player_offsets, unity_offsets}, utils::Encoding, vmm_wrapper::TarkovVmmProcess};

#[repr(C)]
pub struct GameObjectManager {
    _pad0: [u8; 0x20],
    pub last_active_node: u64,  // 0x20
    pub active_nodes: u64,      // 0x28
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct LinkedListObject {
    prev: u64,
    next: u64,
    object: u64
}

pub struct GameWorld {
    pub game_world_ptr: u64,
    pub map_name: String
}

impl TarkovVmmProcess<'_> {
    pub fn get_game_world(&self) -> Result<GameWorld> {
        let game_sig = self.get_game_address()?;
        let rva = self.vmm.mem_read_as::<i32>(game_sig + 3, FLAG_NOCACHE)?;
        let game_ptr = self.vmm.mem_read_as::<u64>(game_sig + 7 + rva as u64, FLAG_NOCACHE)?;
        let game_world = self.vmm.mem_read_as::<GameObjectManager>(game_ptr, 0)?;

        let first = self.vmm.mem_read_as::<LinkedListObject>(game_world.active_nodes, 0)?;
        let last = self.vmm.mem_read_as::<LinkedListObject>(game_world.last_active_node, 0)?;

        //Benchmark this, vs straight traverse vs block in place (async)
        let (send, rec) = mpsc::channel::<Result<GameWorld>>();
        let cancel = Arc::new(AtomicBool::new(false));
        
        let result = thread::scope(|t| {
            
            let fw_cancel = Arc::clone(&cancel);
            let fw_send = send.clone();
            t.spawn(move || { 
                let res = self.find_game_world_fw(first, last, &fw_cancel);
                if res.is_ok() {
                    fw_cancel.store(true, Ordering::Relaxed);
                }
                let _ = fw_send.send(res);
            });

            let bw_cancel = Arc::clone(&cancel);
            let bw_send = send.clone();
            t.spawn(move || {
                let res = self.find_game_world_bw(last, first, &bw_cancel);
                if res.is_ok() {
                    bw_cancel.store(true, Ordering::Relaxed);
                }
                let _ = bw_send.send(res);
            });

            //Should consider waiting for first success instead of first result
            drop(send);
            return rec.recv()?;
        });

        Ok(result?)
    }   

    fn find_game_world_fw(&self, mut current: LinkedListObject, last: LinkedListObject, cancel: &AtomicBool) -> Result<GameWorld> {
        while current.object != last.object && !cancel.load(Ordering::Relaxed) {
            if let Ok(game_world) = self.parse_game_world(&current) {
                println!("Found on forward");
                return Ok(game_world);
            }
            current = self.vmm.mem_read_as::<LinkedListObject>(current.next, 0)?;
        }
        return Err(anyhow!("Game world not found! (Forward)"));
    }

    fn find_game_world_bw(&self, mut current: LinkedListObject, last: LinkedListObject, cancel: &AtomicBool) -> Result<GameWorld> {
        while current.object != last.object && !cancel.load(Ordering::Relaxed) {
            if let Ok(game_world) = self.parse_game_world(&current) {
                println!("Found on forward");
                return Ok(game_world);
            }
            current = self.vmm.mem_read_as::<LinkedListObject>(current.prev, 0)?;
        }
        return Err(anyhow!("Game world not found! (Forward)"));
    }

    fn parse_game_world(&self, current: &LinkedListObject) -> Result<GameWorld> {
        let object_name_ptr = self.vmm.mem_read_as::<u64>(current.object + unity_offsets::GAME_OBJECT_NAME, FLAG_NOCACHE)?;
        let object_name = self.mem_read_string(object_name_ptr, 64, Encoding::UFT8)?;
        
        if !object_name.contains("GameWorld") {
            return Err(anyhow!("Not Found"));
        }
        
        let local_world_ptr = self.mem_read_chain(current.object, unity_offsets::GAME_WORLD_CHAIN)?;
        let mut map_ptr = self.vmm.mem_read_as::<u64>(local_world_ptr + game_offsets::LOCATION_ID, 0)?;
        if map_ptr == 0x0 {
            let local_player = self.vmm.mem_read_as::<u64>(local_world_ptr + game_offsets::MAIN_PLAYER, 0)?;
            map_ptr = self.vmm.mem_read_as::<u64>(local_player + player_offsets::LOCATION, 0)?;
        }

        let map_name = self.mem_read_string(map_ptr + unity_offsets::UNITY_UTF8, 128, Encoding::UNICODE)?;
        if map_name == "hideout" {
            return Err(anyhow!("Found Hideout, skip"))
        }

        return Ok(GameWorld{ game_world_ptr: local_world_ptr, map_name: map_name })
    }

    fn get_game_address(&self) -> Result<u64> {
        let unity_process = self.vmm.map_module(true, true)?.into_iter()
        .find(|x| x.name == "UnityPlayer.dll").ok_or(anyhow!("UnityPlayer.dll not found"))?;

        let signature = "48 89 05 ?? ?? ?? ?? 48 83 C4 ?? C3 33 C9";
        let bytes: Vec<&str> = signature.split_whitespace().collect();
        let mut pattern: Vec<u8> = Vec::new();
        let mut mask: Vec<u8> = Vec::new();
        
        for byte_str in bytes {
            if byte_str == "??" {
                pattern.push(0x00);
                mask.push(0xFF); // Skip this byte (wildcard)
            } else {
                pattern.push(u8::from_str_radix(byte_str, 16)?);
                mask.push(0x00); // Don't skip (exact match)
            }
        }
        
        let mut search = self.vmm.search(
            unity_process.va_base, 
            unity_process.va_base + unity_process.image_size as u64, 
            1,
            0
        )?;
        
        // Add the search term with skipmask
        search.add_search_ex(&pattern, Some(&mask), 1)?;
        search.start();
        let results = search.result();
        
        if results.result.len() > 0 {
        return Ok(results.result[0].0);
        } else {
            return  Err(anyhow!("Signature look up failed"));
        }
    }
}