use crate::{Network, Request, Response, Result};
use bytes::Bytes;
use std::convert::Infallible;
use tower::{util::BoxCloneService, ServiceExt};
use tracing::trace;

#[msim_macros::sim_test]
async fn basic_network() -> Result<()> {
    //let _gaurd = crate::init_tracing_for_testing();

    dbg!("-");
    let msg = b"The Way of Kings";

    dbg!("-");
    let network_1 = build_network().unwrap();
    dbg!("-");
    let network_2 = build_network().unwrap();
    dbg!("-");

    let peer = network_1.connect(network_2.local_addr()).await.unwrap();
    dbg!("-");
    let response = network_1
        .rpc(peer, Request::new(msg.as_ref().into()))
        .await
        .unwrap();
    dbg!("-");
    assert_eq!(response.into_body(), msg.as_ref());

    dbg!("-");
    let msg = b"Words of Radiance";
    dbg!("-");
    let peer_id_1 = network_1.peer_id();
    dbg!("-");
    let response = network_2
        .rpc(peer_id_1, Request::new(msg.as_ref().into()))
        .await
        .unwrap();
    dbg!("-");
    assert_eq!(response.into_body(), msg.as_ref());
    Ok(())
}

#[tokio::test]
async fn connect() -> Result<()> {
    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network()?;
    let network_2 = build_network()?;

    let peer = network_1.connect(network_2.local_addr()).await?;
    assert_eq!(peer, network_2.peer_id());

    Ok(())
}

#[tokio::test]
async fn connect_with_peer_id() -> Result<()> {
    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network()?;
    let network_2 = build_network()?;

    let peer = network_1
        .connect_with_peer_id(network_2.local_addr(), network_2.peer_id())
        .await?;
    assert_eq!(peer, network_2.peer_id());

    Ok(())
}

#[tokio::test]
async fn connect_with_invalid_peer_id() -> Result<()> {
    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network()?;
    let network_2 = build_network()?;
    let network_3 = build_network()?;

    // Try to dial network 2, but with network 3's peer id
    network_1
        .connect_with_peer_id(network_2.local_addr(), network_3.peer_id())
        .await
        .unwrap_err();

    Ok(())
}

#[tokio::test]
async fn connect_with_hostname() -> Result<()> {
    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network()?;
    let network_2 = build_network()?;
    let network_3 = build_network()?;

    let peer = network_1
        .connect_with_peer_id(
            ("localhost", network_2.local_addr().port()),
            network_2.peer_id(),
        )
        .await?;
    assert_eq!(peer, network_2.peer_id());

    let peer = network_1
        .connect_with_peer_id(
            format!("localhost:{}", network_3.local_addr().port()),
            network_3.peer_id(),
        )
        .await?;
    assert_eq!(peer, network_3.peer_id());

    Ok(())
}

fn build_network() -> Result<Network> {
    build_network_with_addr("localhost:0")
}

fn build_network_with_addr(addr: &str) -> Result<Network> {
    let network = Network::bind(addr)
        .random_private_key()
        .server_name("test")
        .start(echo_service())?;

    trace!(
        address =% network.local_addr(),
        peer_id =% network.peer_id(),
        "starting network"
    );

    Ok(network)
}

fn echo_service() -> BoxCloneService<Request<Bytes>, Response<Bytes>, Infallible> {
    let handle = move |request: Request<Bytes>| async move {
        trace!("recieved: {}", request.body().escape_ascii());
        let response = Response::new(request.into_body());
        Result::<Response<Bytes>, Infallible>::Ok(response)
    };

    tower::service_fn(handle).boxed_clone()
}

#[tokio::test]
async fn ip6_calling_ip4() -> Result<()> {
    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network_with_addr("[::]:0")?;
    let network_2 = build_network_with_addr("127.0.0.1:0")?;

    let msg = b"The Way of Kings";
    let peer = network_1.connect(network_2.local_addr()).await?;
    let response = network_1
        .rpc(peer, Request::new(msg.as_ref().into()))
        .await?;

    println!("{}", response.body().escape_ascii());

    Ok(())
}

