use crate::global::manager::tunnel::Tunnel;
use log::error;
use np_base::proxy::inlet::{Inlet, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ProxyManager {
    outlets: Arc<RwLock<HashMap<u32, Outlet>>>,
    inlets: Arc<RwLock<HashMap<u32, Inlet>>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            outlets: Arc::new(RwLock::new(HashMap::new())),
            inlets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn sync_tunnels(&self, tunnels: &Vec<Tunnel>) {
        self.outlets.write().await.retain(|key: &u32, _| {
            tunnels
                .iter()
                .any(|tunnel| tunnel.enabled == 1 && tunnel.id == *key && tunnel.sender == 0)
        });
        self.inlets.write().await.retain(|key: &u32, _| {
            tunnels
                .iter()
                .any(|tunnel| tunnel.enabled == 1 && tunnel.id == *key && tunnel.receiver == 0)
        });

        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled == 1 && tunnel.sender == 0)
        {
            if !self.outlets.read().await.contains_key(&tunnel.id) {
                let tunnel_id = tunnel.id;
                let this_machine = tunnel.receiver == tunnel.sender;
                let inlets = self.inlets.clone();

                let outlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(inlet) = inlets.read().await.get(&tunnel_id) {
                                inlet.input(message).await;
                            }
                        } else {
                        }
                    })
                });
                self.outlets
                    .write()
                    .await
                    .insert(tunnel.id, Outlet::new(outlet_output));
            }
        }

        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled == 1 && tunnel.receiver == 0)
        {
            if !self.inlets.read().await.contains_key(&tunnel.id) {
                let tunnel_id = tunnel.id;
                let this_machine = tunnel.receiver == tunnel.sender;
                let outlets = self.outlets.clone();

                let inlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let outlets = outlets.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(outlet) = outlets.read().await.get(&tunnel_id) {
                                outlet.input(message).await;
                            }
                        } else {
                        }
                    })
                });

                // let mut inlet = Inlet::new(InletProxyType::UDP, tunnel.endpoint.clone());
                let mut inlet = Inlet::new(InletProxyType::TCP, tunnel.endpoint.clone());
                if let Err(err) = inlet
                    .start(tunnel.source.clone().into(), inlet_output)
                    .await
                {
                    let source = tunnel.source.clone();
                    error!("inlet({source}) start error: {err}");
                } else {
                    self.inlets.write().await.insert(tunnel.id, inlet);
                }
            }
        }
    }
}
