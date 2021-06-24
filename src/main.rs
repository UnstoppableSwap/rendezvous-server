mod transport;

use crate::transport::authenticate_and_multiplex;
use anyhow::Result;
use libp2p::core::identity::ed25519::SecretKey;
use libp2p::dns::TokioDnsConfig;
use libp2p::futures::StreamExt;
use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent};
use libp2p::rendezvous::{Config, Rendezvous};
use libp2p::tcp::TokioTcpConfig;
use libp2p::PeerId;
use libp2p::{identity, rendezvous, Swarm};
use libp2p::{NetworkBehaviour, Transport};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(long = "secret-key", help = "32 byte string", parse(try_from_str = parse_secret_key))]
    pub secret_key: SecretKey,
    #[structopt(long = "port")]
    pub port: u16,
}

fn parse_secret_key(s: &str) -> Result<SecretKey> {
    let bytes = s.to_string().into_bytes();
    let secret_key = SecretKey::from_bytes(bytes)?;
    Ok(secret_key)
}

#[derive(Debug)]
enum Event {
    Rendezvous(rendezvous::Event),
    Identify(IdentifyEvent),
}

impl From<rendezvous::Event> for Event {
    fn from(event: rendezvous::Event) -> Self {
        Event::Rendezvous(event)
    }
}

impl From<IdentifyEvent> for Event {
    fn from(event: IdentifyEvent) -> Self {
        Event::Identify(event)
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false)]
#[behaviour(out_event = "Event")]
struct Behaviour {
    identify: Identify,
    rendezvous: Rendezvous,
}

#[tokio::main]
async fn main() {
    let cli = Cli::from_args();

    let identity = identity::Keypair::Ed25519(cli.secret_key.into());

    let peer_id = PeerId::from(identity.public());

    let tcp_with_dns = TokioDnsConfig::system(TokioTcpConfig::new().nodelay(true)).unwrap();

    let transport = authenticate_and_multiplex(tcp_with_dns.boxed(), &identity).unwrap();

    let identify = Identify::new(IdentifyConfig::new(
        "rendezvous/1.0.0".to_string(),
        identity.public(),
    ));
    let rendezvous = Rendezvous::new(identity, Config::default());

    let mut swarm = Swarm::new(
        transport,
        Behaviour {
            identify,
            rendezvous,
        },
        peer_id,
    );

    println!("peer id: {}", swarm.local_peer_id());

    swarm
        .listen_on(format!("/ip4/0.0.0.0/tcp/{}", cli.port).parse().unwrap())
        .unwrap();

    loop {
        let event = swarm.next().await;
        println!("swarm event: {:?}", event);
    }
}
