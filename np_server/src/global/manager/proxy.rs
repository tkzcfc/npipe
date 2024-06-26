use crate::global::manager::tunnel::Tunnel;
use log::error;
use np_base::proxy::inlet::{Inlet, InletProxyType};
use np_base::proxy::outlet::Outlet;
use np_base::proxy::{OutputFuncType, ProxyMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ProxyManager {
    outlets: Arc<Mutex<HashMap<u32, Outlet>>>,
    inlets: Arc<Mutex<HashMap<u32, Inlet>>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            outlets: Arc::new(Mutex::new(HashMap::new())),
            inlets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub async fn sync_tunnels(&self, tunnels: &Vec<Tunnel>) {
        self.outlets.lock().await.retain(|key: &u32, _| {
            tunnels
                .iter()
                .any(|tunnel| tunnel.enabled == 1 && tunnel.id == *key && tunnel.sender == 0)
        });
        self.inlets.lock().await.retain(|key: &u32, _| {
            tunnels
                .iter()
                .any(|tunnel| tunnel.enabled == 1 && tunnel.id == *key && tunnel.receiver == 0)
        });

        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled == 1 && tunnel.sender == 0)
        {
            if !self.outlets.lock().await.contains_key(&tunnel.id) {
                let tunnel_id = tunnel.id;
                let this_machine = tunnel.receiver == tunnel.sender;
                let inlets = self.inlets.clone();

                let outlet_output: OutputFuncType = Arc::new(move |message: ProxyMessage| {
                    let inlets = inlets.clone();
                    Box::pin(async move {
                        if this_machine {
                            if let Some(inlet) = inlets.lock().await.get(&tunnel_id) {
                                inlet.input(message).await;
                            }
                        } else {
                        }
                    })
                });
                self.outlets
                    .lock()
                    .await
                    .insert(tunnel.id, Outlet::new(outlet_output));
            }
        }

        for tunnel in tunnels
            .iter()
            .filter(|tunnel| tunnel.enabled == 1 && tunnel.receiver == 0)
        {
            if !self.inlets.lock().await.contains_key(&tunnel.id) {
                match tunnel.endpoint.parse::<SocketAddr>() {
                    Err(err) => {
                        let endpoint = tunnel.endpoint.clone();
                        error!("unresolvable address({endpoint}) err: {err}");
                    }
                    Ok(output_addr) => {
                        let tunnel_id = tunnel.id;
                        let this_machine = tunnel.receiver == tunnel.sender;
                        let outlets = self.outlets.clone();

                        let inlet_output: OutputFuncType =
                            Arc::new(move |message: ProxyMessage| {
                                let outlets = outlets.clone();
                                Box::pin(async move {
                                    if this_machine {
                                        if let Some(outlet) = outlets.lock().await.get(&tunnel_id) {
                                            outlet.input(message).await;
                                        }
                                    } else {
                                    }
                                })
                            });

                        let mut inlet = Inlet::new(InletProxyType::TCP, output_addr);
                        if let Err(err) = inlet
                            .start(tunnel.source.clone().into(), inlet_output)
                            .await
                        {
                            let source = tunnel.source.clone();
                            error!("inlet({source}) start error: {err}");
                        } else {
                            self.inlets.lock().await.insert(tunnel.id, inlet);
                        }
                    }
                }
            }
        }
    }
}
