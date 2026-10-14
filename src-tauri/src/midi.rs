use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};

use crate::router::MidiRouter;
use crate::types::{MidiDeviceInfo, RoutedEvent};

/// Native MIDI Input Engine handling device discovery, auto-detection of the Akai MPK mini,
/// and low-latency event routing.
pub struct MidiEngine {
    connection: Option<MidiInputConnection<()>>,
    current_device_name: Arc<Mutex<Option<String>>>,
    event_sender: Sender<RoutedEvent>,
    router: Arc<MidiRouter>,
}

impl MidiEngine {
    /// Create a new `MidiEngine` with a crossbeam channel sender for routed events.
    pub fn new(event_sender: Sender<RoutedEvent>) -> Self {
        Self {
            connection: None,
            current_device_name: Arc::new(Mutex::new(None)),
            event_sender,
            router: Arc::new(MidiRouter::new()),
        }
    }

    /// Create a new `MidiEngine` with custom router configuration.
    pub fn with_router(event_sender: Sender<RoutedEvent>, router: MidiRouter) -> Self {
        Self {
            connection: None,
            current_device_name: Arc::new(Mutex::new(None)),
            event_sender,
            router: Arc::new(router),
        }
    }

    /// Enumerates all currently available MIDI input ports on the system.
    pub fn list_devices() -> Result<Vec<MidiDeviceInfo>, String> {
        let midi_in = MidiInput::new("ministation-enumerator")
            .map_err(|e| format!("Failed to initialize MIDI input: {e}"))?;

        let ports = midi_in.ports();
        let mut devices = Vec::with_capacity(ports.len());

        for (index, port) in ports.iter().enumerate() {
            let name = midi_in
                .port_name(port)
                .unwrap_or_else(|_| format!("MIDI Port {index}"));
            let is_akai = Self::is_akai_device_name(&name);

            devices.push(MidiDeviceInfo {
                index,
                name,
                is_akai,
            });
        }

        Ok(devices)
    }

    /// Checks whether a port name matches the Akai MPK series.
    pub fn is_akai_device_name(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.contains("mpk mini") || lower.contains("mpk") || lower.contains("akai")
    }

    /// Connects to a MIDI device by port index.
    pub fn connect_by_index(&mut self, index: usize) -> Result<String, String> {
        self.disconnect();

        let mut midi_in = MidiInput::new("ministation-listener")
            .map_err(|e| format!("Failed to create MIDI input: {e}"))?;
        midi_in.ignore(Ignore::TimeAndActiveSense);

        let ports = midi_in.ports();
        let port: &MidiInputPort = ports
            .get(index)
            .ok_or_else(|| format!("MIDI port index {index} out of range (total: {})", ports.len()))?;

        let device_name = midi_in
            .port_name(port)
            .unwrap_or_else(|_| format!("MIDI Device {index}"));

        let sender = self.event_sender.clone();
        let router = Arc::clone(&self.router);

        let connection = midi_in
            .connect(
                port,
                "ministation-input-port",
                move |_timestamp, message, _| {
                    if let Some(routed) = router.route_raw(message) {
                        let _ = sender.try_send(routed);
                    }
                },
                (),
            )
            .map_err(|e| format!("Failed to connect to MIDI device '{device_name}': {e}"))?;

        self.connection = Some(connection);
        *self.current_device_name.lock().unwrap() = Some(device_name.clone());

        Ok(device_name)
    }

    /// Connects to a MIDI device whose name contains the specified query.
    pub fn connect_by_name(&mut self, query: &str) -> Result<String, String> {
        let devices = Self::list_devices()?;
        let query_lower = query.to_lowercase();

        let target = devices
            .iter()
            .find(|d| d.name.to_lowercase().contains(&query_lower))
            .ok_or_else(|| format!("No MIDI device matching query: '{query}'"))?;

        self.connect_by_index(target.index)
    }

    /// Automatically discovers and connects to the best available MIDI device:
    /// 1. First priority: Connected Akai MPK mini / Akai device.
    /// 2. Second priority: First available MIDI controller port.
    /// Returns `Ok(Some(device_name))` if connected, or `Ok(None)` if no MIDI devices are plugged in.
    pub fn auto_connect(&mut self) -> Result<Option<String>, String> {
        let devices = Self::list_devices()?;
        if devices.is_empty() {
            self.disconnect();
            return Ok(None);
        }

        // Priority 1: Akai device
        if let Some(akai) = devices.iter().find(|d| d.is_akai) {
            let name = self.connect_by_index(akai.index)?;
            return Ok(Some(name));
        }

        // Priority 2: First available device
        if let Some(first) = devices.first() {
            let name = self.connect_by_index(first.index)?;
            return Ok(Some(name));
        }

        Ok(None)
    }

    /// Disconnects from the current MIDI port.
    pub fn disconnect(&mut self) {
        if self.connection.is_some() {
            // Dropping the MidiInputConnection closes the port cleanly
            self.connection = None;
            *self.current_device_name.lock().unwrap() = None;
        }
    }

    /// Returns `true` if currently connected to a MIDI device.
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Returns the name of the currently connected device, if any.
    pub fn current_device(&self) -> Option<String> {
        self.current_device_name.lock().unwrap().clone()
    }
}
