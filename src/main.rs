extern crate glfw;
extern crate x11_dl;
extern crate x11rb;

use glfw::{Action, Context, Key};

use std::ffi::c_void;
use std::time::Duration;

use x11rb::protocol::xproto::{
    ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as XprotoConnectionExt, StackMode,
};
use x11rb::wrapper::ConnectionExt;

mod game_of_life;

fn main() {
    let mut gl = glfw::init(glfw::FAIL_ON_ERRORS.clone()).unwrap();

    // Create a windowed mode window and its OpenGL context
    let (mut window, events) = gl
        .create_window(1920, 1080, "Wallpaper of Life", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    unsafe {
        let xlib_xcb = x11_dl::xlib_xcb::Xlib_xcb::open().unwrap();

        let disp = gl.get_x11_display() as *mut x11_dl::xlib::Display;
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

    let mut n = 0;
    // Loop until the user closes the window
    while !window.should_close() {
        // Swap front and back buffers
        window.swap_buffers();

        // Poll for and process events
        gl.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            dbg!(&event);
            n += 1;
            // Safety net for the bug that causes the giant window to not move to the background
            if n > 100 {
                window.set_should_close(true);
            }
            // println!("{:?}", event);
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                _ => {}
            }
        }
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
