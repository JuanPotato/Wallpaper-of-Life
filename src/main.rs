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
    width: u32,
    height: u32,
    scale: u32,
    window: Window,
    events: std::sync::mpsc::Receiver<(f64, WindowEvent)>,

    front_tex: GLenum,
    front_buf: GLuint,
    back_tex: GLenum,
    back_buf: GLuint,

    gol_frame_buffer: GLuint,

    gol_shader: GLuint,
    gol_uni_state: GLint,
    copy_shader: GLuint,
    copy_uni_state: GLint,

    vertex_array: GLuint,
    vertex_buffer: GLuint,
}

impl WoL {
    fn new() -> WoL {
        let mut my_glfw = glfw::init(glfw::FAIL_ON_ERRORS.clone()).unwrap();

        let (width, height) = my_glfw.with_primary_monitor(|g, mon| {
            let vid = mon.unwrap().get_video_mode().unwrap();
            (vid.width, vid.height)
        });

        let scale = 8;

        my_glfw.window_hint(WindowHint::ContextVersionMajor(3));
        my_glfw.window_hint(WindowHint::ContextVersionMinor(3));
        my_glfw.window_hint(WindowHint::OpenGlProfile(OpenGlProfileHint::Core));

        // Create a windowed mode window and its OpenGL context
        let (mut window, events) = my_glfw
            .create_window(
                width,
                height,
                "Wallpaper of Life",
                glfw::WindowMode::Windowed,
            )
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

            make_window_wallpaper(xcb_conn, win as u32, width, height);
        }

        // Make the window's context current
        window.make_current();

        // window.set_all_polling(true);
        window.set_refresh_polling(true);
        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        // window.set_cursor_enter_polling(true);

