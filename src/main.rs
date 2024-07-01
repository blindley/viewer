// #![windows_subsystem = "windows"]

use clap::Parser;

mod image_renderer;
use image_renderer::{Renderer, ImageRenderer};

mod texture;
use texture::Texture;

// mod shader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let image_paths = {
        if cli.image_paths.len() != 0 {
            let mut image_paths = Vec::new();
            for p in cli.image_paths.iter() {
                if p.is_dir() {
                    image_paths.append(&mut all_images_in_directory(p)?);
                } else {
                    image_paths.push(p.clone());
                }
            }
            image_paths
        } else {
            all_images_in_directory(".")?
        }
    };

    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title(image_paths[0].to_string_lossy().to_owned());
    
    let wc = glutin::ContextBuilder::new().build_windowed(wb, &el).unwrap();
    let wc = unsafe { wc.make_current().unwrap() };
    
    gl::load_with(|p| wc.get_proc_address(p) as *const _);
    
    let mut app_data = AppData::new(image_paths);

    let frame_duration = std::time::Duration::new(0, 1000000000 / 60);
    let mut next_update_time = std::time::Instant::now() + frame_duration;

    el.run(move |event, _, control_flow| {
        use glutin::event::{Event, WindowEvent, StartCause};
        use glutin::event_loop::ControlFlow;

        *control_flow = ControlFlow::WaitUntil(next_update_time);

        match event {
            Event::LoopDestroyed => return,

            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached { .. } => {
                    if app_data.update(frame_duration.as_secs_f32()) {
                        wc.window().request_redraw();
                    }

                    next_update_time = next_update_time + frame_duration;
                    *control_flow = ControlFlow::WaitUntil(next_update_time);
                },

                _ => (),
            },

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    wc.resize(physical_size);
                    app_data.resize_window([physical_size.width as _, physical_size.height as _]);
                },

                WindowEvent::KeyboardInput { input, .. } => {
                    use glutin::event::VirtualKeyCode::{Escape, Left, Right, X};
                    use glutin::event::ElementState::Pressed;
                    match (input.virtual_keycode, input.state) {
                        (Some(Escape), Pressed) => *control_flow = ControlFlow::Exit,
                        (Some(Left), Pressed) => {
                            #[allow(deprecated)]
                            if input.modifiers.shift() {
                                app_data.shift_left();
                                wc.window().set_title(&app_data.new_window_title());
                            } else {
                                app_data.cycle_left();
                                wc.window().set_title(&app_data.new_window_title());
                                wc.window().request_redraw();
                            }
                        },
                        (Some(Right), Pressed) => {
                            #[allow(deprecated)]
                            if input.modifiers.shift() {
                                app_data.shift_right();
                                wc.window().set_title(&app_data.new_window_title());
                            } else {
                                app_data.cycle_right();
                                wc.window().set_title(&app_data.new_window_title());
                                wc.window().request_redraw();
                            }
                        },
                        (Some(X), Pressed) => {
                            app_data.drop_current();
                            wc.window().set_title(&app_data.new_window_title());
                            wc.window().request_redraw();
                        }
                        _ => (),
                    }
                },

                WindowEvent::CursorMoved { position, .. } => {
                    app_data.cursor_position = [position.x as i32, position.y as i32];
                    wc.window().set_title(&app_data.new_window_title());
                }


                _ => (),
            },
            Event::RedrawRequested(_) => {
                app_data.redraw();
                wc.swap_buffers().unwrap();
            },

            _ => (),
        }
    });
}

fn all_images_in_directory<P: AsRef<std::path::Path>>(dir: P)
    -> std::io::Result<Vec<std::path::PathBuf>>
{
    let mut paths = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if let Some(ext) = entry.path().extension() {
            if let Some(ext_str) = ext.to_str() {
                match ext_str {
                    "png" | "jpg" | "bmp" | "gif" | "jpeg"
                        => paths.push(entry.path().clone()),
                    _ => (),
                }
            }
        }
    }

    Ok(paths)
}

/// A basic image viewer
#[derive(Debug, Parser)]
struct Cli {
    image_paths: Vec<std::path::PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSignature {
    modified: Option<std::time::SystemTime>,
    created: Option<std::time::SystemTime>,
    len: u64,
}

impl FileSignature {
    fn new<P: AsRef<std::path::Path>>(path: P) -> Result<FileSignature, Box<dyn std::error::Error>> {
        let mdata = std::fs::metadata(path)?;
        Ok(FileSignature {
            modified: mdata.modified().ok(),
            created: mdata.created().ok(),
            len: mdata.len(),
        })
    }
}

#[derive(Debug)]
struct TextureFile {
    texture: Texture,
    path: std::path::PathBuf,
    sig: FileSignature,
}

#[derive(Debug)]
struct AppData {
    image_paths: Vec<TextureFile>,
    current_image_index: usize,
    window_size: [i32;2],
    cursor_position: [i32;2],
    renderer: StableAspectRatioImageRenderer,
    
    seconds_elapsed: f32,
}

impl AppData {
    fn new(image_paths: Vec<std::path::PathBuf>) -> AppData {
        let renderer = StableAspectRatioImageRenderer::new();
        // renderer.set_texture_data(&image_paths[0]).unwrap();

        let image_paths = image_paths.iter().map(|p| {
                let sig = FileSignature::new(p).unwrap();
                let texture = Texture::from_file(p).unwrap();
                TextureFile { texture, path: p.clone(), sig }
            }
        ).collect();

        let mut app_data = AppData {
            image_paths,
            current_image_index: 0,
            window_size: [1,1],
            cursor_position: [0,0],
            renderer,
            seconds_elapsed: 0.0,
        };
    
        if let Err(_) = app_data.reload_texture() {
            eprintln!("failed to load {:?}", app_data.image_paths[0].path);
            std::process::exit(-1);
        }

        app_data
    }

