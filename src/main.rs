extern crate gl;
extern crate glfw;
extern crate x11_dl;
extern crate x11rb;

use glfw::{Action, Context, Key, OpenGlProfileHint, Window, WindowEvent, WindowHint};
// include the OpenGL type aliases
use gl::types::*;

use std::ffi::{c_void, CStr, CString};
use std::time::{Duration, Instant};
use x11rb::protocol::xproto::{
    ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as XprotoConnectionExt, StackMode,
};
use x11rb::wrapper::ConnectionExt;

mod game_of_life;

use game_of_life::BasicGoL;
use std::ptr::null;

fn main() {
    // BasicGoL::new(1000, 1000).bench(1000);
    // let mut gol = BasicGoL::new(5, 5);
    // gol.fill_with_gliders();
    // gol.print("  ", "██");
    // gol.tick();
    // gol.print("  ", "██");

    let mut wol = WoL::new();
    wol.main_loop();
}

struct WoL {
    glfw: glfw::Glfw,
    window: Window,
    events: std::sync::mpsc::Receiver<(f64, WindowEvent)>,
    gol: BasicGoL,

    shader_program: GLuint,
    vertex_array: GLuint,
    vertex_buffer: GLuint,
}

impl WoL {
    fn new() -> WoL {
        let mut my_glfw = glfw::init(glfw::FAIL_ON_ERRORS.clone()).unwrap();
        my_glfw.window_hint(WindowHint::ContextVersionMajor(3));
        my_glfw.window_hint(WindowHint::ContextVersionMinor(3));
        my_glfw.window_hint(WindowHint::OpenGlProfile(OpenGlProfileHint::Core));

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

        // window.set_all_polling(true);
        window.set_refresh_polling(true);
        // window.set_mouse_button_polling(true);
        // window.set_cursor_pos_polling(true);
        // window.set_cursor_enter_polling(true);

        gl::load_with(|s| my_glfw.get_proc_address_raw(s));
        unsafe {
            unsafe {
                gl::Viewport(0, 0, 1920, 1080);
                gl::ClearColor(0.3, 0.3, 0.5, 1.0);
            }
        }

        let vertex_shader_src = CString::new(
            "#version 330 core
layout (location = 0) in vec3 aPos;

void main()
{
    gl_Position = vec4(aPos, 1.0);
}",
        )
        .unwrap();

        let fragment_shader_src = CString::new(
            "#version 330 core
out vec4 outColor;

void main()
{
    outColor = vec4(1.0f, 1.0f, 1.0f, 1.0f);
}",
        )
        .unwrap();

        let shader_program =
            program_from_sources(&vertex_shader_src, &fragment_shader_src).unwrap();

        let vertices: [GLfloat; 9] = [0.0, 0.5, 0.0, 0.5, -0.5, 0.0, -0.5, -0.5, 0.0];

        let mut vertex_array: GLuint = 0;
        let mut vertex_buffer: GLuint = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vertex_array);
            gl::BindVertexArray(vertex_array);

            gl::GenBuffers(1, &mut vertex_buffer);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(&vertices) as isize,
                vertices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (std::mem::size_of::<GLfloat>() * 3) as i32,
                0 as *const c_void,
            );
            gl::EnableVertexAttribArray(0);
        }

        Self {
            glfw: my_glfw,
            window,
            events,
            gol: BasicGoL::new(100, 100),

            shader_program,
            vertex_array,
            vertex_buffer,
        }
    }

    fn main_loop(&mut self) {
        // Loop until the user closes the window
        let mut last_draw = Instant::now() - Duration::from_secs(1);
        let max_frame_time = Duration::from_secs_f64(1.0);
        let min_frame_time = Duration::from_secs_f64(1.0 / 60.0);

        while !self.window.should_close() {
            let now = Instant::now();
            let delta = now.duration_since(last_draw);

            let time_to_next_tick = if delta.as_secs_f64() > 1.0 {
                0.0
            } else {
                1.0 - delta.as_secs_f64()
            };

            let mut should_redraw = false;

            // Poll for and process events
            self.glfw.wait_events_timeout(time_to_next_tick);
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

            let now = Instant::now();
            let delta = now.duration_since(last_draw);

            if should_redraw || delta >= max_frame_time {
                dbg!();
                last_draw = now;
                self.draw(false);
            }
        }
    }

    fn draw(&mut self, new_tick: bool) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.shader_program);
            gl::BindVertexArray(self.vertex_array);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            dbg!(gl::GetError());
        }

        self.window.swap_buffers();
    }
}

fn program_from_sources(vertex_src: &CStr, fragment_src: &CStr) -> Result<GLuint, String> {
    let vertex_shader = shader_from_source(vertex_src, gl::VERTEX_SHADER)?;
    let fragment_shader = shader_from_source(fragment_src, gl::FRAGMENT_SHADER)?;

    // link shaders
    let shader_program = unsafe { gl::CreateProgram() };
    unsafe {
        gl::AttachShader(shader_program, vertex_shader);
        gl::AttachShader(shader_program, fragment_shader);

        let out = CString::new("outColor").unwrap();
        gl::BindFragDataLocation(shader_program, 0, out.as_ptr() as *const GLchar);

        gl::LinkProgram(shader_program);
    }

    // Linking error check
    let mut success: gl::types::GLint = 1;
    unsafe {
        gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
    }

    if success == 0 {
        // Get length of error
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl::GetProgramiv(shader_program, gl::INFO_LOG_LENGTH, &mut len);
        }
        // allocate buffer of correct size
        let error: CString = make_whitespace_cstring(len as usize);
        // Grab the error
        unsafe {
            gl::GetProgramInfoLog(
                shader_program,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar,
            );
        }

        return Err(error.to_string_lossy().to_string());
    }

    unsafe {
        gl::UseProgram(shader_program);
        // gl::DeleteShader(vertex_shader);
        // gl::DeleteShader(fragment_shader);
    }

    Ok(shader_program)
}

fn shader_from_source(source: &CStr, kind: GLenum) -> Result<GLuint, String> {
    // Get shader id
    let id = unsafe { gl::CreateShader(kind) };

    // Compile shader
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }

    let mut success: gl::types::GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }

    if success == 0 {
        // Get length of error
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        }
        // allocate buffer of correct size
        let error: CString = make_whitespace_cstring(len as usize);
        // Grab the error
        unsafe {
            gl::GetShaderInfoLog(
                id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar,
            );
        }

        Err(error.to_string_lossy().to_string())
    } else {
        Ok(id)
    }
}

fn make_whitespace_cstring(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.resize(len as usize, b' ');
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}

unsafe fn make_window_wallpaper(raw_xcb_conn: *mut c_void, window: u32) {
    let xcb = x11rb::xcb_ffi::XCBConnection::from_raw_xcb_connection(raw_xcb_conn as _, false)
        .expect("Couldn't make XCBConnection from raw xcb connection");

    xcb.unmap_window(window).unwrap();

    xcb.sync().unwrap();

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
