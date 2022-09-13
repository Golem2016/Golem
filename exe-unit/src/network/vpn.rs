use std::convert::TryFrom;
use std::ops::Not;
use std::path::PathBuf;
use std::str::FromStr;

use actix::prelude::*;
// use futures::{future, FutureExt, TryFutureExt, TryStreamExt};
use futures::{future, FutureExt, TryFutureExt};

use ya_client_model::NodeId;
use ya_core_model::activity::{self, RpcMessageError, VpnControl, VpnPacket};
use ya_core_model::identity;
use ya_core_model::net::{RegisterVpnEndpoint, VpnEndpointType};
use ya_runtime_api::server::{CreateNetwork, NetworkInterface, RuntimeService};
use ya_service_bus::typed::Endpoint as GsbEndpoint;
use ya_service_bus::{actix_rpc, typed, RpcEndpoint, RpcEnvelope, RpcRawCall};
// use ya_utils_networking::socket::{write_prefix, Endpoint, RxBuffer};
use ya_utils_networking::socket::RxBuffer;
use ya_utils_networking::vpn::network::DuoEndpoint;
use ya_utils_networking::vpn::{common::ntoh, Error as NetError, PeekPacket};
use ya_utils_networking::vpn::{ArpField, ArpPacket, EtherFrame, EtherType, IpPacket, Networks};

use crate::acl::Acl;
use crate::error::Error;
use crate::message::Shutdown;
use crate::network;
use crate::state::Deployment;

pub(crate) async fn start_vpn<R: RuntimeService>(
    acl: Acl,
    service: &R,
    deployment: &Deployment,
) -> crate::Result<Option<Addr<Vpn>>> {
    if !deployment.networking() {
        return Ok(None);
    }

    log::info!("Starting VPN service...");

    // FIXME: allow use of non-default identities
    let node_id = typed::service(identity::BUS_ID)
        .send(identity::Get::ByDefault)
        .await?
        .map_err(|e| Error::Other(format!("failed to retrieve default identity: {e}")))?
        .ok_or_else(|| Error::Other("no default identity set".to_string()))?
        .node_id;

    let networks = deployment
        .networks
        .values()
        .map(TryFrom::try_from)
        .collect::<crate::Result<_>>()?;
    let hosts = deployment.hosts.clone();

    let created = service
        .create_network(CreateNetwork {
            networks,
            hosts,
            interface: NetworkInterface::Vpn as i32,
        })
        .await
        .map_err(|e| Error::Other(format!("initialization error: {:?}", e)))?;

    // let endpoint = match created.endpoint.clone() {
    //     Some(endpoint) => match endpoint {
    //         ya_runtime_api::server::proto::response::create_network::Endpoint::Socket(path) => {
    //             Endpoint::socket(path).await?
    //         }
    //     },
    //     None => return Err(Error::Other("endpoint already connected".into())),
    // };
    // let vpn = Vpn::try_new(node_id, acl, endpoint, deployment.clone())?;

    let endpoint_path = match created.endpoint {
        Some(kind) => match kind {
            ya_runtime_api::server::proto::response::create_network::Endpoint::Socket(path) => {
                PathBuf::from(path)
            }
        },
        None => return Err(Error::Other("endpoint not defined".into())),
    };
    let vpn = Vpn::try_new(node_id, acl, endpoint_path, deployment.clone())?;

    Ok(Some(vpn.start()))
}

pub(crate) struct Vpn {
    default_id: String,
    // TODO: Populate & use ACL
    #[allow(unused)]
    acl: Acl,
    networks: Networks<DuoEndpoint<GsbEndpoint>>,
    // endpoint: Endpoint,
    endpoint_path: PathBuf,
    rx_buf: Option<RxBuffer>,
}

impl Vpn {
    fn try_new(
        node_id: NodeId,
        acl: Acl,
        // endpoint: Endpoint,
        endpoint_path: PathBuf,
        deployment: Deployment,
    ) -> crate::Result<Self> {
        let mut networks = Networks::default();

        deployment
            .networks
            .iter()
            .try_for_each(|(id, net)| networks.add(id.clone(), net.network))?;

        deployment.networks.into_iter().try_for_each(|(id, net)| {
            let network = networks.get_mut(&id).unwrap();
            net.nodes
                .into_iter()
                .try_for_each(|(ip, id)| network.add_node(ip, &id, network::gsb_endpoint))?;
            Ok::<_, NetError>(())
        })?;

        Ok(Self {
            default_id: node_id.to_string(),
            acl,
            networks,
            // endpoint,
            endpoint_path,
            rx_buf: Some(Default::default()),
        })
    }