    fn redraw(&self) {
        unsafe {
            gl::ClearColor(0.1, 0.1, 0.1, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            self.renderer.render();
        }
    }

    fn reload_texture(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let texture = &self.image_paths[self.current_image_index].texture;
        self.renderer.set_texture_data(texture)?;
        Ok(())
    }

    fn resize_window(&mut self, size: [i32;2]) {
        self.window_size = size;
        self.renderer.resize_window(size);
        
        unsafe { gl::Viewport(0, 0, size[0], size[1]); }
    }

    fn update(&mut self, seconds_elapsed: f32) -> bool {
        self.seconds_elapsed += seconds_elapsed;

        if self.seconds_elapsed >= 1.0 {
            // just reset it, we don't need a stable framerate
            self.seconds_elapsed = 0.0;

            let f = &mut self.image_paths[self.current_image_index];

            // check if file has been modified
            if let Ok(sig) = FileSignature::new(&f.path) {
                if f.sig != sig {
                    f.sig = sig;
                    if self.reload_texture().is_ok() {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn current_image_path(&self) -> &std::path::PathBuf {
        &self.image_paths[self.current_image_index].path
    }

    fn new_window_title(&self) -> String {
        let image_path = self.current_image_path().to_string_lossy();
        let [width, height] = self.renderer.get_image_size();
        let [cursor_x, cursor_y] = self.cursor_position;
        let current_index = self.current_image_index + 1;
        let total = self.image_paths.len();
        format!("{} | {}x{} | {}/{} | ({},{})",
            image_path, width, height, current_index, total, cursor_x, cursor_y)
    }

    fn cycle_left(&mut self) {
        let new_index = self.current_image_index + self.image_paths.len() - 1;
        self.current_image_index = new_index % self.image_paths.len();
        self.reload_texture().unwrap();
    }

    fn cycle_right(&mut self) {
        let new_index = self.current_image_index + 1;
        self.current_image_index = new_index % self.image_paths.len();
        self.reload_texture().unwrap();
    }

    fn swap_image_positions(&mut self, a: usize, b: usize) {
        self.image_paths.swap(a, b);
    }

    fn shift_right(&mut self) {
        let this_index = self.current_image_index;
        let other_index = (this_index + 1) % self.image_paths.len();
        self.swap_image_positions(this_index, other_index);
        self.current_image_index = other_index;
    }

    fn shift_left(&mut self) {
        let this_index = self.current_image_index;
        let other_index = (this_index + self.image_paths.len() - 1) % self.image_paths.len();
        self.swap_image_positions(this_index, other_index);
        self.current_image_index = other_index;
    }

    fn drop_current(&mut self) {
        self.image_paths.remove(self.current_image_index);
        if self.current_image_index == self.image_paths.len() {
            self.current_image_index = 0;
        }
        self.reload_texture().unwrap();
    }
}

#[derive(Debug)]
struct StableAspectRatioImageRenderer {
    image_renderer: ImageRenderer,
    window_size: [i32;2],
    scale: [f32;2],
    translate: [f32;2],
}

impl StableAspectRatioImageRenderer {
    pub fn new() -> StableAspectRatioImageRenderer {
        StableAspectRatioImageRenderer {
            image_renderer: ImageRenderer::new(),
            window_size: [1,1],
            scale: [1.0, 1.0],
            translate: [0.0, 0.0]
        }
    }

    fn resize_window(&mut self, size: [i32;2]) {
        self.window_size = size;
        self.recalculate_aspect_ratio();
    }

    fn recalculate_aspect_ratio(&mut self) {
        let view_width = (self.window_size[0] as f32) * self.scale[0];
        let view_height = (self.window_size[1] as f32) * self.scale[1];
        let view_aspect_ratio = view_width / view_height;

        let image_size = self.get_image_size();
        let image_aspect_ratio =
            (image_size[0] as f32) / (image_size[1] as f32);

        if view_aspect_ratio < image_aspect_ratio {
            let yscale = view_aspect_ratio / image_aspect_ratio;
            let scale = [self.scale[0], self.scale[1] * yscale];
            self.image_renderer.set_scale(scale);
        } else {
            let xscale = image_aspect_ratio / view_aspect_ratio;
            let scale = [self.scale[0] * xscale, self.scale[1]];
            self.image_renderer.set_scale(scale);
        }
    }

    pub fn set_texture_data(&mut self, texture: &Texture)
        -> Result<(), Box<dyn std::error::Error>>
    {
        self.image_renderer.set_texture_data(texture)?;
        self.recalculate_aspect_ratio();
        Ok(())
    }

    pub fn get_image_size(&self) -> [i32; 2] {
        self.image_renderer.get_image_size()
    }
}

impl Renderer for StableAspectRatioImageRenderer {
    fn render(&self) {
        self.image_renderer.render();
    }

    fn set_scale(&mut self, scale: [f32;2]) {
        self.scale = scale;
        self.recalculate_aspect_ratio();
    }

    fn set_translate(&mut self, translate: [f32;2]) {
        self.translate = translate;
        self.image_renderer.set_translate(translate);
    }
}
