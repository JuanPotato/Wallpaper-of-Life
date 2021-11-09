extern crate glfw;
extern crate x11;

use glfw::{Action, Context, Key};
use std::os::raw::c_ulong;
use x11::xlib::{Display, Window, Screen};
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::time::Duration;
use std::ptr::{null, null_mut};

fn main() {
    unsafe {
        let mut wol = WoL::new();
        wol.init_background();
        dbg!(wol);
    }
    // let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS.clone()).unwrap();
    // glfw.window_hint(glfw::WindowHint::Resizable(false));
    // glfw.window_hint(glfw::WindowHint::Decorated(false));
    // glfw.window_hint(glfw::WindowHint::Floating(true));
    //
    // let (mut window, events) = glfw
    //     .create_window(300, 300, "Hello this is window", glfw::WindowMode::Windowed)
    //     .expect("Failed to create GLFW window.");
    //
    // unsafe {
    //     let x11_disp: *mut Display = std::mem::transmute(glfw.get_x11_display());
    //     let testtttt = x11::xlib::XOpenDisplay(std::ptr::null());
    //
    //     let x11_win: c_ulong = window.get_x11_window() as u64;
    //     let x11_screen = x11::xlib::XDefaultScreen(x11_disp);
    //
    //     let est_win = x11::xlib::XRootWindow(testtttt, x11_screen);
    //
    //
    //     dbg!(x11_disp);
    //     dbg!(testtttt);
    //
    //     dbg!(x11_win, est_win);
    //     dbg!(x11_screen);
    //
    //     x11::xlib::XLowerWindow(x11_disp, x11_win);
    //     // x11::xlib::XDisplayHeight()
    // }
    //
    // window.make_current();
    //
    // // window.set_all_polling(true);
    // // window.set_pos_polling(true);
    // // window.set_size_polling(true);
    // // window.set_close_polling(true);
    // // window.set_refresh_polling(true);
    // // window.set_focus_polling(true);
    // // window.set_iconify_polling(true);
    // // window.set_framebuffer_size_polling(true);
    // // window.set_key_polling(true);
    // // window.set_char_polling(true);
    // // window.set_char_mods_polling(true);
    // // window.set_mouse_button_polling(true);
    // // window.set_cursor_pos_polling(true);
    // // window.set_cursor_enter_polling(true);
    // // window.set_scroll_polling(true);
    // // window.set_maximize_polling(true);
    // // window.set_content_scale_polling(true);
    //
    // while !window.should_close() {
    //     glfw.poll_events();
    //     for (_, event) in glfw::flush_messages(&events) {
    //         handle_window_event(&mut window, event);
    //     }
    // }
}

fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    dbg!(&event);
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
        _ => {}
    }
}

#[derive(Debug)]
struct WoL {
    // xlib: x11::Xlib,
    disp: *mut Display,
    screen_num: i32,
    root: Window,
    desktop: Window,
    window: Window,

    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,

    swa: x11::xlib::XSetWindowAttributes,
    fbc: x11::glx::GLXFBConfig,
    vi: *mut x11::xlib::XVisualInfo,
    pict: *mut x11::xrender::XRenderPictFormat,
    cmap: x11::xlib::Colormap,
}

