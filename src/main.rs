extern crate x11rb;

use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::render::{ConnectionExt as RenderConnectionExt, Directformat, Pictforminfo};
use x11rb::protocol::xproto::{
    BackPixmap, ConfigureWindowAux, ConnectionExt as XprotoConnectionExt, EventMask,
    Gravity, StackMode,
};
use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

fn main() {
    let mut wol = WoL::new();
    wol.init_background();
}

struct WoL {
    xcb: x11rb::rust_connection::RustConnection,
    screen_num: usize,
    root: x11rb::protocol::xproto::Window,
    window: x11rb::protocol::xproto::Window,

    offset_x: i16,
    offset_y: i16,
    width: u16,
    height: u16,
}

impl WoL {
    fn new() -> WoL {
        let (xcb, screen_num) = x11rb::connect(None).unwrap();
        let screen = &xcb.setup().roots[screen_num];
        let root = screen.root;
        let width = screen.width_in_pixels;
        let height = screen.height_in_pixels;

        WoL {
            xcb,
            screen_num,
            root,
            window: 0,

            offset_x: 0,
            offset_y: 0,
            width,
            height,
        }
    }

    fn screen(&mut self) -> &x11rb::protocol::xproto::Screen {
        &self.xcb.setup().roots[self.screen_num]
    }

    fn init_background(&mut self) {
        self.window = self.xcb.generate_id().unwrap();

        let screen = self.screen();
        let depth = screen.root_depth;
        let visual = screen.root_visual;

        // Window parameters
        let swa = x11rb::protocol::xproto::CreateWindowAux::new()
            .background_pixmap(BackPixmap::PARENT_RELATIVE)
            .background_pixel(screen.white_pixel)
            .border_pixmap(0)
            .border_pixel(0)
            .bit_gravity(Gravity::BIT_FORGET)
            .win_gravity(Gravity::BIT_FORGET)
            .override_redirect(1)
            .event_mask(EventMask::STRUCTURE_NOTIFY | EventMask::EXPOSURE | EventMask::KEY_PRESS);

        self.xcb.create_window(
            depth,
            self.window,
            self.root,
            self.offset_x,
            self.offset_y,
            self.width,
            self.height,
            0,
            x11rb::protocol::xproto::WindowClass::INPUT_OUTPUT,
            visual,
            &swa,
        ).unwrap().check().unwrap();

        // Move the window to the bottom so it is the background
        self.xcb.configure_window(
            self.window,
            &ConfigureWindowAux::new().stack_mode(StackMode::BELOW),
        ).unwrap().check().unwrap();

        // Draw and sync
        self.xcb.map_window(self.window).unwrap().check().unwrap();
        self.xcb.sync().unwrap();

        std::thread::sleep(Duration::from_secs(2));
    }
}
