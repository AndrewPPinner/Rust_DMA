use std::sync::Arc;

use anyhow::Result;
use base64::{Engine, engine::general_purpose};
use easy_upnp::{Ipv4Cidr, PortMappingProtocol, UpnpConfig, add_ports};
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use webrtc::{api::APIBuilder, data_channel::{RTCDataChannel, data_channel_state::RTCDataChannelState}, ice_transport::ice_server::RTCIceServer, peer_connection::{configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription}};

pub enum ServerType {
    WebRTC,
    WebSockets
}

pub struct Connection {
    inner: InnerConnection
}

enum InnerConnection {
    WebRTC(WebRTCConnection),
    WebSocket(WebSocketConnection)
}

pub struct WebRTCConnection {
    data_channel: Arc<RTCDataChannel>
}
pub struct WebSocketConnection {}

impl Connection {
    pub async fn send<T>(&self, value: &T) -> Result<()> where T: Serialize
    {
        match &self.inner {
            InnerConnection::WebRTC(web_rtc_conn) => {
                if let Ok(data) = serde_json::to_string(&value) {
                    web_rtc_conn.data_channel.send_text(data).await?;
                }
                return Ok(());
            },
            InnerConnection::WebSocket(web_socket_connection) => todo!(),
        }
    }
    pub fn is_open(&self) -> bool {
        match &self.inner {
            InnerConnection::WebRTC(web_rtc_conn) => {
                return web_rtc_conn.data_channel.ready_state() == RTCDataChannelState::Open;
            },
            InnerConnection::WebSocket(web_socket_connection) => todo!(),
        }
    }
}

impl ServerType {
    pub async fn start_server(&self, sender_channel: &Sender<Connection>) -> Result<()> {
        match self {
            ServerType::WebRTC => {
                open_tcp_signalling_port();
                handle_web_rtc(sender_channel).await?;
                return Ok(());
            },
            ServerType::WebSockets => todo!(),
        }
    }
}

async fn handle_web_rtc(sender_channel: &Sender<Connection>) -> Result<()> {
    let web_rtc_api = APIBuilder::new().build();
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let server = tiny_http::Server::http("0.0.0.0:54124").unwrap();
        println!("listening on {}", server.server_addr());
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
            sender_channel.send(conn).await?;


            let response = tiny_http::Response::from_string(base64);
            rq.respond(response)?;
            println!("Signaling complete");
        }

        return Ok(());
}

fn open_tcp_signalling_port() {
    let config_specific_address = UpnpConfig {
        address: Some(Ipv4Cidr::from_str("192.168.1.106/24").unwrap()),
            port: 54124,
            protocol: PortMappingProtocol::TCP,
            duration: 3600,
            comment: "Webserver alternative".to_string(),
    };
    for _ in add_ports([config_specific_address]) {}
}