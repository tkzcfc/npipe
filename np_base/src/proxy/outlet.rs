pub enum OutletProxyType {
    TCP,
    UDP,
}

struct Outlet {
    outlet_proxy_type: OutletProxyType,
}

impl Outlet {
    pub fn new(outlet_proxy_type: OutletProxyType)  -> Self{
        Self {
            outlet_proxy_type
        }
    }
    pub async fn start(&self) {

    }

    pub async fn stop(&mut self) {

    }
}