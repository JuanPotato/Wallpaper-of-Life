extern crate x11_dl;
extern crate x11rb;

use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ColormapAlloc, ConfigureWindowAux, ConnectionExt as XprotoConnectionExt, EventMask, StackMode, Point, GcontextWrapper, CreateGCAux, Rectangle};
use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};
use x11_dl::{glx, xlib, xrender};
use x11rb::protocol::Event;
use x11rb::errors::ReplyOrIdError;

fn main() {
    let mut wol = WoL::new();
    wol.init_background();
    wol.main_loop();
}

struct WoL {
    xcb: x11rb::xcb_ffi::XCBConnection,
    xlib: xlib::Xlib,
    glx: glx::Glx,
    xrender: xrender::Xrender,

    display: *mut xlib::Display,
    screen_num: usize,
    root: x11rb::protocol::xproto::Window,
    window: x11rb::protocol::xproto::Window,

    fbconfig: glx::GLXFBConfig,
    visual_info: xlib::XVisualInfo,
    pict_format: *mut xrender::XRenderPictFormat,
    colormap: x11rb::protocol::xproto::Colormap,

    offset_x: i16,
    offset_y: i16,
    width: u16,
    height: u16,
}

impl WoL {
    fn new() -> WoL {
        let xlib = xlib::Xlib::open().unwrap();
        let xlib_xcb = x11_dl::xlib_xcb::Xlib_xcb::open().unwrap();
        let glx = glx::Glx::open().unwrap();
        let xrender = xrender::Xrender::open().unwrap();

        let (display, screen_num, xcb) = unsafe {
            /* Open Xlib Display */
            let display = (xlib.XOpenDisplay)(null());
            if display.is_null() {
                panic!("Can't open display");
            }

            let default_screen = (xlib.XDefaultScreen)(display);

            /* Get the XCB connection from the display */
            let xcb_conn = (xlib_xcb.XGetXCBConnection)(display);
            if xcb_conn.is_null() {
                panic!("Can't get xcb connection from display");
            }

            let conn =
                x11rb::xcb_ffi::XCBConnection::from_raw_xcb_connection(xcb_conn as *mut _, true)
                    .expect("Couldn't make XCBConnection from raw xcb connection");

            (display, default_screen as usize, conn)
        };

        let screen = &xcb.setup().roots[screen_num];
        let root = screen.root;
        let width = screen.width_in_pixels;
        let height = screen.height_in_pixels;

        WoL {
            xcb,
            xlib,
            glx,
            xrender,

            display,
            screen_num,
            root,
            window: 0,

            fbconfig: null_mut(),
            visual_info: unsafe { MaybeUninit::zeroed().assume_init() },
            pict_format: null_mut(),
            colormap: 0,

            offset_x: 0,
            offset_y: 0,
            width,
            height,
        }
    }

    fn screen(&mut self) -> &x11rb::protocol::xproto::Screen {
        &self.xcb.setup().roots[self.screen_num]
    }

    unsafe fn find_fbconfig(&mut self) {
        let attr = [
            glx::GLX_X_RENDERABLE, true as i32,
            glx::GLX_DRAWABLE_TYPE, glx::GLX_WINDOW_BIT,
            glx::GLX_RENDER_TYPE, glx::GLX_RGBA_BIT,
            glx::GLX_X_VISUAL_TYPE, glx::GLX_TRUE_COLOR,
            glx::GLX_RED_SIZE, 8,
            glx::GLX_GREEN_SIZE, 8,
            glx::GLX_BLUE_SIZE, 8,
            glx::GLX_ALPHA_SIZE, 8,
            glx::GLX_DEPTH_SIZE, 24,
            glx::GLX_STENCIL_SIZE, 8,
            glx::GLX_DOUBLEBUFFER, true as i32,
            //GLX_SAMPLE_BUFFERS  , 1,
            //GLX_SAMPLES         , 4,
            glx::GLX_NONE,
        ];

        let mut fbconfig_count = 0;
        let matched_fbconfigs = (self.glx.glXChooseFBConfig)(
            self.display,
            self.screen_num as i32,
            attr.as_ptr(),
            &mut fbconfig_count,
        );
        if matched_fbconfigs.is_null() {
            panic!("Couldn't get FB configs\n");
        }

        for i in 0..fbconfig_count {
            let fbconfig = *matched_fbconfigs.offset(i as isize);

            let visual_info = (self.glx.glXGetVisualFromFBConfig)(self.display, fbconfig);
            if visual_info.is_null() {
                continue;
            }

            let pict_format =
                (self.xrender.XRenderFindVisualFormat)(self.display, (*visual_info).visual);
            (self.xlib.XFree)(visual_info as *mut c_void);
            if pict_format.is_null() {
                continue;
            }

            self.pict_format = pict_format;
            self.fbconfig = *matched_fbconfigs.offset(i as isize);
            if (*pict_format).direct.alphaMask > 0 {
                break;
            }
        }

        (self.xlib.XFree)(matched_fbconfigs as *mut c_void);

        let visual_info = (self.glx.glXGetVisualFromFBConfig)(self.display, self.fbconfig);
        if visual_info.is_null() {
            panic!("Couldn't get a visual\n");
        }
        self.visual_info = *visual_info;
        (self.xlib.XFree)(visual_info as *mut c_void);
    }

