use std::{io::Write, sync::Arc};

use anyhow::{Result};
use base64::{Engine, engine::general_purpose};
use bytes::Bytes;
use easy_upnp::{Ipv4Cidr, PortMappingProtocol, UpnpConfig, add_ports};
use serde::Serialize;
use tokio::{sync::mpsc::{self, Sender}};
use webrtc::{api::APIBuilder, data, data_channel::{RTCDataChannel, data_channel_state::RTCDataChannelState}, ice_transport::ice_server::RTCIceServer, peer_connection::{configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription}};

pub enum ServerType {
    WebRTC,
    WebSockets,
    SSE
}

pub struct Connection {
    inner: InnerConnection
}

enum InnerConnection {
    WebRTC(WebRTCConnection),
    WebSocket(WebSocketConnection),
    SSE(SSEConnection)
}

pub struct WebRTCConnection {
    data_channel: Arc<RTCDataChannel>
}
pub struct WebSocketConnection {}
pub struct SSEConnection {
    writer_channel: Sender<Bytes>
}

impl Connection {
    pub async fn send<T>(&self, value: &T) -> Result<()> where T: Serialize
    {
        match &self.inner {
            InnerConnection::WebRTC(web_rtc_conn) => {
                //Could look into flatbuffers in the future if needed
                if let Ok(data) = rmp_serde::to_vec(value) {
                    let bytes = Bytes::from(data);
                    web_rtc_conn.data_channel.send(&bytes).await?;
                }
                return Ok(());
            },
            InnerConnection::WebSocket(web_socket_connection) => todo!(),
            InnerConnection::SSE(sse_connection) => {
                if let Ok(data) = rmp_serde::to_vec(value) {
                    let bytes = Bytes::from(data);
                    sse_connection.writer_channel.send(bytes).await?;
                }
                return Ok(());
            },
        }
    }
    pub fn is_open(&self) -> bool {
        match &self.inner {
            InnerConnection::WebRTC(web_rtc_conn) => {
                return web_rtc_conn.data_channel.ready_state() == RTCDataChannelState::Open;
            },
            InnerConnection::WebSocket(web_socket_connection) => todo!(),
            InnerConnection::SSE(sseconnection) => todo!(),
        }
    }
}

impl ServerType {
    pub async fn start_server(&self, sender_channel: &Sender<Connection>) -> Result<()> {
        match self {
            ServerType::WebRTC => {
                open_tcp_port();
                start_rtc_signal_server(sender_channel).await?;
                return Ok(());
            },
            ServerType::WebSockets => todo!(),
            ServerType::SSE => {
                open_tcp_port();
                start_sse_server(sender_channel).await?;
                return Ok(());
            }
        }
    }
}

async fn start_sse_server(sender_channel: &Sender<Connection>) -> Result<()> {
    let server = tiny_http::Server::http("0.0.0.0:54124").unwrap();
    
    for rq in server.incoming_requests() {
        println!("sse request recieved");
        let (tx, mut rx) = mpsc::channel(10);
        let mut writer = rq.into_writer();
        writer.write_all(
            b"HTTP/1.1 200 OK\r\n\
              Content-Type: text/event-stream\r\n\
              Cache-Control: no-cache\r\n\
              Connection: keep-alive\r\n\r\n"
        )?;
        writer.flush()?;

        let conn = Connection {
            inner: InnerConnection::SSE(
                SSEConnection { writer_channel: tx }
            )
        };

        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                if writer.write_all(b"data: ").is_err()
                    || writer.write_all(&data).is_err()
                    || writer.write_all(b"\n\n").is_err() || writer.flush().is_err() {
                    break;
                }
            }
            drop(rx);
        });
        
        if let Err(_) = sender_channel.send(conn).await {
            println!("Writer connection broken");
            continue;
        }
        tokio::task::yield_now().await;
        println!("sse request connfigured");
    }
    return Ok(());
}

async fn start_rtc_signal_server(sender_channel: &Sender<Connection>) -> Result<()> {
    let web_rtc_api = APIBuilder::new().build();
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let server = tiny_http::Server::http("0.0.0.0:54124").unwrap();
        for mut rq in server.incoming_requests() {
            let mut client_sdp = String::new();
            rq.as_reader().read_to_string(&mut client_sdp)?;
            let decoded = general_purpose::STANDARD.decode(client_sdp)?;
            let request_offer = serde_json::from_str::<RTCSessionDescription>(std::str::from_utf8(&decoded)?)?;

            let peer = web_rtc_api.new_peer_connection(config.clone()).await?;
            let data_channel = peer.create_data_channel("updates", None).await?;

            peer.set_remote_description(request_offer).await?;

            let answer = peer.create_answer(None).await?;
            peer.set_local_description(answer).await?;

            let mut gather_complete = peer.gathering_complete_promise().await;
            let _ = gather_complete.recv().await;

            
            let answer = match peer.local_description().await {
                Some(x) => x,
                None => {
                    println!("Couldn't load local desc");
                    let response = tiny_http::Response::from_string("Couldn't esstablish connection.");
                    rq.respond(response)?;
                    continue
                },
            }; 

            let desc_json = serde_json::to_string(&answer)?;
            let base64 = general_purpose::STANDARD.encode(desc_json);
            let conn = Connection {
                inner: InnerConnection::WebRTC(WebRTCConnection {
                    data_channel: data_channel,
                }),
            };
            
            if let Err(err) = sender_channel.send(conn).await {
                let response = tiny_http::Response::from_string(err.to_string());
                rq.respond(response)?;
                continue;
            }

            let response = tiny_http::Response::from_string(base64);
            rq.respond(response)?;
            println!("Signaling complete");
            tokio::task::yield_now().await;
        }

    return Ok(());
}

fn open_tcp_port() {
    let config_specific_address = UpnpConfig {
        address: Some(Ipv4Cidr::from_str("192.168.1.106/24").unwrap()),
            port: 54124,
            protocol: PortMappingProtocol::TCP,
            duration: 3600,
            comment: "Webserver alternative".to_string(),
    };
    for _ in add_ports([config_specific_address]) {}
}