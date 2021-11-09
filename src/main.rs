extern crate x11_dl;
extern crate x11rb;

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};
use std::time::Duration;
use x11_dl::xlib::{
    ExposureMask, KeyPressMask, StructureNotifyMask,
};
use x11_dl::{
    glx,
    xlib,
    xrender,
};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    BackPixmap, ConfigureWindowAux, ConnectionExt as XprotoConnectionExt, Gravity,
    MapState, StackMode,
};
use x11rb::wrapper::ConnectionExt;

fn main() {
    unsafe {
        let mut wol = WoLXlib::new();
        wol.init_background();
    }

    let mut wol = WoL::new();
    wol.init_background();
}

struct WoL {
    xcb: x11rb::rust_connection::RustConnection,
    // disp: *mut Display,
    screen_num: usize,
    root: x11rb::protocol::xproto::Window,
    desktop: x11rb::protocol::xproto::Window,
    window: x11rb::protocol::xproto::Window,

    offset_x: i16,
    offset_y: i16,
    width: u16,
    height: u16,
    // swa: xlib::XSetWindowAttributes,
    // fbc: glx::GLXFBConfig,
    // vi: *mut xlib::XVisualInfo,
    // pict: *mut xrender::XRenderPictFormat,
    // cmap: xlib::Colormap,
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
            desktop: 0,
            window: 0,

            offset_x: 0,
            offset_y: 0,
            width,
            height,
            //
            // swa: MaybeUninit::zeroed().assume_init(),
            // fbc: MaybeUninit::zeroed().assume_init(),
            // vi: null_mut(),
            // pict: null_mut(),
            // cmap: MaybeUninit::zeroed().assume_init(),
        }
    }

    fn find_subwindow(
        &mut self,
        mut win: x11rb::protocol::xproto::Window,
        w: u16,
        h: u16,
    ) -> x11rb::protocol::xproto::Window {
        for _ in 0..10 {
            let query_tree_reply = self.xcb.query_tree(win).unwrap().reply().unwrap();

            let mut next_win = win;

            for child in query_tree_reply.children {
                let attr_reply = self
                    .xcb
                    .get_window_attributes(child)
                    .unwrap()
                    .reply()
                    .unwrap();
                let dim_reply = self.xcb.get_geometry(child).unwrap().reply().unwrap();

                if attr_reply.map_state != MapState::UNMAPPED
                    && dim_reply.width == w
                    && dim_reply.height == h
                {
                    next_win = child;
                    break;
                }
            }

            if next_win != win {
                win = next_win;
            } else {
                break;
            }
        }

        return win;
    }

    fn find_desktop_window(&mut self) -> x11rb::protocol::xproto::Window {
        self.desktop = self.find_subwindow(self.root, self.width, self.height);
        return self.desktop;
    }

    fn init_background(&mut self) {
        // let attr = [
        //     glx::GLX_X_RENDERABLE, true as i32,
        //     glx::GLX_DRAWABLE_TYPE, glx::GLX_WINDOW_BIT,
        //     glx::GLX_RENDER_TYPE, glx::GLX_RGBA_BIT,
        //     glx::GLX_X_VISUAL_TYPE, glx::GLX_TRUE_COLOR,
        //     glx::GLX_RED_SIZE, 8,
        //     glx::GLX_GREEN_SIZE, 8,
        //     glx::GLX_BLUE_SIZE, 8,
        //     glx::GLX_ALPHA_SIZE, 8,
        //     glx::GLX_DEPTH_SIZE, 24,
        //     glx::GLX_STENCIL_SIZE, 8,
        //     glx::GLX_DOUBLEBUFFER, true as i32,
        //     //GLX_SAMPLE_BUFFERS  , 1,
        //     //GLX_SAMPLES         , 4,
        //     glx::GLX_NONE
        // ];
        //
        // let fbcs = self.xcb.glx_get_fb_configs(self.screen_num).unwrap().reply().unwrap();
        //
        // let mut elemc = 0;
        // let fbcs = (self.glx.glXChooseFBConfig)(self.disp, self.screen_num, attr.as_ptr(), &mut elemc);
        // if (fbcs.is_null()) {
        //     panic!("Couldn't get FB configs\n");
        // }
        //
        // let mut pict;
        // let mut fbc = null_mut();
        //
        // for i in 0..elemc {
        //     let vi = (self.glx.glXGetVisualFromFBConfig)(self.disp, *fbcs.offset(i as isize));
        //     if vi.is_null() {
        //         continue;
        //     }
        //
        //     pict = self.xcb.render_query_pict().unwrap().reply().unwrap();
        //     pict = (self.xrender.XRenderFindVisualFormat)(self.disp, (*vi).visual);
        //     (self.xlib.XFree)(vi as *mut c_void);
        //     if pict.is_null() {
        //         continue;
        //     }
        //
        //     fbc = *fbcs.offset(i as isize);
        //     if ((*pict).direct.alphaMask > 0) {
        //         break;
        //     }
        // }
        //
        // (self.xlib.XFree)(fbcs as *mut c_void);
        // dbg!(fbcs);
        //
        // let vi = (self.glx.glXGetVisualFromFBConfig)(self.disp, fbc);
        // if vi.is_null() {
        //     panic!("Couldn't get a visual\n");
        // }
        // let vi = *vi;
        //
        //

        self.window = self.xcb.generate_id().unwrap();
        let screen = &self.xcb.setup().roots[self.screen_num];
        dbg!(screen);
        let colormap = screen.default_colormap;
        let visual = screen.root_visual;

        // Window parameters
        let swa = x11rb::protocol::xproto::CreateWindowAux::new()
            .background_pixmap(BackPixmap::PARENT_RELATIVE)
            .background_pixel(0)
            .border_pixmap(0)
            .border_pixel(0)
            .bit_gravity(Gravity::BIT_FORGET)
            .win_gravity(Gravity::BIT_FORGET)
            .override_redirect(1)
            // self.swa.colormap = (self.xlib.XCreateColormap)(self.disp, self.root, vi.visual, xlib::AllocNone);
            .event_mask((StructureNotifyMask | ExposureMask | KeyPressMask) as u32);

        self.xcb
            .create_window(
                screen.root_depth,
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
            )
            .unwrap()
            .check()
            .unwrap();
        self.xcb.configure_window(
            self.window,
            &ConfigureWindowAux::new().stack_mode(StackMode::BELOW),
        ).unwrap().check().unwrap();
        self.xcb.map_window(self.window).unwrap().check().unwrap();
        self.xcb.sync().unwrap();

        std::thread::sleep(Duration::from_secs(5));
    }
}

