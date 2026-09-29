//! Omarchy/Hyprland virtual input adapter. The compositor remains the sole
//! authority for accepting virtual devices. No root or uinput access is used.

use std::error::Error;
use std::os::fd::AsFd;
use wayland_client::protocol::{wl_keyboard, wl_pointer, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, delegate_noop};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

#[derive(Default)]
struct RegistryState {
    seat: Option<(u32, u32)>,
    pointer_manager: Option<(u32, u32)>,
    keyboard_manager: Option<(u32, u32)>,
    keymap: Option<(std::os::fd::OwnedFd, u32, u32)>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for RegistryState {
    fn event(
        state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_seat" => state.seat = Some((name, version)),
                "zwlr_virtual_pointer_manager_v1" => state.pointer_manager = Some((name, version)),
                "zwp_virtual_keyboard_manager_v1" => state.keyboard_manager = Some((name, version)),
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for RegistryState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Keymap { format, fd, size } = event {
            state.keymap = Some((fd, u32::from(format), size));
        }
    }
}

delegate_noop!(RegistryState: ignore wl_seat::WlSeat);
delegate_noop!(RegistryState: ignore ZwpVirtualKeyboardManagerV1);
delegate_noop!(RegistryState: ignore ZwpVirtualKeyboardV1);
delegate_noop!(RegistryState: ignore ZwlrVirtualPointerManagerV1);
delegate_noop!(RegistryState: ignore ZwlrVirtualPointerV1);

pub struct VirtualInput {
    connection: Connection,
    queue: EventQueue<RegistryState>,
    state: RegistryState,
    pointer: ZwlrVirtualPointerV1,
    keyboard: ZwpVirtualKeyboardV1,
}

impl VirtualInput {
    pub fn connect() -> Result<Self, Box<dyn Error>> {
        let connection = Connection::connect_to_env()?;
        let display = connection.display();
        let mut queue = connection.new_event_queue();
        let qh = queue.handle();
        let registry = display.get_registry(&qh, ());
        let mut state = RegistryState::default();
        queue.roundtrip(&mut state)?;

        let (seat_name, seat_version) = state.seat.ok_or("no Wayland seat")?;
        let (pointer_name, pointer_version) =
            state.pointer_manager.ok_or("no virtual pointer protocol")?;
        let (keyboard_name, keyboard_version) = state
            .keyboard_manager
            .ok_or("no virtual keyboard protocol")?;
        let seat = registry.bind::<wl_seat::WlSeat, _, _>(seat_name, seat_version.min(9), &qh, ());
        let pointer_manager = registry.bind::<ZwlrVirtualPointerManagerV1, _, _>(
            pointer_name,
            pointer_version.min(2),
            &qh,
            (),
        );
        let keyboard_manager = registry.bind::<ZwpVirtualKeyboardManagerV1, _, _>(
            keyboard_name,
            keyboard_version.min(1),
            &qh,
            (),
        );
        let physical_keyboard = seat.get_keyboard(&qh, ());
        queue.roundtrip(&mut state)?;
        let (fd, format, size) = state
            .keymap
            .take()
            .ok_or("compositor did not provide a keyboard keymap")?;

        let pointer = pointer_manager.create_virtual_pointer(Some(&seat), &qh, ());
        let keyboard = keyboard_manager.create_virtual_keyboard(&seat, &qh, ());
        keyboard.keymap(format, fd.as_fd(), size);
        queue.roundtrip(&mut state)?;
        physical_keyboard.release();
        Ok(Self {
            connection,
            queue,
            state,
            pointer,
            keyboard,
        })
    }

    pub fn motion(&mut self, dx: f64, dy: f64, time_ms: u32) -> Result<(), Box<dyn Error>> {
        self.pointer.motion(time_ms, dx, dy);
        self.pointer.frame();
        self.flush()
    }

    pub fn button(
        &mut self,
        button: u32,
        pressed: bool,
        time_ms: u32,
    ) -> Result<(), Box<dyn Error>> {
        self.pointer.button(
            time_ms,
            button,
            if pressed {
                wl_pointer::ButtonState::Pressed
            } else {
                wl_pointer::ButtonState::Released
            },
        );
        self.pointer.frame();
        self.flush()
    }

    pub fn key(&mut self, key: u32, pressed: bool, time_ms: u32) -> Result<(), Box<dyn Error>> {
        self.keyboard.key(time_ms, key, if pressed { 1 } else { 0 });
        self.flush()
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        self.connection.flush()?;
        self.queue.dispatch_pending(&mut self.state)?;
        Ok(())
    }
}