    fn handle_ip(&mut self, frame: EtherFrame, ctx: &mut Context<Self>) {
        let ip_pkt = IpPacket::packet(frame.payload());
        log::trace!("[vpn] egress packet to {:?}", ip_pkt.dst_address());

        if ip_pkt.is_broadcast() {
            let futs = self
                .networks
                .endpoints()
                .into_iter()
                .map(|e| e.udp.call_raw_as(&self.default_id, frame.as_ref().to_vec()))
                .collect::<Vec<_>>();
            futs.is_empty().not().then(|| {
                let fut = future::join_all(futs).then(|_| future::ready(()));
                ctx.spawn(fut.into_actor(self))
            });
        } else {
            let ip = ip_pkt.dst_address();
            match self.networks.endpoint(ip) {
                Some(endpoint) => self.forward_frame(endpoint, frame, ctx),
                None => log::debug!("[vpn] no endpoint for {ip:?}"),
            }
        }
    }

    fn handle_arp(&mut self, frame: EtherFrame, ctx: &mut Context<Self>) {
        let arp = ArpPacket::packet(frame.payload());
        // forward only IP ARP packets
        if arp.get_field(ArpField::PTYPE) != [8, 0] {
            return;
        }

        let ip = arp.get_field(ArpField::TPA);
        match self.networks.endpoint(ip) {
            Some(endpoint) => self.forward_frame(endpoint, frame, ctx),
            None => log::debug!("[vpn] no endpoint for {ip:?}"),
        }
    }

    fn handle_packet(&mut self, packet: Packet) {
        let network_id = packet.network_id;
        let node_id = packet.caller;
        let data = packet.data.into_boxed_slice();

        // fixme: should requestor be queried for unknown IP addresses instead?
        // read and add unknown node id -> ip if it doesn't exist
        if let Ok(ether_type) = EtherFrame::peek_type(&data) {
            let payload = EtherFrame::peek_payload(&data).unwrap();
            let ip = match ether_type {
                EtherType::Arp => {
                    let pkt = ArpPacket::packet(payload);
                    ntoh(pkt.get_field(ArpField::SPA))
                }
                EtherType::Ip => {
                    let pkt = IpPacket::packet(payload);
                    ntoh(pkt.src_address())
                }
                _ => None,
            };

            if let Some(ip) = ip {
                let _ = self.networks.get_mut(&network_id).map(|network| {
                    if !network.nodes().contains_key(&node_id) {
                        log::debug!("[vpn] adding new node: {} {}", ip, node_id);
                        let _ = network.add_node(ip, &node_id, network::gsb_endpoint);
                    }
                });
            }
        }

        // let mut data = data.into();
        // write_prefix(&mut data);
        //
        // if let Err(e) = self.endpoint.tx.send(Ok(data)) {
        //     log::debug!("[vpn] ingress error: {}", e);
        // }
    }

    fn forward_frame(
        &mut self,
        endpoint: DuoEndpoint<GsbEndpoint>,
        frame: EtherFrame,
        ctx: &mut Context<Self>,
    ) {
        let data: Vec<_> = frame.into();
        log::trace!("[vpn] egress {} b", data.len());

        endpoint
            .udp
            .call_raw_as(&self.default_id, data)
            .map_err(|err| log::debug!("[vpn] call error: {err}"))
            .then(|_| future::ready(()))
            .into_actor(self)
            .spawn(ctx);
    }

    fn register_nodes(&self, ctx: &mut Context<Self>) {
        log::info!("register_nodes called");

        let ids = self
            .networks
            .iter()
            .flat_map(|n| n.nodes().keys().collect::<Vec<_>>())
            .filter_map(|id| NodeId::from_str(id.as_str()).ok())
            .filter(|n| n.to_string() != self.default_id)
            .collect::<Vec<_>>();

        let msg = RegisterVpnEndpoint {
            ids,
            endpoint: VpnEndpointType::Socket(self.endpoint_path.clone()),
        };

        async move {
            log::info!("Registering VPN nodes");

            match typed::service("/net/vpn").send(msg).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(Error::Other(e)),
                Err(e) => Err(Error::from(e)),
            }
        }
        .into_actor(self)
        .map(move |result, _actor, ctx| {
            if let Err(e) = result {
                log::error!("VPN registration error: {e}");
                ctx.stop();
            }
        })
        .wait(ctx);
    }
}

impl Actor for Vpn {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.networks.as_ref().keys().for_each(|net| {
            let actor = ctx.address();
            let net_id = net.clone();
            let vpn_id = activity::exeunit::network_id(&net_id);

            actix_rpc::bind::<VpnControl>(&vpn_id, ctx.address().recipient());
            actix_rpc::bind_raw(&format!("{vpn_id}/raw"), ctx.address().recipient());

            typed::bind_with_caller::<VpnPacket, _, _>(&vpn_id, move |caller, pkt| {
                actor
                    .send(Packet {
                        network_id: net_id.clone(),
                        caller,
                        data: pkt.0,
                    })
                    .then(|sent| match sent {
                        Ok(result) => future::ready(result),
                        Err(err) => future::err(RpcMessageError::Service(err.to_string())),
                    })
            });
        });