struct WoLXlib {
    xlib: xlib::Xlib,
    glx: glx::Glx,
    xrender: xrender::Xrender,
    disp: *mut xlib::Display,
    screen_num: i32,
    root: xlib::Window,
    desktop: xlib::Window,
    window: xlib::Window,

    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,

    swa: xlib::XSetWindowAttributes,
    fbc: glx::GLXFBConfig,
    vi: *mut xlib::XVisualInfo,
    pict: *mut xrender::XRenderPictFormat,
    cmap: xlib::Colormap,
}

impl WoLXlib {
    unsafe fn new() -> WoLXlib {
        let xlib = x11_dl::xlib::Xlib::open().unwrap();
        let glx = x11_dl::glx::Glx::open().unwrap();
        let xrender = x11_dl::xrender::Xrender::open().unwrap();
        let disp = (xlib.XOpenDisplay)(null());
        let screen_num = (xlib.XDefaultScreen)(disp);
        let root = (xlib.XRootWindow)(disp, screen_num);
        let width = (xlib.XDisplayWidth)(disp, screen_num);
        let height = (xlib.XDisplayHeight)(disp, screen_num);

        WoLXlib {
            xlib,
            glx,
            xrender,
            disp: disp,
            screen_num: screen_num,
            root: root,
            desktop: 0,
            window: 0,

            offset_x: 0,
            offset_y: 0,
            width: width,
            height: height,

            swa: MaybeUninit::zeroed().assume_init(),
            fbc: MaybeUninit::zeroed().assume_init(),
            vi: null_mut(),
            pict: null_mut(),
            cmap: MaybeUninit::zeroed().assume_init(),
        }
    }

