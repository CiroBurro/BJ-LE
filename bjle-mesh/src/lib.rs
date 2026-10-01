use bluer::adv::{Advertisement, Type, AdvertisementHandle};
use bluer::{Adapter, AdapterEvent, Session};
use futures::Stream;
use maplit::btreemap;

pub struct Peer {
    adapter: Adapter,
    handle: Option<AdvertisementHandle>,
    events: Stream<AdapterEvent>
}

impl Peer {
    pub async fn new() -> Result<Peer, bluer::Error> {
        let session = Session::new().await?;
        let adapter = session.default_adapter().await?;

        Ok(Peer {
                adapter: adapter,
                handle: None,
                events: None
        })
    }
    pub async fn send(&mut self, payload: Vec<u8>) -> bluer::Result<()>{
        self.handle = None;

        let adv = Advertisement {
            advertisement_type: Type::Broadcast,
            manufacturer_data: btreemap! { 0xDCBA => payload },
            ..Default::default()
        };

        self.handle = Some(self.adapter.advertise(adv).await?);
        Ok(())
    }

    pub async fn start_listening(&mut self) -> bluer::Result<()>{
        self.events = Some(self.adapter.discover_devices().await?);
        Ok(())
    }

    pub async fn stop_listening(&mut self) -> bluer::Result<()>{
        self.events = None;
        Ok(())
    }

    pub async fn get_data(&mut self) -> Result<()>{
        while let Some(ev) = self.events.next().await {
            if let AdapterEvent::DeviceAdded(addr) = ev {
                let device = self.adapter.device(addr)?;
                let data = device.manufacturer_data().await?;
                if let Some(bytes) = data.get(&0xDCBA) {
                    println!("{:?}" , *bytes);
                }
            }
        }
    }

}