        self.register_nodes(ctx);

        // match self.endpoint.rx.take() {
        //     Some(rx) => {
        //         Self::add_stream(rx.map_err(Error::from), ctx);
        //         log::info!("[vpn] service started")
        //     }
        //     None => {
        //         log::error!("[vpn] local endpoint missing");
        //         ctx.stop();
        //     }
        // };

        log::info!("[vpn] service started");
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        log::info!("[vpn] stopping service");

        let networks = self.networks.as_ref().keys().cloned().collect::<Vec<_>>();
        async move {
            for net in networks {
                let vpn_id = activity::exeunit::network_id(&net);
                let _ = typed::unbind(&vpn_id).await;
            }
        }
        .into_actor(self)
        .wait(ctx);

        Running::Stop
    }
}

/// Egress traffic handler (Runtime -> VPN)
impl StreamHandler<crate::Result<Vec<u8>>> for Vpn {
    fn handle(&mut self, result: crate::Result<Vec<u8>>, ctx: &mut Context<Self>) {
        let received = match result {
            Ok(vec) => vec,
            Err(err) => return log::debug!("[vpn] error (egress): {err}"),
        };
        let mut rx_buf = match self.rx_buf.take() {
            Some(buf) => buf,
            None => return log::error!("[vpn] programming error: rx buffer already taken"),
        };

        for packet in rx_buf.process(received) {
            match EtherFrame::try_from(packet) {
                Ok(frame) => match &frame {
                    EtherFrame::Arp(_) => self.handle_arp(frame, ctx),
                    EtherFrame::Ip(_) => self.handle_ip(frame, ctx),
                    frame => log::debug!("[vpn] unimplemented EtherType: {}", frame),
                },
                Err(err) => {
                    match &err {
                        NetError::ProtocolNotSupported(_) => (),
                        _ => log::debug!("[vpn] frame error (egress): {}", err),
                    };
                    continue;
                }
            };
        }

        self.rx_buf.replace(rx_buf);
    }
}

/// Ingress traffic handler (VPN -> Runtime)
impl Handler<RpcRawCall> for Vpn {
    type Result = Result<Vec<u8>, ya_service_bus::Error>;

    fn handle(&mut self, msg: RpcRawCall, _ctx: &mut Self::Context) -> Self::Result {
        let network_id = {
            let mut split = msg.addr.rsplit('/').skip(1);
            match split.next() {
                Some(network_id) => network_id.to_string(),
                None => {
                    return Err(ya_service_bus::Error::GsbBadRequest(
                        "Missing network id".to_string(),
                    ))
                }
            }
        };

        self.handle_packet(Packet {
            network_id,
            caller: msg.caller,
            data: msg.body,
        });

        Ok(Vec::new())
    }
}

impl Handler<Packet> for Vpn {
    type Result = <Packet as Message>::Result;

    fn handle(&mut self, packet: Packet, _ctx: &mut Context<Self>) -> Self::Result {
        self.handle_packet(packet);
        Ok(())
    }
}

impl Handler<RpcEnvelope<VpnControl>> for Vpn {
    type Result = <RpcEnvelope<VpnControl> as Message>::Result;

    fn handle(&mut self, msg: RpcEnvelope<VpnControl>, ctx: &mut Context<Self>) -> Self::Result {
        // if !self.acl.has_access(msg.caller(), AccessRole::Control) {
        //     return Err(AclError::Forbidden(msg.caller().to_string(), AccessRole::Control).into());
        // }

        match msg.into_inner() {
            VpnControl::AddNodes { network_id, nodes } => {
                let network = self.networks.get_mut(&network_id).map_err(Error::from)?;
                for (ip, id) in Deployment::map_nodes(nodes).map_err(Error::from)? {
                    network
                        .add_node(ip, &id, network::gsb_endpoint)
                        .map_err(Error::from)?;
                }

                self.register_nodes(ctx);
            }
            VpnControl::RemoveNodes {
                network_id,
                node_ids,
            } => {
                let network = self.networks.get_mut(&network_id).map_err(Error::from)?;
                node_ids.into_iter().for_each(|id| network.remove_node(&id));
            }
        }
        Ok(())
    }
}

impl Handler<Shutdown> for Vpn {
    type Result = <Shutdown as Message>::Result;

    fn handle(&mut self, msg: Shutdown, ctx: &mut Context<Self>) -> Self::Result {
        log::info!("[vpn] shutting down: {:?}", msg.0);
        ctx.stop();
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "<RpcEnvelope<VpnPacket> as Message>::Result")]
pub(crate) struct Packet {
    pub network_id: String,
    pub caller: String,
    pub data: Vec<u8>,
}
