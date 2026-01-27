// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use libp2p::{
    PeerId, Swarm, Transport,
    core::upgrade,
    dcutr,
    futures::StreamExt,
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identify,
    identity::Keypair,
    kad::{self, store::MemoryStore},
    mdns, noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// P2P message types
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum P2PMessage {
    NewTransaction(String), // Serialized transaction
    NewBlock(String),       // Serialized FULL block with transactions
    NewDagVertex(String),   // Serialized DAG vertex for multi-node sync
    BlockRequest(u64, u64), // (height, timestamp) - timestamp makes it unique
    BlockResponse(String),  // Full block data response with transactions
    PeerInfo(PeerInfoMsg),
}

#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct PeerInfoMsg {
    pub height: u64,
    pub peer_id: String,
    pub timestamp: u64, // Add timestamp to make messages unique
}

/// Network behavior combining multiple protocols
#[derive(NetworkBehaviour)]
pub struct KanariBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub dcutr: dcutr::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub relay: relay::Behaviour,
}

pub struct P2PNetwork {
    pub swarm: Swarm<KanariBehaviour>,
    pub topics: P2PTopics,
}

pub struct P2PTopics {
    pub blocks: IdentTopic,
    pub transactions: IdentTopic,
    pub peers: IdentTopic,
    pub dag_vertices: IdentTopic,
}

impl P2PNetwork {
    pub fn new(keypair: Keypair, listen_port: u16, enable_relay_server: bool) -> Result<Self> {
        let local_peer_id = PeerId::from(keypair.public());
        info!("Local peer id: {}", local_peer_id);

        // Create transport
        let transport = tcp::tokio::Transport::default()
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&keypair)?)
            .multiplex(yamux::Config::default())
            .boxed();

        // Create Gossipsub behavior
        let message_id_fn = |message: &gossipsub::Message| {
            let mut hasher = DefaultHasher::new();
            message.data.hash(&mut hasher);
            gossipsub::MessageId::from(hasher.finish().to_string())
        };

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Permissive)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|e| anyhow::anyhow!("Gossipsub config error: {}", e))?;

        let mut gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create gossipsub: {}", e))?;

        // Create topics
        let blocks_topic = IdentTopic::new("kanari/blocks");
        let tx_topic = IdentTopic::new("kanari/transactions");
        let peers_topic = IdentTopic::new("kanari/peers");
        let dag_vertices_topic = IdentTopic::new("kanari/dag_vertices");

        // Subscribe to topics
        gossipsub.subscribe(&blocks_topic)?;
        gossipsub.subscribe(&tx_topic)?;
        gossipsub.subscribe(&peers_topic)?;
        gossipsub.subscribe(&dag_vertices_topic)?;

        // Create mDNS for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

        // Create Kademlia DHT for peer discovery
        let mut kademlia = kad::Behaviour::new(local_peer_id, MemoryStore::new(local_peer_id));

        // Bootstrap Kademlia
        kademlia.set_mode(Some(kad::Mode::Server));

        // Create DCUtR for hole punching (works without relay client for direct connections)
        let dcutr = dcutr::Behaviour::new(local_peer_id);

        // Create Identify protocol for peer information exchange
        let identify = identify::Behaviour::new(identify::Config::new(
            "/kanari/1.0.0".to_string(),
            keypair.public(),
        ));

        // Create Ping for connection keep-alive
        let ping = ping::Behaviour::new(ping::Config::new());

        // Create relay server (will only accept relay requests if configured properly)
        let relay_config = if enable_relay_server {
            relay::Config::default()
        } else {
            // Limit relay to essentially disable it without Option<T>
            relay::Config {
                max_reservations: 0,
                max_circuits: 0,
                ..Default::default()
            }
        };
        let relay = relay::Behaviour::new(local_peer_id, relay_config);

        // Create behavior
        let behaviour = KanariBehaviour {
            gossipsub,
            mdns,
            kademlia,
            dcutr,
            identify,
            ping,
            relay,
        };

        // Create swarm
        let mut swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor()
                .with_idle_connection_timeout(Duration::from_secs(60)),
        );

        // Listen on all interfaces
        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", listen_port);
        swarm.listen_on(listen_addr.parse()?)?;

        Ok(Self {
            swarm,
            topics: P2PTopics {
                blocks: blocks_topic,
                transactions: tx_topic,
                peers: peers_topic,
                dag_vertices: dag_vertices_topic,
            },
        })
    }

    pub fn publish_message(&mut self, msg: P2PMessage) -> Result<()> {
        let topic = match &msg {
            P2PMessage::NewBlock(_)
            | P2PMessage::BlockResponse(_)
            | P2PMessage::BlockRequest(_, _) => &self.topics.blocks,
            P2PMessage::NewTransaction(_) => &self.topics.transactions,
            P2PMessage::PeerInfo(_) => &self.topics.peers,
            P2PMessage::NewDagVertex(_) => &self.topics.dag_vertices,
        };

        let config = bincode::config::standard();
        let data = bincode::encode_to_vec(&msg, config)
            .map_err(|e| anyhow::anyhow!("Failed to encode message: {}", e))?;

        // Publish and handle duplicate gracefully
        match self
            .swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), data)
        {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("Duplicate") {
                    // Duplicate is not an error, just skip silently
                    return Ok(());
                }
                if err_str.contains("InsufficientPeers") {
                     // This is normal when starting up or isolated - just log debug/info
                     tracing::debug!("No peers subscribed to topic yet");
                     return Ok(());
                }
                
                // Log warning but don't fail
                warn!("Publish warning: {}", e);
                Ok(())
            }
        }
    }
}

