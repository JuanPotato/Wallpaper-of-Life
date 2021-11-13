extern crate gl;
extern crate glfw;
extern crate x11_dl;
extern crate x11rb;

use glfw::{Action, Context, Key, Window, WindowEvent};
// include the OpenGL type aliases
use gl::types::*;

use std::ffi::c_void;
use std::time::{Duration, Instant};
use x11rb::protocol::xproto::{
    ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as XprotoConnectionExt, StackMode,
};
use x11rb::wrapper::ConnectionExt;

mod game_of_life;

fn main() {
    let mut wol = WoL::new();
    wol.main_loop();
}

struct WoL {
    glfw: glfw::Glfw,
    window: Window,
    events: std::sync::mpsc::Receiver<(f64, WindowEvent)>,
}

impl WoL {
    fn new() -> WoL {
        let mut my_glfw = glfw::init(glfw::FAIL_ON_ERRORS.clone()).unwrap();

        // Create a windowed mode window and its OpenGL context
        let (mut window, events) = my_glfw
            .create_window(1920, 1080, "Wallpaper of Life", glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");

        unsafe {
            let xlib_xcb = x11_dl::xlib_xcb::Xlib_xcb::open().unwrap();

            let disp = my_glfw.get_x11_display() as *mut x11_dl::xlib::Display;
            let win = window.get_x11_window();

            /* Get the XCB connection from the display */
            let xcb_conn = (xlib_xcb.XGetXCBConnection)(disp);
            if xcb_conn.is_null() {
                panic!("Can't get xcb connection from display");
            }

            make_window_wallpaper(xcb_conn, win as u32);
        }

        // Make the window's context current
        window.make_current();

        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_cursor_enter_polling(true);

        gl::load_with(|s| my_glfw.get_proc_address_raw(s));
        unsafe {
            gl::ClearColor(1.0, 1.0, 1.0, 1.0);
        }

        Self {
            glfw: my_glfw,
            window,
            events,
        }
    }

    fn main_loop(&mut self) {
        // Loop until the user closes the window
        let mut last_draw = Instant::now() - Duration::from_secs(1);
        let max_frame_time = Duration::from_secs_f32(1.0 / 60.0);

        while !self.window.should_close() {
            let now = Instant::now();
            let delta = now.duration_since(last_draw);

            let mut should_redraw = false;

            // Poll for and process events
            self.glfw.poll_events();

            for (_, event) in glfw::flush_messages(&self.events) {
                dbg!(&event);
                match event {
                    glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true);
                        should_redraw = false;
                    }
                    glfw::WindowEvent::Refresh => {
                        should_redraw = true;
                    }
                    _ => {}
                }
            }

            if should_redraw || delta >= max_frame_time {
                self.draw();
            }
        }
    }

    fn draw(&mut self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        self.window.swap_buffers();
    }
}

unsafe fn make_window_wallpaper(raw_xcb_conn: *mut c_void, window: u32) {
    let xcb = x11rb::xcb_ffi::XCBConnection::from_raw_xcb_connection(raw_xcb_conn as _, false)
        .expect("Couldn't make XCBConnection from raw xcb connection");

    xcb.unmap_window(window).unwrap();

    // This makes it work
    xcb.change_window_attributes(
        window,
        &ChangeWindowAttributesAux::new()
            .override_redirect(1)
            .border_pixel(0),
    )
    .unwrap()
    .check()
    .unwrap();

    // Move the window to the bottom so it is the background
    xcb.configure_window(
        window,
        &ConfigureWindowAux::new()
            .stack_mode(StackMode::BELOW)
            .width(1920)
            .height(1080),
    )
    .unwrap()
    .check()
    .unwrap();

    xcb.map_window(window).unwrap();
    xcb.sync().unwrap();
}