impl WoL {
    unsafe fn new() -> WoL {
        // let xlib = x11::Xlib::open().unwrap();
        // let disp = xlib.XOpenDisplay(None);
        // let screen_num = xlib.XDefaultScreen(disp);
        // let root = xlib.XRootWindow(disp, screen);
        // let width = xlib.XDisplayWidth(disp, screen);
        // let height = xlib.XDisplayHeight(disp, screen);

        let disp = x11::xlib::XOpenDisplay(std::ptr::null());
        let screen_num = x11::xlib::XDefaultScreen(disp);
        let root = x11::xlib::XRootWindow(disp, screen_num);
        let width = x11::xlib::XDisplayWidth(disp, screen_num);
        let height = x11::xlib::XDisplayHeight(disp, screen_num);

        WoL {
            // xlib,
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

    unsafe fn find_subwindow(&mut self, mut win: Window, w: i32, h: i32) -> Window {
        let mut troot: Window = 0;
        let mut parent: Window = 0;
        let mut children: *mut Window = std::ptr::null_mut();
        let mut n = 0;

        for i in 0..10 {
            x11::xlib::XQueryTree(self.disp, win, &mut troot, &mut parent, &mut children, &mut n);

            for j in 0..n {
                let mut attrs = MaybeUninit::zeroed().assume_init();
                let res = x11::xlib::XGetWindowAttributes(self.disp, *children.offset(j as isize), &mut attrs);

                if (res != 0) {
                    if (attrs.map_state != 0 && (attrs.width == w && attrs.height == h)) {
                        win = *children.offset(j as isize);
                        break;
                    }
                }
            }

            x11::xlib::XFree(children as *mut c_void);
        }

        return win;
    }

    unsafe fn find_desktop_window(&mut self) -> Window {
        let mut win = self.find_subwindow(self.root, -1, -1);
        win = self.find_subwindow(win, self.width, self.height);

        self.desktop = win;

        return win;
    }

    unsafe fn init_background(&mut self) {
        let attr = [
            x11::glx::GLX_X_RENDERABLE    , true as i32,
            x11::glx::GLX_DRAWABLE_TYPE   , x11::glx::GLX_WINDOW_BIT,
            x11::glx::GLX_RENDER_TYPE     , x11::glx::GLX_RGBA_BIT,
            x11::glx::GLX_X_VISUAL_TYPE   , x11::glx::GLX_TRUE_COLOR,
            x11::glx::GLX_RED_SIZE        , 8,
            x11::glx::GLX_GREEN_SIZE      , 8,
            x11::glx::GLX_BLUE_SIZE       , 8,
            x11::glx::GLX_ALPHA_SIZE      , 8,
            x11::glx::GLX_DEPTH_SIZE      , 24,
            x11::glx::GLX_STENCIL_SIZE    , 8,
            x11::glx::GLX_DOUBLEBUFFER    , true as i32,
            //GLX_SAMPLE_BUFFERS  , 1,
            //GLX_SAMPLES         , 4,
            x11::glx::GLX_NONE
        ];

        let mut elemc = 0;
        let fbcs = x11::glx::glXChooseFBConfig(self.disp, self.screen_num, attr.as_ptr(), &mut elemc);
        if (fbcs.is_null()) {
            panic!("Couldn't get FB configs\n");
        }

        let mut pict;
        let mut fbc = null_mut();

        for i in 0..elemc {
            let vi = x11::glx::glXGetVisualFromFBConfig(self.disp, *fbcs.offset(i as isize));
            if vi.is_null() {
                continue;
            }

            pict = x11::xrender::XRenderFindVisualFormat(self.disp, (*vi).visual);
            x11::xlib::XFree(vi as *mut c_void);
            if pict.is_null() {
                continue;
            }

            fbc = *fbcs.offset(i as isize);
            if ((*pict).direct.alphaMask > 0) {
                break;
            }
        }

        x11::xlib::XFree(fbcs as *mut c_void);
        dbg!(fbcs);

        let vi = x11::glx::glXGetVisualFromFBConfig(self.disp, fbc);
        if vi.is_null() {
            panic!("Couldn't get a visual\n");
        }
        let vi = *vi;

        // Window parameters
        self.swa.background_pixel = 0;
        self.swa.border_pixel = 0;
        self.swa.colormap = x11::xlib::XCreateColormap(self.disp, self.root, vi.visual, x11::xlib::AllocNone);
        self.swa.event_mask = x11::xlib::StructureNotifyMask | x11::xlib::ExposureMask | x11::xlib::KeyPressMask;
        let mask = x11::xlib::CWBackPixel | x11::xlib::CWBorderPixel | x11::xlib::CWColormap | x11::xlib::CWEventMask;

        println!("Window depth {}, {}x{}\n", vi.depth, self.width, self.height);

        self.window = x11::xlib::XCreateWindow(self.disp, self.root, self.offset_x, self.offset_y, self.width as u32, self.height as u32, 0, vi.depth, x11::xlib::InputOutput as u32, vi.visual, mask, &mut self.swa);

        // if self.find_desktop_window() == 0 {
        //     panic!("Error: couldn't find desktop window\n");
        // }
        //
        // let window = x11::xlib::XCreateSimpleWindow(
        //     self.disp, self.root, self.offset_x, self.offset_y, self.width as u32, self.height as u32,
        //     0,
        //     x11::xlib::XBlackPixel(self.disp, self.screen_num),
        //     x11::xlib::XWhitePixel(self.disp, self.screen_num));
        //

        x11::xlib::XMapWindow(self.disp, self.window);
        x11::xlib::XLowerWindow(self.disp, self.window);
        x11::xlib::XSync(self.disp, 0);
        std::thread::sleep(Duration::from_secs(5));
    }
}

/*
xwin *init_xwin()
{
    xwin *win = (xwin *)malloc(sizeof(struct xwin));

    if (!(win->display = XOpenDisplay(NULL))) {
        printf("Couldn't open X11 display\n");
        exit(1);
    }

    initBackground(win);

    if (cfg.plasma) {
        long value = XInternAtom(win->display, "_NET_WM_WINDOW_TYPE_DESKTOP", false);
        XChangeProperty(win->display, win->window,
                        XInternAtom(win->display, "_NET_WM_WINDOW_TYPE", false),
                        XA_ATOM, 32, PropModeReplace, (unsigned char *) &value, 1);
        XMapWindow(win->display, win->window);
    }

    if (cfg.transparency < 1.0) {
        uint32_t cardinal_alpha = (uint32_t) (cfg.transparency * (uint32_t)-1) ;
        XChangeProperty(win->display, win->window,
                        XInternAtom(win->display, "_NET_WM_WINDOW_OPACITY", 0),
                        XA_CARDINAL, 32, PropModeReplace, (uint8_t*) &cardinal_alpha, 1);
    }
    return win;
}

void initBackground(xwin *win)
{
    int attr[] = {
        GLX_X_RENDERABLE    , True,
        GLX_DRAWABLE_TYPE   , GLX_WINDOW_BIT,
        GLX_RENDER_TYPE     , GLX_RGBA_BIT,
        GLX_X_VISUAL_TYPE   , GLX_TRUE_COLOR,
        GLX_RED_SIZE        , 8,
        GLX_GREEN_SIZE      , 8,
        GLX_BLUE_SIZE       , 8,
        GLX_ALPHA_SIZE      , 8,
        GLX_DEPTH_SIZE      , 24,
        GLX_STENCIL_SIZE    , 8,
        GLX_DOUBLEBUFFER    , True,
        //GLX_SAMPLE_BUFFERS  , 1,
        //GLX_SAMPLES         , 4,
        None
    };

    win->screenNum = DefaultScreen(win->display);
    win->root = RootWindow(win->display, win->screenNum);

    if (cfg.geometry) {
        win->offX = cfg.offX, win->offY = cfg.offY;
        win->width = cfg.width, win->height = cfg.height;
    } else {
        win->width = DisplayWidth(win->display, win->screenNum),
        win->height = DisplayHeight(win->display, win->screenNum);
    }

    if(!find_desktop_window(win)) {
        printf("Error: couldn't find desktop window\n");
        exit(1);
    }

    int elemc;
    win->fbcs = glXChooseFBConfig(win->display, win->screenNum, attr, &elemc);
    if (!win->fbcs) {
        printf("Couldn't get FB configs\n");
        exit(1);
    }

    for (int i = 0; i < elemc; i++) {
        win->vi = (XVisualInfo *)glXGetVisualFromFBConfig(win->display, win->fbcs[i]);
        if (!win->vi)
               continue;

        win->pict = XRenderFindVisualFormat(win->display, win->vi->visual);
        XFree(win->vi);
        if (!win->pict)
            continue;

        win->fbc = win->fbcs[i];
        if (win->pict->direct.alphaMask > 0)
            break;
    }

    XFree(win->fbcs);

    win->vi = (XVisualInfo *)glXGetVisualFromFBConfig(win->display, win->fbc);
    if (!win->vi) {
        printf("Couldn't get a visual\n");
        exit(1);
    }

    // Window parameters
    win->swa.background_pixmap = ParentRelative;
    win->swa.background_pixel = 0;
    win->swa.border_pixmap = 0;
    win->swa.border_pixel = 0;
    win->swa.bit_gravity = 0;
    win->swa.win_gravity = 0;
    win->swa.override_redirect = True;
    win->swa.colormap = XCreateColormap(win->display, win->root, win->vi->visual, AllocNone);
    win->swa.event_mask = StructureNotifyMask | ExposureMask;
    unsigned long mask = CWOverrideRedirect | CWBackingStore | CWBackPixel | CWBorderPixel | CWColormap;

    if (cfg.debug)
        printf("Window depth %d, %dx%d\n", win->vi->depth, win->width, win->height);

    win->window = XCreateWindow(win->display, win->root, win->offX, win->offY, win->width, win->height, 0,
            win->vi->depth, InputOutput, win->vi->visual, mask, &win->swa);

    XLowerWindow(win->display, win->window);
}



void swapBuffers(xwin *win) {
    glXSwapBuffers(win->display, win->window);
    checkErrors("Swapping buffs");
}
 */