    fn init_background(&mut self) {
        unsafe {
            self.find_fbconfig();
        }

        self.colormap = self.xcb.generate_id().unwrap();
        self.xcb
            .create_colormap(
                ColormapAlloc::NONE,
                self.colormap,
                self.root,
                self.visual_info.visualid as u32,
            )
            .unwrap()
            .check()
            .unwrap();

        let all_events = EventMask::KEY_PRESS |
            EventMask::KEY_RELEASE |
            EventMask::BUTTON_PRESS |
            EventMask::BUTTON_RELEASE |
            EventMask::ENTER_WINDOW |
            EventMask::LEAVE_WINDOW |
            // EventMask::POINTER_MOTION |
            // EventMask::POINTER_MOTION_HINT |
            EventMask::BUTTON1_MOTION |
            EventMask::BUTTON2_MOTION |
            EventMask::BUTTON3_MOTION |
            EventMask::BUTTON4_MOTION |
            EventMask::BUTTON5_MOTION |
            // EventMask::BUTTON_MOTION |
            // EventMask::KEYMAP_STATE |
            EventMask::EXPOSURE |
            EventMask::VISIBILITY_CHANGE |
            EventMask::STRUCTURE_NOTIFY |
            EventMask::RESIZE_REDIRECT |
            // EventMask::SUBSTRUCTURE_NOTIFY |
            // EventMask::SUBSTRUCTURE_REDIRECT |
            // EventMask::FOCUS_CHANGE |
            // EventMask::PROPERTY_CHANGE |
            // EventMask::COLOR_MAP_CHANGE |
            EventMask::OWNER_GRAB_BUTTON;

        let swa = x11rb::protocol::xproto::CreateWindowAux::new()
            // .background_pixmap(BackPixmap::NONE)
            .background_pixel(0x00ff00)
            // .border_pixmap(0)
            .border_pixel(0)
            // .backing_store(BackingStore::NOT_USEFUL)
            .event_mask(all_events)
            .do_not_propogate_mask(0)
            .override_redirect(1)
            .colormap(self.colormap);

        self.window = self.xcb.generate_id().unwrap();
        self.xcb
            .create_window(
                self.visual_info.depth as u8,
                self.window,
                self.root,
                self.offset_x,
                self.offset_y,
                self.width,
                self.height,
                0,
                x11rb::protocol::xproto::WindowClass::INPUT_OUTPUT,
                self.visual_info.visualid as u32,
                &swa,
            )
            .unwrap()
            .check()
            .unwrap();

        // Move the window to the bottom so it is the background
        self.xcb
            .configure_window(
                self.window,
                &ConfigureWindowAux::new().stack_mode(StackMode::BELOW),
            )
            .unwrap()
            .check()
            .unwrap();

        // Draw and sync
        self.xcb.map_window(self.window).unwrap().check().unwrap();
        self.xcb.sync().unwrap();
    }

    fn main_loop(&mut self) {
        let mut buttons = [false; 10];
        let black_gc = create_gc_with_foreground(&self.xcb, self.window, 0x000000).unwrap();
        let white_gc = create_gc_with_foreground(&self.xcb, self.window, 0xffffff).unwrap();

        loop {
            let event = self.xcb.wait_for_event().unwrap();

            match event {
                // Event::KeyPress(event) => {}
                Event::MotionNotify(event) => {
                    if buttons[1] {
                        self.xcb.poly_fill_rectangle(
                            self.window, black_gc.gcontext(),
                            &[Rectangle { x: event.event_x, y: event.event_y, width: 10, height: 10 }],
                        ).unwrap().check().unwrap();
                    } else {
                        self.xcb.poly_fill_rectangle(
                            self.window, white_gc.gcontext(),
                            &[Rectangle { x: event.event_x, y: event.event_y, width: 10, height: 10 }],
                        ).unwrap().check().unwrap();
                    }
                    // println!("x: {}, y: {}", event.event_x, event.event_y);
                }
                Event::ButtonPress(event) => {
                    buttons[event.detail as usize] = true;
                    if event.detail == 1 {
                        self.xcb.poly_fill_rectangle(
                            self.window, black_gc.gcontext(),
                            &[Rectangle { x: event.event_x, y: event.event_y, width: 10, height: 10 }],
                        ).unwrap().check().unwrap();
                    } else {
                        self.xcb.poly_fill_rectangle(
                            self.window, white_gc.gcontext(),
                            &[Rectangle { x: event.event_x, y: event.event_y, width: 10, height: 10 }],
                        ).unwrap().check().unwrap();
                    }
                }
                Event::ButtonRelease(event) => {
                    buttons[event.detail as usize] = false;
                    if event.detail == 1 {}
                }
                // Event::ButtonPress(event) => { }
                // Event::ButtonPress(event) => { }

                // Event::Error(_) => println!("Got an unexpected error"),
                _ => {
                    // println!("Got an unknown event");
                    // println!("{:?}", &event);
                }
            }
        }
    }
}

fn create_gc_with_foreground<C: Connection>(
    conn: &C,
    win_id: x11rb::protocol::xproto::Window,
    foreground: u32,
) -> Result<GcontextWrapper<'_, C>, ReplyOrIdError> {
    GcontextWrapper::create_gc(
        conn,
        win_id,
        &CreateGCAux::new()
            .graphics_exposures(0)
            .foreground(foreground),
    )
}