        gl::load_with(|s| my_glfw.get_proc_address_raw(s));
        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
            gl::ClearColor(0.3, 0.3, 0.5, 1.0);
        }

        let quad_vertex = CString::new(include_str!("../glsl/quad.vert")).unwrap();
        let gol_frag_shader = CString::new(include_str!("../glsl/gol.frag")).unwrap();
        let copy_frag_shader = CString::new(include_str!("../glsl/copy.frag")).unwrap();

        let gol_shader = program_from_sources(&quad_vertex, &gol_frag_shader).unwrap();
        let gol_uni_state = get_uniform_location(gol_shader, "state");
        let gol_uni_scale = get_uniform_location(gol_shader, "scale");

        let copy_shader = program_from_sources(&quad_vertex, &copy_frag_shader).unwrap();
        let copy_uni_state = get_uniform_location(copy_shader, "state");
        let copy_uni_scale = get_uniform_location(copy_shader, "scale");

        #[rustfmt::skip]
        let vertices: [GLfloat; 8] = [
            -1.0, -1.0,
             1.0, -1.0,
            -1.0,  1.0,
             1.0,  1.0
        ];

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
                2,
                gl::FLOAT,
                gl::FALSE,
                (std::mem::size_of::<GLfloat>() * 2) as i32,
                0 as *const c_void,
            );
            gl::EnableVertexAttribArray(0);
        }

        // Create texture to hold color buffer
        let front_tex = gl::TEXTURE0;
        let front_tex_id =
            make_texture2d(front_tex, width as _, height as _, gl::REPEAT, gl::NEAREST);
        let back_tex = gl::TEXTURE1;
        let back_tex_id =
            make_texture2d(back_tex, width as _, height as _, gl::REPEAT, gl::NEAREST);

        unsafe {
            gl::UseProgram(gol_shader);
            gl::Uniform1i(gol_uni_state, (back_tex - gl::TEXTURE0) as i32);
            gl::Uniform2f(gol_uni_scale, width as GLfloat, height as GLfloat);

            gl::UseProgram(copy_shader);
            gl::Uniform1i(copy_uni_state, (front_tex - gl::TEXTURE0) as i32);
            gl::Uniform2f(
                copy_uni_scale,
                (width * scale) as GLfloat,
                (height * scale) as GLfloat,
            );
        }

        let mut gol_frame_buffer = 0;
        unsafe {
            // Create framebuffer
            gl::GenFramebuffers(1, &mut gol_frame_buffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, gol_frame_buffer);

            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                front_tex_id,
                0,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Self {
            glfw: my_glfw,
            width,
            height,
            scale,
            window,
            events,

            front_tex,
            front_buf: front_tex_id,
            back_tex,
            back_buf: back_tex_id,

            gol_frame_buffer,

            gol_shader,
            gol_uni_state,
            copy_shader,
            copy_uni_state,

            vertex_array,
            vertex_buffer,
        }
    }

    fn main_loop(&mut self) {
        // Loop until the user closes the window
        let mut last_tick = Instant::now() - Duration::from_secs(1);
        let delay = 0.25;

        let max_frame_time = Duration::from_secs_f64(delay);
        let min_frame_time = Duration::from_secs_f64(1.0 / 60.0);

        let mut mouse_pos = (0, 0);

        while !self.window.should_close() {
            let now = Instant::now();
            let delta = now.duration_since(last_tick);

            let time_to_next_tick = if delta.as_secs_f64() > delay {
                0.0
            } else {
                delay - delta.as_secs_f64()
            };

            let mut should_redraw = false;

            // Poll for and process events
            self.glfw.wait_events_timeout(time_to_next_tick);
            for (_, event) in glfw::flush_messages(&self.events) {
                // dbg!(&event);
                match event {
                    glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true);
                        should_redraw = false;
                    }
                    glfw::WindowEvent::Refresh => {
                        should_redraw = true;
                    }
                    glfw::WindowEvent::MouseButton(but, act, mods) => {
                        unsafe {
                            gl::ActiveTexture(self.front_tex); // GL_TEXTURE0-31

                            dbg!(gl::GetError());
                            let pixels = [
                                0x00000000,
                                0xffffffff,
                                0x00000000,
                                0xffffffff,
                                0x00000000,
                                0x00000000,
                                0xffffffff,
                                0xffffffff,
                                0xffffffffu32,
                            ];
                            let x = mouse_pos.0 / 1;
                            let y = mouse_pos.1 / 1;
                            dbg!((x, y));
                            gl::TexSubImage2D(
                                gl::TEXTURE_2D,
                                0,
                                x as i32,
                                (self.height - y) as i32,
                                3,
                                3,
                                gl::RGBA,
                                gl::UNSIGNED_BYTE,
                                pixels.as_ptr() as _,
                            );
                            dbg!(gl::GetError());
                            should_redraw = true;
                        }
                    }
                    glfw::WindowEvent::CursorPos(x, y) => {
                        mouse_pos = (x as u32, y as u32);
                    }
                    _ => {}
                }
            }

            let now = Instant::now();
            let delta = now.duration_since(last_tick);
            let tick = delta >= max_frame_time;

            if should_redraw || tick {
                dbg!();
                if tick {
                    last_tick = now;
                }
                self.draw(tick);
            }
        }
    }

    fn draw(&mut self, new_tick: bool) {
        if new_tick {
            unsafe {
                // About to generate a new state, swap front and back
                std::mem::swap(&mut self.back_buf, &mut self.front_buf);
                std::mem::swap(&mut self.back_tex, &mut self.front_tex);

                // Bind to the frame buffer since we need to render to it
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.gol_frame_buffer);

                // Make sure to render to the newly swapped front
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    self.front_buf,
                    0,
                );

                gl::UseProgram(self.gol_shader);

                // Set updated uniform so they read from the right place
                gl::Uniform1i(self.gol_uni_state, (self.back_tex - gl::TEXTURE0) as i32);

                // Use gol shader to compute next tick
                gl::BindVertexArray(self.vertex_array);
                gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

                // Unbind program
                gl::UseProgram(0);

                // Unbind so that we can render to the screen now
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                dbg!(gl::GetError()); // <3
            }
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::UseProgram(self.copy_shader);

            gl::Uniform1i(self.copy_uni_state, (self.front_tex - gl::TEXTURE0) as i32);
            gl::BindVertexArray(self.vertex_array);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            gl::UseProgram(0);
            dbg!(gl::GetError()); // <3
        }

        self.window.swap_buffers();
    }
}

fn get_uniform_location(program: GLuint, uniform: &str) -> GLint {
    let uniform_cstr = CString::new(uniform).unwrap();
    unsafe { gl::GetUniformLocation(program, uniform_cstr.as_ptr() as *const GLchar) }
}

fn make_texture2d(
    texture: GLenum,
    width: GLint,
    height: GLint,
    wrap: GLenum,
    scale: GLenum,
) -> GLuint {
    let mut texture_id = 0;

    unsafe {
        gl::GenTextures(1, &mut texture_id);
        gl::ActiveTexture(texture); // GL_TEXTURE0-31
        gl::BindTexture(gl::TEXTURE_2D, texture_id);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            width,
            height,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, scale as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, scale as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap as i32);
    }

    texture_id
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

unsafe fn make_window_wallpaper(raw_xcb_conn: *mut c_void, window: u32, width: u32, height: u32) {
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
            .x(0)
            .y(0)
            .width(width)
            .height(height),
    )
    .unwrap()
    .check()
    .unwrap();

    xcb.map_window(window).unwrap();
    xcb.sync().unwrap();
}
