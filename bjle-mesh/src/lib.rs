use bluer::adv::{Advertisement, AdvertisementHandle, Type};
use bluer::{Adapter, AdapterEvent, DiscoveryFilter, DiscoveryTransport, Session};
use futures::{Stream, StreamExt};
use maplit::btreemap;

pub struct Peer {
    adapter: Adapter,
    handle: Option<AdvertisementHandle>,
    // Use a boxed stream for the adapter events
    events: Option<Box<dyn Stream<Item = AdapterEvent> + Unpin>>,
}

impl Peer {
    pub async fn new() -> Result<Peer, bluer::Error> {
        let session = Session::new().await?;
        let adapter = session.default_adapter().await?;
        // Ensure the adapter is powered
        adapter.set_powered(true).await?;

        Ok(Peer {
            adapter,
            handle: None,
            events: None,
        })
    }

    pub async fn send(&mut self, payload: Vec<u8>) -> bluer::Result<()> {
        self.handle = None;

        let adv = Advertisement {
            advertisement_type: Type::Broadcast,
            manufacturer_data: btreemap! { 0xDCBA => payload },
            ..Default::default()
        };

        self.handle = Some(self.adapter.advertise(adv).await?);
        Ok(())
    }

    pub async fn start_listening(&mut self) -> bluer::Result<()> {
        if self.events.is_some() {
            return Ok(());
        }

        self.adapter
            .set_discovery_filter(DiscoveryFilter {
                transport: DiscoveryTransport::Le,
                duplicate_data: true,
                ..Default::default()
            })
            .await?;
        let stream = self.adapter.discover_devices_with_changes().await?;
        self.events = Some(Box::new(stream));
        Ok(())
    }

    pub async fn stop_listening(&mut self) -> bluer::Result<()> {
        self.events = None;
        Ok(())
    }

    pub async fn get_data(&mut self) -> bluer::Result<()> {
        // Borrow the stream so cancelling this future does not drop discovery.
        let adapter_events = match self.events.as_mut() {
            Some(stream) => stream,
            None => return Ok(()),
        };

        while let Some(ev) = adapter_events.next().await {
            if let AdapterEvent::DeviceAdded(addr) = ev {
                let device = self.adapter.device(addr)?;

                // DeviceAdded also reports property changes on this discovery stream.
                if let Some(md) = device.manufacturer_data().await? {
                    if let Some(data) = md.get(&0xDCBA) {
                        println!("Received manufacturer data for {:?}: {:?}", addr, data);
                    }
                }
            }
        }
        Ok(())
    }
}