#[tokio::test]
async fn localhost_calling_anyaddr() -> Result<()> {
    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network_with_addr("0.0.0.0:0")?;
    let network_2 = build_network_with_addr("127.0.0.1:0")?;

    let msg = b"The Way of Kings";
    let peer = network_2
        .connect((std::net::Ipv4Addr::LOCALHOST, network_1.local_addr().port()))
        .await?;

    let response = network_2
        .rpc(peer, Request::new(msg.as_ref().into()))
        .await?;

    println!("{}", response.body().escape_ascii());

    let response = network_1
        .rpc(network_2.peer_id(), Request::new(msg.as_ref().into()))
        .await?;

    println!("{}", response.body().escape_ascii());

    Ok(())
}

#[tokio::test]
async fn dropped_connection() -> Result<()> {
    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network()?;
    let network_2 = build_network()?;

    let msg = b"The Way of Kings";
    let peer = network_1.connect(network_2.local_addr()).await?;
    let response = network_1
        .rpc(peer, Request::new(msg.as_ref().into()))
        .await?;

    println!("{}", response.body().escape_ascii());

    let peer = network_1.peer(peer).unwrap();

    drop(network_2);

    peer.rpc(Request::new(msg.as_ref().into()))
        .await
        .unwrap_err();

    Ok(())
}

#[tokio::test]
async fn basic_connectivity_check() -> Result<()> {
    use crate::types::{DisconnectReason, PeerEvent::*};

    let _gaurd = crate::init_tracing_for_testing();

    let network_1 = build_network()?;
    let network_2 = build_network()?;

    let peer_id_1 = network_1.peer_id();
    let peer_id_2 = network_2.peer_id();

    let peer_info_2 = crate::types::PeerInfo {
        peer_id: peer_id_2,
        affinity: crate::types::PeerAffinity::High,
        address: vec![network_2.local_addr().into()],
    };
    let mut subscriber_1 = network_1.0.active_peers.subscribe().0;
    let mut subscriber_2 = network_2.0.active_peers.subscribe().0;

    network_1.known_peers().insert(peer_info_2);

    assert_eq!(NewPeer(peer_id_2), subscriber_1.recv().await?);
    assert_eq!(NewPeer(peer_id_1), subscriber_2.recv().await?);

    network_1.known_peers().remove(&peer_id_2).unwrap();
    network_1.disconnect(peer_id_2)?;

    assert_eq!(
        LostPeer(peer_id_2, DisconnectReason::Requested),
        subscriber_1.recv().await?
    );
    assert_eq!(
        LostPeer(peer_id_1, DisconnectReason::ConnectionLost),
        subscriber_2.recv().await?
    );

    Ok(())
}

// Ensure that when all Network handles are dropped that the network is shutdown
#[tokio::test]
async fn drop_shutdown() -> Result<()> {
    use tokio::sync::mpsc::error::TryRecvError;

    let _gaurd = crate::init_tracing_for_testing();

    let (sender, mut reciever) = tokio::sync::mpsc::channel::<()>(1);

    let service = {
        let handle = move |request: Request<Bytes>| {
            let sender = sender.clone();
            async move {
                let _sender = sender;
                let response = Response::new(request.into_body());
                Result::<Response<Bytes>, Infallible>::Ok(response)
            }
        };

        tower::service_fn(handle)
    };

    let network = Network::bind("localhost:0")
        .random_private_key()
        .server_name("test")
        .start(service)?;

    let network_2 = build_network()?;

    let peer = network_2.connect(network.local_addr()).await?;
    let _response = network_2.rpc(peer, Request::new(Bytes::new())).await?;

    assert_eq!(Err(TryRecvError::Empty), reciever.try_recv());

    let network_ref = network.downgrade();

    // Just check to see if upgrade is successful
    assert!(network_ref.upgrade().is_some());

    drop(network);

    // Now network upgrading should fail
    assert!(network_ref.upgrade().is_none());

    // And the network should eventually be completely stopped
    assert_eq!(None, reciever.recv().await);
    assert_eq!(Err(TryRecvError::Disconnected), reciever.try_recv());

    let err = network_2
        .rpc(peer, Request::new(Bytes::new()))
        .await
        .unwrap_err();

    tracing::info!("err: {err}");

    Ok(())
}
