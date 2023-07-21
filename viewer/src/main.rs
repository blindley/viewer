#![windows_subsystem = "windows"]

mod image_renderer;
use image_renderer::ImageRenderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    if args.len() < 1 {
        eprintln!("expected image filename");
        std::process::exit(-1);
    }

    
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
    .with_title("fuzzy pickles");
    
    let wc = glutin::ContextBuilder::new().build_windowed(wb, &el).unwrap();
    let wc = unsafe { wc.make_current().unwrap() };
    
    gl::load_with(|p| wc.get_proc_address(p) as *const _);
    
    let mut app_data = {
        let filename = args[0].clone();
        AppData::new(filename)
    };

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
                    use glutin::event::VirtualKeyCode::Escape;
                    match input.virtual_keycode {
                        Some(Escape) => *control_flow = ControlFlow::Exit,
                        _ => (),
                    }
                },

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
struct AppData {
    image_path: std::path::PathBuf,
    sig: FileSignature,
    window_size: [i32;2],
    renderer: ImageRenderer,
    
    seconds_elapsed: f32,
}

impl AppData {
    fn new<P: Into<std::path::PathBuf>>(image_path: P) -> AppData {
        let mut renderer = ImageRenderer::new();
        let image_path = image_path.into();
        renderer.set_texture_data(&image_path).unwrap();
        
        let image_path = image_path.into();
        let sig = FileSignature::new(&image_path).unwrap();
    
        let mut app_data = AppData {
            image_path,
            sig: sig,
            window_size: [1,1],
            renderer,
            seconds_elapsed: 0.0,
        };
    
        if let Err(_) = app_data.reload_texture() {
            eprintln!("failed to load {:?}", app_data.image_path);
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
        self.renderer.set_texture_data(&self.image_path)?;

        self.recalculate_aspect_ratio();
        Ok(())
    }

    fn resize_window(&mut self, size: [i32;2]) {
        self.window_size = size;
        self.recalculate_aspect_ratio();
        
        unsafe { gl::Viewport(0, 0, size[0], size[1]); }
    }

    fn recalculate_aspect_ratio(&self) {
        let window_aspect_ratio =
            (self.window_size[0] as f32) / (self.window_size[1] as f32);
        let image_size = self.renderer.get_image_size();
        let image_aspect_ratio =
            (image_size[0] as f32) / (image_size[1] as f32);

        if window_aspect_ratio < image_aspect_ratio {
            let yscale = window_aspect_ratio / image_aspect_ratio;
            self.renderer.set_scale([1.0, yscale]);
        } else {
            let xscale = image_aspect_ratio / window_aspect_ratio;
            self.renderer.set_scale([xscale, 1.0]);
        }
    }

    fn update(&mut self, seconds_elapsed: f32) -> bool {
        self.seconds_elapsed += seconds_elapsed;

        if self.seconds_elapsed >= 1.0 {
            // just reset it, we don't need a stable framerate
            self.seconds_elapsed = 0.0;

            // check if file has been modified
            if let Ok(sig) = FileSignature::new(&self.image_path) {
                if self.sig != sig {
                    self.sig = sig;
                    if self.reload_texture().is_ok() {
                        return true;
                    }
                }
            }
        }

        false
    }
}