    unsafe fn find_subwindow(&mut self, mut win: xlib::Window, w: i32, h: i32) -> xlib::Window {
        let mut troot: xlib::Window = 0;
        let mut parent: xlib::Window = 0;
        let mut children: *mut xlib::Window = std::ptr::null_mut();
        let mut n = 0;

        for _ in 0..10 {
            (self.xlib.XQueryTree)(
                self.disp,
                win,
                &mut troot,
                &mut parent,
                &mut children,
                &mut n,
            );

            for j in 0..n {
                let mut attrs = MaybeUninit::zeroed().assume_init();
                let res = (self.xlib.XGetWindowAttributes)(
                    self.disp,
                    *children.offset(j as isize),
                    &mut attrs,
                );

                if res != 0 {
                    if attrs.map_state != 0 && (attrs.width == w && attrs.height == h) {
                        win = *children.offset(j as isize);
                        break;
                    }
                }
            }

            (self.xlib.XFree)(children as *mut c_void);
        }

        return win;
    }

    unsafe fn find_desktop_window(&mut self) -> xlib::Window {
        let mut win = self.find_subwindow(self.root, -1, -1);
        win = self.find_subwindow(win, self.width, self.height);

        self.desktop = win;

        return win;
    }

    unsafe fn init_background(&mut self) {
        let attr = [
            glx::GLX_X_RENDERABLE,
            true as i32,
            glx::GLX_DRAWABLE_TYPE,
            glx::GLX_WINDOW_BIT,
            glx::GLX_RENDER_TYPE,
            glx::GLX_RGBA_BIT,
            glx::GLX_X_VISUAL_TYPE,
            glx::GLX_TRUE_COLOR,
            glx::GLX_RED_SIZE,
            8,
            glx::GLX_GREEN_SIZE,
            8,
            glx::GLX_BLUE_SIZE,
            8,
            glx::GLX_ALPHA_SIZE,
            8,
            glx::GLX_DEPTH_SIZE,
            24,
            glx::GLX_STENCIL_SIZE,
            8,
            glx::GLX_DOUBLEBUFFER,
            true as i32,
            //GLX_SAMPLE_BUFFERS  , 1,
            //GLX_SAMPLES         , 4,
            glx::GLX_NONE,
        ];

        let mut elemc = 0;
        let fbcs =
            (self.glx.glXChooseFBConfig)(self.disp, self.screen_num, attr.as_ptr(), &mut elemc);
        if fbcs.is_null() {
            panic!("Couldn't get FB configs\n");
        }

        let mut pict;
        let mut fbc = null_mut();

        for i in 0..elemc {
            let vi = (self.glx.glXGetVisualFromFBConfig)(self.disp, *fbcs.offset(i as isize));
            if vi.is_null() {
                continue;
            }

            pict = (self.xrender.XRenderFindVisualFormat)(self.disp, (*vi).visual);
            (self.xlib.XFree)(vi as *mut c_void);
            if pict.is_null() {
                continue;
            }

            fbc = *fbcs.offset(i as isize);
            if (*pict).direct.alphaMask > 0 {
                break;
            }
        }

        (self.xlib.XFree)(fbcs as *mut c_void);
        dbg!(fbcs);

        let vi = (self.glx.glXGetVisualFromFBConfig)(self.disp, fbc);
        if vi.is_null() {
            panic!("Couldn't get a visual\n");
        }
        let vi = *vi;

        // Window parameters
        self.swa.background_pixmap = xlib::ParentRelative as u64;
        self.swa.background_pixel = 0;
        self.swa.border_pixmap = 0;
        self.swa.border_pixel = 0;
        self.swa.bit_gravity = 0;
        self.swa.win_gravity = 0;
        self.swa.override_redirect = xlib::True;
        self.swa.colormap =
            (self.xlib.XCreateColormap)(self.disp, self.root, vi.visual, xlib::AllocNone);
        self.swa.event_mask = xlib::StructureNotifyMask | xlib::ExposureMask; // | xlib::KeyPressMask
        let mask = xlib::CWOverrideRedirect
            | xlib::CWBackingStore
            | xlib::CWBackPixel
            | xlib::CWBorderPixel
            | xlib::CWColormap;

        println!(
            "Window depth {}, {}x{}\n",
            vi.depth, self.width, self.height
        );

        self.window = (self.xlib.XCreateWindow)(
            self.disp,
            self.root,
            self.offset_x,
            self.offset_y,
            self.width as u32,
            self.height as u32,
            0,
            vi.depth,
            xlib::InputOutput as u32,
            vi.visual,
            mask,
            &mut self.swa,
        );

        (self.xlib.XLowerWindow)(self.disp, self.window);
        (self.xlib.XMapWindow)(self.disp, self.window);
        (self.xlib.XSync)(self.disp, 0);
        std::thread::sleep(Duration::from_secs(5));
    }
}
