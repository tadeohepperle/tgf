use std::sync::Arc;

use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    monitor::MonitorHandle,
    platform::x11::WindowBuilderExtX11,
    window::{Window, WindowBuilder},
};

pub trait AppT {
    fn receive_window_event(&mut self, event: &WindowEvent);

    fn update(&mut self, cb: &mut RunnerCallbacks);
}

pub struct WindowConfig {
    pub window_name: &'static str,
    pub width: u32,
    pub height: u32,
    pub fullscreen: Option<MonitorPreference>,
}

pub enum MonitorPreference {
    Smallest,
    Largest,
    Primary,
}

impl WindowConfig {
    pub fn new() -> Self {
        Self {
            window_name: "Vert App",
            width: 1200,
            height: 700,
            fullscreen: None,
        }
    }

    pub fn fullscreen(mut self) -> Self {
        self.fullscreen = Some(MonitorPreference::Primary);
        self
    }

    pub fn largest_fullscreen(mut self) -> Self {
        self.fullscreen = Some(MonitorPreference::Largest);
        self
    }

    pub fn smallest_fullscreen(mut self) -> Self {
        self.fullscreen = Some(MonitorPreference::Smallest);
        self
    }
}
impl Default for WindowConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Runner {
    event_loop: EventLoop<()>,
    window: Arc<Window>,
}

impl Runner {
    pub fn window(&self) -> Arc<Window> {
        self.window.clone()
    }

    pub fn new(config: WindowConfig) -> Self {
        let (window, event_loop) = create_window_and_event_loop(config);
        let window = Arc::new(window);

        Self { event_loop, window }
    }

    pub fn run(self, app: &mut dyn AppT) -> anyhow::Result<()> {
        let window = self.window.clone();
        self.event_loop.run(move |event, window_target| {
            // check what kinds of events received:
            match &event {
                Event::NewEvents(_) => {}
                Event::WindowEvent { window_id, event } => {
                    if *window_id != self.window.id() {
                        return;
                    }

                    app.receive_window_event(event);

                    if matches!(event, WindowEvent::RedrawRequested) {
                        //  this is called every frame:
                        let mut cb = RunnerCallbacks::new();
                        app.update(&mut cb);

                        if let Some(reason) = cb.exit {
                            println!("Exit: {reason}");
                            window_target.exit();
                        } else {
                            window.request_redraw()
                        }
                    }
                }
                Event::DeviceEvent { .. } => {}
                Event::UserEvent(_) => {}
                Event::Suspended => {}
                Event::Resumed => {}
                Event::AboutToWait => {}
                Event::LoopExiting => {}
                Event::MemoryWarning => {}
            }
        })?;
        Ok(())
    }
}

fn select_monitor(event_loop: &EventLoop<()>, preference: MonitorPreference) -> MonitorHandle {
    if let MonitorPreference::Primary = preference {
        return event_loop.primary_monitor().unwrap();
    }

    let mut monitors: Vec<winit::monitor::MonitorHandle> =
        event_loop.available_monitors().collect();

    monitors.sort_by(|a, b| {
        dbg!(a.size(), b.size());
        let a = a.size().width + a.size().height;
        let b = b.size().width + b.size().height;
        a.cmp(&b)
    });

    match preference {
        MonitorPreference::Smallest => monitors.into_iter().next().unwrap(),
        MonitorPreference::Largest => monitors.into_iter().next_back().unwrap(),
        MonitorPreference::Primary => unreachable!(),
    }
}

pub struct RunnerCallbacks {
    /// String is the exit reason
    exit: Option<String>,
}

impl RunnerCallbacks {
    fn new() -> Self {
        Self { exit: None }
    }

    pub fn exit(&mut self, s: &str) {
        self.exit = Some(s.to_owned())
    }
}

pub fn create_window_and_event_loop(config: WindowConfig) -> (Window, EventLoop<()>) {
    let event_loop = EventLoop::new().unwrap();

    // let _video_mode = monitor.video_modes().next();
    // // let size = video_mode
    // //     .clone()
    // //     .map_or(PhysicalSize::new(800, 600), |vm| vm.size());

    let size = PhysicalSize::new(config.width, config.height);
    let mut window = WindowBuilder::new()
        .with_visible(true)
        .with_title(config.window_name)
        .with_inner_size(size)
        .with_resizable(true); //
                               // .with_base_size(size)

    if let Some(monitor) = config.fullscreen {
        let monitor = select_monitor(&event_loop, monitor);
        window = window.with_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(monitor))));
    };

    (window.build(&event_loop).unwrap(), event_loop)
}