pub struct P2PEventHandler {
    pub network: P2PNetwork,
    pub message_tx: mpsc::UnboundedSender<P2PMessage>,
    pub outgoing_rx: Option<mpsc::UnboundedReceiver<P2PMessage>>,
    pub peer_store: Option<std::sync::Arc<tokio::sync::Mutex<crate::peer_store::PeerStore>>>,
}

impl P2PEventHandler {
    pub fn new(network: P2PNetwork, message_tx: mpsc::UnboundedSender<P2PMessage>) -> Self {
        Self {
            network,
            message_tx,
            outgoing_rx: None,
            peer_store: None,
        }
    }

    pub fn with_outgoing(mut self, outgoing_rx: mpsc::UnboundedReceiver<P2PMessage>) -> Self {
        self.outgoing_rx = Some(outgoing_rx);
        self
    }

    pub fn with_peer_store(
        mut self,
        peer_store: std::sync::Arc<tokio::sync::Mutex<crate::peer_store::PeerStore>>,
    ) -> Self {
        self.peer_store = Some(peer_store);
        self
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                // Handle swarm events
                Some(event) = self.network.swarm.next() => {
                    self.handle_event(event).await;
                }
                // Handle outgoing messages to publish
                Some(msg) = async {
                    match &mut self.outgoing_rx {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    if let Err(e) = self.network.publish_message(msg) {
                        warn!("Failed to publish outgoing message: {}", e);
                    }
                }
                else => break,
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<KanariBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            SwarmEvent::Behaviour(KanariBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: _,
                message_id: _,
                message,
            })) => {
                let config = bincode::config::standard();
                if let Ok((msg, _)) =
                    bincode::decode_from_slice::<P2PMessage, _>(&message.data, config)
                    && let Err(e) = self.message_tx.send(msg)
                {
                    warn!("Failed to forward P2P message: {}", e);
                }
            }
            SwarmEvent::Behaviour(KanariBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, multiaddr) in peers {
                    info!("Discovered peer: {} at {}", peer_id, multiaddr);
                    self.network
                        .swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                    self.network
                        .swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, multiaddr);
                }
            }
            SwarmEvent::Behaviour(KanariBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _) in peers {
                    info!("Peer expired: {}", peer_id);
                    self.network
                        .swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                ..
            } => {
                info!(
                    "Connection established with {} at {} (total: {})",
                    peer_id,
                    endpoint.get_remote_address(),
                    num_established
                );

                // Save peer to persistent store
                if let Some(peer_store) = &self.peer_store {
                    let mut store = peer_store.lock().await;
                    let addresses = vec![endpoint.get_remote_address().clone()];
                    store.add_peer(peer_id, addresses);

                    // Save to disk (async, ignore errors)
                    if let Err(e) = store.save() {
                        warn!("Failed to save peer store: {}", e);
                    }
                }
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                cause,
                num_established,
                ..
            } => {
                info!(
                    "Connection closed with {} (remaining: {}) - {:?}",
                    peer_id, num_established, cause
                );
            }
            SwarmEvent::Behaviour(KanariBehaviourEvent::Dcutr(dcutr::Event {
                remote_peer_id,
                result,
            })) => match result {
                Ok(_) => {
                    info!("DCUtR hole punching succeeded with {}", remote_peer_id);
                }
                Err(e) => {
                    warn!(
                        "DCUtR hole punching failed with {}: {:?}",
                        remote_peer_id, e
                    );
                }
            },
            SwarmEvent::Behaviour(KanariBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) => {
                info!(
                    "Identified peer {}: protocol {}, agent {}",
                    peer_id, info.protocol_version, info.agent_version
                );
                // Add identified addresses to Kademlia
                for addr in info.listen_addrs {
                    self.network
                        .swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }
            SwarmEvent::Behaviour(KanariBehaviourEvent::Relay(
                relay::Event::ReservationReqAccepted { src_peer_id, .. },
            )) => {
                info!("Relay: Accepted reservation request from {}", src_peer_id);
            }
            SwarmEvent::Behaviour(KanariBehaviourEvent::Relay(
                relay::Event::CircuitReqAccepted {
                    src_peer_id,
                    dst_peer_id,
                },
            )) => {
                info!(
                    "Relay: Accepted circuit from {} to {}",
                    src_peer_id, dst_peer_id
                );
            }
            SwarmEvent::Behaviour(KanariBehaviourEvent::Relay(relay::Event::CircuitClosed {
                src_peer_id,
                dst_peer_id,
                ..
            })) => {
                info!(
                    "Relay: Circuit closed between {} and {}",
                    src_peer_id, dst_peer_id
                );
            }
            _ => {}
        }
    }
}
