use wayland_client::protocol::{
    wl_compositor::WlCompositor,
    wl_output::{Event, WlOutput},
    wl_seat::WlSeat,
    wl_shm::WlShm,
    wl_surface::WlSurface,
};
use wayland_client::{Display, EventQueue, GlobalManager, Main};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

#[derive(Debug)]
pub struct Output {
    pub wl_output: Main<WlOutput>,
    pub name: String,
    pub scale: i32,
    pub width: i32,
    pub height: i32,
    configured: bool,
}

impl Output {
    fn new(wl_output: Main<WlOutput>) -> Output {
        Output {
            wl_output,
            name: String::new(),
            scale: 1,
            height: 0,
            width: 0,
            configured: false,
        }
    }
}

#[derive(Debug)]
pub struct Environment {
    pub outputs: Vec<Output>,
    pub seats: Vec<Main<WlSeat>>,
    pub shm: Option<Main<WlShm>>,
    pub compositor: Option<Main<WlCompositor>>,
    pub layer_shell: Option<Main<ZwlrLayerShellV1>>,
}

impl Environment {
    pub fn new(display: &Display, event_queue: &mut EventQueue) -> Environment {
        let attached_display = (*display).clone().attach(event_queue.token());
        let mut environment = Environment {
            compositor: None,
            layer_shell: None,
            shm: None,
            seats: Vec::new(),
            outputs: Vec::new(),
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
                    |output: Main<WlOutput>, mut environment: DispatchData| {
                        let mut clock = 0;
                        output.quick_assign(move |wl_output, event, mut output_handle| {
                            let output_handle = output_handle.get::<Vec<Output>>().unwrap();
                            for output in output_handle {
                                if !output.configured {
                                    match &event {
                                        Event::Geometry {
                                            x: _,
                                            y: _,
                                            physical_width: _,
                                            physical_height: _,
                                            subpixel: _,
                                            make,
                                            model: _,
                                            transform: _,
                                        } => {
                                            output.name = make.to_string();
                                        }
                                        Event::Mode {
                                            flags: _,
                                            width,
                                            height,
                                            refresh: _,
                                        } => {
                                            output.width = *width;
                                            output.height = *height;
                                        }
                                        Event::Scale { factor } => {
                                            output.scale = *factor;
                                        }
                                        _ => {}
                                    }
                                    if clock == 3 {
                                        output.configured = true;
                                        output.wl_output = wl_output.clone();
                                    }
                                    clock += 1;
                                } else {
                                    break;
                                }
                            }
                        });
                        environment
                            .get::<Environment>()
                            .unwrap()
                            .outputs
                            .push(Output::new(output));
                    }
                ]
            ),
        );
        event_queue
            .sync_roundtrip(&mut environment, |_, _, _| unreachable!())
            .unwrap();

        event_queue
            .sync_roundtrip(&mut environment.outputs, |_, _, _| unreachable!())
            .unwrap();

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
