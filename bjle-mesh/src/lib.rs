use bluer::adv::{Advertisement, AdvertisementHandle, Type};
use bluer::{Adapter, AdapterEvent, DeviceEvent, DeviceProperty, Session};
use futures::Stream;
use futures::{StreamExt, pin_mut};
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
        // This returns a Stream of AdapterEvent
        let stream = self.adapter.discover_devices().await?;
        self.events = Some(Box::new(stream));
        Ok(())
    }

    pub async fn stop_listening(&mut self) -> bluer::Result<()> {
        self.events = None;
        Ok(())
    }

    pub async fn get_data(&mut self) -> bluer::Result<()> {
        // Take the stream out of the Option, or return an error if not set
        let mut adapter_events = match self.events.take() {
            Some(stream) => stream,
            None => return Ok(()), // or return an error
        };

        while let Some(ev) = adapter_events.next().await {
            if let AdapterEvent::DeviceAdded(addr) = ev {
                let device = self.adapter.device(addr)?;

                // **Key change:** Listen to the device's own event stream
                let device_events = device.events().await?;
                pin_mut!(device_events);

                while let Some(device_event) = device_events.next().await {
                    if let DeviceEvent::PropertyChanged(DeviceProperty::ManufacturerData(md)) =
                        device_event
                    {
                        // `md` is a BTreeMap<u16, Vec<u8>>
                        if let Some(data) = md.get(&0xDCBA) {
                            // `data` is the Vec<u8> payload you want
                            println!("Received manufacturer data for {:?}: {:?}", addr, data);
                            // You can now process `data` as needed.
                        }
                    }
                }
            }
        }
        // Put the stream back if you want to keep listening
        self.events = Some(adapter_events);
        Ok(())
    }
}
