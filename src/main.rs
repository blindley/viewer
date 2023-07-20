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

    el.run(move |event, _, control_flow| {
        use glutin::event::{Event, WindowEvent, StartCause};
        use glutin::event_loop::ControlFlow;

        *control_flow = ControlFlow::WaitUntil(app_data.next_update_time);

        match event {
            Event::LoopDestroyed => return,

            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached { .. } => {
                    app_data.next_update_time += app_data.time_between_updates;
                    *control_flow = ControlFlow::WaitUntil(app_data.next_update_time);

                    if let Ok(sig) = FileSignature::new(&app_data.image_path) {
                        if app_data.sig != sig {
                            app_data.sig = sig;
                            if app_data.reload_texture().is_ok() {
                                wc.window().request_redraw()
                            }
                        }
                    }
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

#[derive(Debug, Clone)]
struct AppData {
    image_path: std::path::PathBuf,
    sig: FileSignature,
    window_size: [i32;2],
    next_update_time: std::time::Instant,
    time_between_updates: std::time::Duration,
    renderer: ImageRenderer,
}

impl AppData {
    fn new<P: Into<std::path::PathBuf>>(image_path: P) -> AppData {
        let mut renderer = ImageRenderer::new();
        let image_path = image_path.into();
        renderer.set_texture_data(&image_path).unwrap();
        
        let time_between_updates = std::time::Duration::new(1, 0);
        let image_path = image_path.into();
        let sig = FileSignature::new(&image_path).unwrap();
    
        let mut app_data = AppData {
            image_path,
            sig: sig,
            window_size: [1,1],
            time_between_updates: time_between_updates,
            next_update_time: std::time::Instant::now() + time_between_updates,
            renderer,
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
        let image_size = self.renderer.get_size();
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
}

