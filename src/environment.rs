use super::app;
use snui::wayland::Buffer;
use wayland_client::protocol::{
    wl_compositor::WlCompositor,
    wl_output::{Event, WlOutput},
    wl_seat::WlSeat,
    wl_shm::WlShm,
    wl_surface::WlSurface,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1,
};
use std::time;
use smithay_client_toolkit::shm::DoubleMemPool;
use wayland_client::{Display, EventQueue, GlobalManager, Main};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;

pub struct Environment {
    pub paper: app::Paper,
    pub seats: Vec<Main<WlSeat>>,
    pub shm: Option<Main<WlShm>>,
    pub compositor: Option<Main<WlCompositor>>,
    pub layer_shell: Option<Main<ZwlrLayerShellV1>>,
}

impl Environment {
    pub fn new(display: &Display, event_queue: &mut EventQueue, paper: app::Paper) -> Environment {
        let attached_display = (*display).clone().attach(event_queue.token());
        let environment = Environment {
            paper,
            compositor: None,
            layer_shell: None,
            shm: None,
            seats: Vec::new(),
        };

        GlobalManager::new_with_cb(
            &attached_display,
            wayland_client::global_filter!(
                [
                    ZwlrLayerShellV1,
                    1,
                    |layer_shell: Main<ZwlrLayerShellV1>, mut environment: DispatchData| {
                        environment.get::<Environment>().unwrap().layer_shell = Some(layer_shell);
                    }
                ],
                [
                    WlShm,
                    1,
                    |wl_shm: Main<WlShm>, mut environment: DispatchData| {
                        wl_shm.quick_assign(move |_, _, _| {});
                        environment.get::<Environment>().unwrap().shm = Some(wl_shm);
                    }
                ],
                [
                    WlSeat,
                    7,
                    |wl_seat: Main<WlSeat>, mut environment: DispatchData| {
                        wl_seat.quick_assign(move |_, _, _| {});
                        environment
                            .get::<Environment>()
                            .unwrap()
                            .seats
                            .push(wl_seat);
                    }
                ],
                [
                    WlCompositor,
                    4,
                    |wl_compositor: Main<WlCompositor>, mut environment: DispatchData| {
                        environment.get::<Environment>().unwrap().compositor = Some(wl_compositor);
                    }
                ],
                [
                    WlOutput,
                    3,
                    move |output: Main<WlOutput>, mut environment: DispatchData| {
                        if let Some(env) = environment.get::<Environment>() {
                            let surface = env.get_surface();
                            if env.layer_shell.is_some() && env.compositor.is_some() && env.shm.is_some() {
                                let mut draw = true;
                                let layer_surface = env
                                    .layer_shell
                                    .as_ref()
                                    .expect("Compositor doesn't implement the LayerShell protocol")
                                    .get_layer_surface(&surface, Some(&output), Layer::Background, String::from("wallpaper"));
                                let attached = Attached::from(env.shm.clone().expect("No shared memory pool"));
                                output.quick_assign(move |_, event, mut env| match event {
                                    Event::Geometry {
                                        x: _,
                                        y: _,
                                        physical_width: _,
                                        physical_height: _,
                                        subpixel: _,
                                        make,
                                        model: _,
                                        transform: _,
                                    } =>  if let Some(env) = env.get::<Environment>() {
                                        if let Some(output) = env.paper.output.as_ref() {
                                            draw = output.eq(&make);
                                        }
                                    }
                                    Event::Mode {
                                        flags: _,
                                        width,
                                        height,
                                        refresh: _,
                                    } => {
                                        if draw {
                                            let surface = surface.clone();
                                            layer_surface.set_size(width as u32, height as u32);
                                            if let Some(env) = env.get::<Environment>() {
                                                if env.paper.border.is_some() {
                                                    layer_surface.set_exclusive_zone(1);
                                                } else {
                                                    layer_surface.set_exclusive_zone(-1);
                                                }
                                            }
                                            surface.commit();
                                            let mut mempool = DoubleMemPool::new(attached.clone(), |_| {}).unwrap();
                                            let mut timer: Option<time::Instant> = None;
                                            layer_surface.quick_assign(move |layer_surface, event, mut env| match event {
                                                zwlr_layer_surface_v1::Event::Configure{serial, width, height} => {
                                                    if timer.is_none() || timer.as_ref().unwrap().elapsed() > time::Duration::from_millis(300) {
                                                        timer = Some(time::Instant::now());
                                                        layer_surface.ack_configure(serial);

														if let Some(mut pool) = mempool.pool() {
    														if pool.resize((width * height) as usize * 4).is_ok() {
                                                                if let Ok(mut buf) = Buffer::new(
                                                                    width,
                                                                    height,
                                                                    &mut pool,
                                                                ) {
                                                                    if let Some(env) = env.get::<Environment>() {
                        												app::draw(&mut buf, &mut env.paper, width, height);
                                                                        buf.attach(&surface, 0, 0);
                                                                        surface.damage(
                                                                            0,
                                                                            0,
                                                                            1 << 30,
                                                                            1 << 30
                                                                        );
                                                                        surface.commit();
                                                                    }
        														}
                                                            }
														}
                                                    }
                                                }
                                                _ => {
                                                    layer_surface.destroy();
                                                }
                                            });
                                        }
                                    }
                                    Event::Scale { factor } => {
                                        surface.set_buffer_scale(factor);
                                    }
                                    _ => {}
                                });
                            }
                        }
                    }
                ]
            ),
        );
        environment
    }
    pub fn get_surface(&self) -> Main<WlSurface> {
        let wl_surface = self
            .compositor
            .as_ref()
            .expect("Compositor literally doesn't exist")
            .create_surface();
        wl_surface.quick_assign(move |_, _, _| {});
        wl_surface
    }
}
