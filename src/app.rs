use snui::snui::*;
use snui::wayland::*;
use snui::widgets::*;
use wayland_client::protocol::{
    wl_surface::WlSurface,
};
use std::path::Path;
use wayland_client::Main;
use smithay_client_toolkit::shm::AutoMemPool;
use std::io::{Write, BufWriter};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1,
    zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
};

#[derive(Clone, Debug)]
pub struct Paper {
    style: Style,
    border: Option<(u32, u32)>,
}

impl Paper {
    pub fn is_some(&self) -> bool {
        match self.style {
            Style::None => false,
            _ => true
        }
    }
    pub fn default() -> Self {
        Paper {
            style: Style::None,
            border: None
        }
    }
    pub fn style(&mut self, style: Style) {
        self.style = style
    }
    pub fn border(&mut self, gap: u32, color: u32) {
        self.border = Some((gap, color));
   }
}

#[derive(Clone, Debug)]
pub enum Style {
    Color(u32),
    Tiled(String),
    Image(String),
    None,
}

pub struct Snape {
    width: i32,
    height: i32,
    pub mempool: AutoMemPool,
    pub surface: Main<WlSurface>,
    pub layer_surface: Main<ZwlrLayerSurfaceV1>,
}

impl Snape {
    pub fn new(
        width: i32,
        height: i32,
        surface: Main<WlSurface>,
        layer_surface: Main<ZwlrLayerSurfaceV1>,
        mempool: AutoMemPool
    ) -> Snape {
        layer_surface.set_size(width as u32, height as u32);
        surface.commit();
        Snape {
            width,
            height,
            mempool,
            layer_surface,
            surface
        }
    }
    fn destroy(&self) {
        self.surface.destroy();
        self.layer_surface.destroy();
    }
    fn draw(&mut self, paper: &Paper, width: u32, height: u32) {
        let mut buffer = Buffer::new(
            self.width,
            self.height,
            (4 * self.width) as i32,
            &mut self.mempool,
        );
        match &paper.style {
            Style::Color(color) => {
                let pxcount = buffer.size()/4;
                self.layer_surface.set_exclusive_zone(-1);
                let mut writer = BufWriter::new(buffer.get_mut_buf());
                for _ in 0..pxcount {
                    writer.write_all(&color.to_ne_bytes()).unwrap();
                }
                writer.flush().unwrap();
            }
            Style::Image(path) => {
                let path = Path::new(&path);
                self.layer_surface.set_exclusive_zone(-1);
                let image = Image::new_with_size(path, self.width as u32, self.height as u32).unwrap();
                buffer.composite(&to_surface(&image), 0, 0);
            }
            Style::Tiled(path) => {
                let path = Path::new(&path);
                self.layer_surface.set_exclusive_zone(-1);
                let bg = tile(path, self.width as u32, self.height as u32);
                buffer.composite(&bg, 0, 0);
            }
            _ => {}
        }
        if let Some((gap, color)) = paper.border {
            self.layer_surface.set_exclusive_zone(0);
            let color = Content::Pixel(color);
            let border_hor = Surface::new(width, gap, color).unwrap();
            let border_ver = Surface::new(gap, height, color).unwrap();
            buffer.composite(&border_hor, 0, 0);
            buffer.composite(&border_hor, 0, height-gap);
            buffer.composite(&border_ver, 0, 0);
            buffer.composite(&border_ver, width-gap, 0);
        }
        buffer.attach(&self.surface, 0, 0);
        self.surface.damage(
            0,
            0,
            1 << 30,
            1 << 30
        );
        self.surface.damage_buffer(
            0,
            0,
            1 << 30,
            1 << 30
        );
    }
    pub fn dispatch_surface(mut self, paper: Paper) {
        self.layer_surface.clone().quick_assign(move |layer_surface, event, _| {
            match event {
                zwlr_layer_surface_v1::Event::Configure {
                    serial,
                    width,
                    height,
                } => {
                    layer_surface.ack_configure(serial);
                    layer_surface.set_size(width, height);
                    self.mempool.resize((width * height) as usize).unwrap();

                    // The client should use commit to notify itself
                    // that it has been configured
                    // The client is also responsible for damage
                    self.draw(&paper, width, height);
                    self.surface.commit();
                }
                zwlr_layer_surface_v1::Event::Closed => {
                    self.destroy();
                }
                _ => {}
            }
        });
    }
}

pub fn tile(path: &Path, width: u32, height: u32) -> Surface {
    let mut y = 0;
    let image = Image::new(path);
    let img_width = image.as_ref().unwrap().get_width();
    let img_height = image.as_ref().unwrap().get_height();
    let surface_width = img_width * (width as f64/img_width as f64).ceil() as u32;
    let surface_height = img_height * (height as f64/img_height as f64).ceil() as u32;
    let mut surface = Surface::empty(surface_width, surface_height);
    while y < height {
        let mut x = 0;
        while x < width {
            image.as_ref().unwrap().draw(&mut surface, x, y);
            x += img_width;
        }
        y += img_height;
    }
    surface
}
