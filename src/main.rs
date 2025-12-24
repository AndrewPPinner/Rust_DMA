mod tarkov;
mod constants;
mod vmm_wrapper;
mod utils;
mod server;


use std::{thread};
use anyhow::Result;
use memprocfs::{FLAG_NOCACHE, Vmm};
use tokio::{join, sync::mpsc, time::{sleep}};

use crate::{server::{Connection, ServerType}, tarkov::players::PopulatedPlayer, vmm_wrapper::TarkovVmmProcess};

#[tokio::main]
async fn main() -> Result<()> {
    //Need to handle looping until game process found. That way you don't have to wait to open until game starts
    let (player_tx, mut player_rx) = tokio::sync::watch::channel::<Vec<PopulatedPlayer>>(Vec::with_capacity(10));
    let (data_channel_tx, mut data_channel_rx) = mpsc::channel(10);

    let reader_thread = tokio::task::spawn_blocking(move || -> Result<(), anyhow::Error> {
        let vmm = Vmm::new("C:\\Users\\andpp\\code\\Rust_DMA\\src\\vmm.dll", &vec!["-device", "fpga"])?;
        let process = vmm.process_from_name("EscapeFromTarkov.exe")?;
        
        let unity_base = process.get_module_base("UnityPlayer.dll")?;
        let tarkov_process = TarkovVmmProcess { vmm: process, unity_base: unity_base, scatter: process.mem_scatter(FLAG_NOCACHE)? };
    
        let game_world = tarkov_process.get_game_world()?;
        println!("GameWorld Found {} | Map {}", game_world.game_world_ptr, game_world.map_name);
        let players = tarkov_process.get_players(game_world.game_world_ptr)?;
        loop {
            tarkov_process.scatter.execute()?;
            //Get new players as they join
            //Skip players if they are dead
            let mut populated_players: Vec<PopulatedPlayer> = Vec::with_capacity(players.len());
            for p in &players {
                populated_players.push(tarkov_process.populate_player(&p)?);
            }
            if let Err(err) = player_tx.send(populated_players) {
                println!("Send errored: {}", err);
                continue;
            }
        }
    });

    //Handle exits better
    //Figure out why sometimes, when a new client is connected, they don't get data...
    let broadcast_thread = thread::spawn(move || -> Result<(), anyhow::Error> {
        let async_thread = 
            tokio::runtime::Builder::new_current_thread()
            .enable_all().build()?;

        async_thread.block_on(async {
            let mut connections = Vec::<Connection>::with_capacity(5);
            let delay_dur = tokio::time::Duration::from_millis((1000 + 60 / 2) / 60 as u64);
            loop {
                sleep(delay_dur).await;
                if let Err(err) = player_rx.changed().await {
                    println!("Shit done broken: {}", err);
                    sleep(tokio::time::Duration::from_millis(1000)).await;
                    continue;
                }
                let populated_players = player_rx.borrow_and_update();
                if let Ok(c) = data_channel_rx.try_recv() {
                    connections.retain(|x| x.is_open());
                    connections.push(c);
                    println!("Total connections: {}", connections.len());
                }

                for conn in &connections {
                    if let Err(err) = conn.send::<Vec<PopulatedPlayer>>(populated_players.as_ref()).await {
                        println!("Something broke: {}", err)
                        //Probably should close if err indicates conn closed
                    }
                }
            }
        });
        return Ok(());
    });

    //Check config and either start WebRTC or WebSocket or NONE
    let server_thread = tokio::spawn(async move {
        ServerType::WebRTC.start_server(&data_channel_tx).await?;
        Ok::<(), anyhow::Error>(())
    });

    let res = join!(reader_thread, server_thread);
    println!("broadcast_thread result: {:?}", broadcast_thread);
    println!("res: {:?}", res);

    return Ok(())
}