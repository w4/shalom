use ::image::{imageops, GenericImageView, Pixel, Rgba, RgbaImage};
use iced::{
    advanced::svg::Handle,
    mouse::Cursor,
    widget::{
        canvas,
        canvas::{Cache, Geometry, LineDash, Path, Stroke, Style},
        image, svg, Canvas,
    },
    Color, Point, Rectangle, Renderer, Theme,
};
use once_cell::sync::Lazy;
use stackblur_iter::imgref::Img;
use usvg::{tiny_skia_path::PathSegment, NodeKind, Transform, TreeParsing};

pub mod colours {
    use iced::Color;

    macro_rules! colour {
        ($r:literal, $g:literal, $b:literal) => {{
            Color {
                r: $r / 255.0,
                g: $g / 255.0,
                b: $b / 255.0,
                a: 1.0,
            }
        }};
    }

    pub const SYSTEM_GRAY6: Color = colour!(28.0, 28.0, 30.0);
    pub const ORANGE: Color = colour!(255.0, 149.0, 0.0);

    pub const SLATE_200: Color = colour!(226.0, 232.0, 240.0);
    pub const SLATE_400: Color = colour!(148.0, 163.0, 184.0);

    pub const SKY_500: Color = colour!(14.0, 165.0, 233.0);
}

#[derive(Copy, Clone)]
pub enum Icon {
    Home,
    Back,
    Bulb,
    Hamburger,
    Speaker,
    SpeakerMuted,
    Backward,
    Forward,
    Play,
    Pause,
    Repeat,
    Repeat1,
    Cloud,
    ClearNight,
    Fog,
    Hail,
    Thunderstorms,
    ThunderstormsRain,
    PartlyCloudyDay,
    PartlyCloudyNight,
    ExtremeRain,
    Rain,
    Snow,
    ClearDay,
    Wind,
    Hvac,
    Shuffle,
    SpeakerFull,
    Dead,
    Search,
    Close,
}

impl Icon {
    pub fn data(self) -> &'static [u8] {
        macro_rules! image {
            ($path:expr) => {
                include_bytes!(concat!("../../assets/icons/", $path, ".svg"))
            };
        }

        match self {
            Self::Home => image!("home"),
            Self::Back => image!("back"),
            Self::Bulb => image!("light-bulb"),
            Self::Hamburger => image!("hamburger"),
            Self::Speaker => image!("speaker"),
            Self::SpeakerMuted => image!("speaker-muted"),
            Self::SpeakerFull => image!("speaker-full"),
            Self::Backward => image!("backward"),
            Self::Forward => image!("forward"),
            Self::Play => image!("play"),
            Self::Pause => image!("pause"),
            Self::Repeat => image!("repeat"),
            Self::Cloud => image!("cloud"),
            Self::ClearNight => image!("clear-night"),
            Self::Fog => image!("fog"),
            Self::Hail => image!("hail"),
            Self::Thunderstorms => image!("thunderstorms"),
            Self::ThunderstormsRain => image!("thunderstorms-rain"),
            Self::PartlyCloudyDay => image!("partly-cloudy-day"),
            Self::PartlyCloudyNight => image!("partly-cloudy-night"),
            Self::ExtremeRain => image!("extreme-rain"),
            Self::Rain => image!("rain"),
            Self::Snow => image!("snow"),
            Self::ClearDay => image!("clear-day"),
            Self::Hvac => image!("hvac"),
            Self::Wind => image!("wind"),
            Self::Shuffle => image!("shuffle"),
            Self::Repeat1 => image!("repeat-1"),
            Self::Dead => image!("dead"),
            Self::Search => image!("search"),
            Self::Close => image!("close"),
        }
    }

    pub fn handle(self) -> svg::Handle {
        macro_rules! image {
            ($v:expr) => {{
                static HANDLE: Lazy<svg::Handle> =
                    Lazy::new(|| svg::Handle::from_memory($v.data()));
                (*HANDLE).clone()
            }};
        }

        match self {
            Self::Home => image!(Icon::Home),
            Self::Back => image!(Icon::Back),
            Self::Bulb => image!(Icon::Bulb),
            Self::Hamburger => image!(Icon::Hamburger),
            Self::Speaker => image!(Icon::Speaker),
            Self::SpeakerMuted => image!(Icon::SpeakerMuted),
            Self::SpeakerFull => image!(Icon::SpeakerFull),
            Self::Backward => image!(Icon::Backward),
            Self::Forward => image!(Icon::Forward),
            Self::Play => image!(Icon::Play),
            Self::Pause => image!(Icon::Pause),
            Self::Repeat => image!(Icon::Repeat),
            Self::Cloud => image!(Icon::Cloud),
            Self::ClearNight => image!(Icon::ClearNight),
            Self::Fog => image!(Icon::Fog),
            Self::Hail => image!(Icon::Hail),
            Self::Thunderstorms => image!(Icon::Thunderstorms),
            Self::ThunderstormsRain => image!(Icon::ThunderstormsRain),
            Self::PartlyCloudyDay => image!(Icon::PartlyCloudyDay),
            Self::PartlyCloudyNight => image!(Icon::PartlyCloudyNight),
            Self::ExtremeRain => image!(Icon::ExtremeRain),
            Self::Rain => image!(Icon::Rain),
            Self::Snow => image!(Icon::Snow),
            Self::ClearDay => image!(Icon::ClearDay),
            Self::Hvac => image!(Icon::Hvac),
            Self::Wind => image!(Icon::Wind),
            Self::Shuffle => image!(Icon::Shuffle),
            Self::Repeat1 => image!(Icon::Repeat1),
            Self::Dead => image!(Icon::Dead),
            Self::Search => image!(Icon::Search),
            Self::Close => image!(Icon::Close),
        }
    }

    pub fn canvas<M>(self, color: Color) -> Canvas<IconCanvas, M, Renderer> {
        macro_rules! image {
            ($v:expr) => {{
                thread_local! {
                    static HANDLE: once_cell::unsync::Lazy<usvg::Tree> = once_cell::unsync::Lazy::new(|| usvg::Tree::from_data($v.data(), &usvg::Options::default()).unwrap());
                }

                HANDLE.with(|v| (*v).clone())
            }};
        }

        let svg = match self {
            Self::Home => image!(Icon::Home),
            Self::Back => image!(Icon::Back),
            Self::Bulb => image!(Icon::Bulb),
            Self::Hamburger => image!(Icon::Hamburger),
            Self::Speaker => image!(Icon::Speaker),
            Self::SpeakerMuted => image!(Icon::SpeakerMuted),
            Self::SpeakerFull => image!(Icon::SpeakerFull),
            Self::Backward => image!(Icon::Backward),
            Self::Forward => image!(Icon::Forward),
            Self::Play => image!(Icon::Play),
            Self::Pause => image!(Icon::Pause),
            Self::Repeat => image!(Icon::Repeat),
            Self::Cloud => image!(Icon::Cloud),
            Self::ClearNight => image!(Icon::ClearNight),
            Self::Fog => image!(Icon::Fog),
            Self::Hail => image!(Icon::Hail),
            Self::Thunderstorms => image!(Icon::Thunderstorms),
            Self::ThunderstormsRain => image!(Icon::ThunderstormsRain),
            Self::PartlyCloudyDay => image!(Icon::PartlyCloudyDay),
            Self::PartlyCloudyNight => image!(Icon::PartlyCloudyNight),
            Self::ExtremeRain => image!(Icon::ExtremeRain),
            Self::Rain => image!(Icon::Rain),
            Self::Snow => image!(Icon::Snow),
            Self::ClearDay => image!(Icon::ClearDay),
            Self::Hvac => image!(Icon::Hvac),
            Self::Wind => image!(Icon::Wind),
            Self::Shuffle => image!(Icon::Shuffle),
            Self::Repeat1 => image!(Icon::Repeat1),
            Self::Dead => image!(Icon::Dead),
            Self::Search => image!(Icon::Search),
            Self::Close => image!(Icon::Close),
        };

        canvas(IconCanvas {
            cache: Cache::new(),
            svg,
            color,
        })
    }
}

impl From<Icon> for svg::Handle {
    fn from(value: Icon) -> Handle {
        value.handle()
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum Image {
    LivingRoom,
    Kitchen,
    Bathroom,
    Bedroom,
    DiningRoom,
    Sunset,
    UnknownArtist,
}

impl Image {
    fn handle(self) -> image::Handle {
        macro_rules! image {
            ($path:expr) => {{
                static FILE: &[u8] = include_bytes!($path);
                static HANDLE: Lazy<image::Handle> = Lazy::new(|| {
                    let img = ::image::load_from_memory(FILE).unwrap();
                    let (h, w) = img.dimensions();
                    let data = img.into_rgba8().into_raw();
                    image::Handle::from_pixels(h, w, data)
                });
                (*HANDLE).clone()
            }};
        }

        match self {
            Image::LivingRoom => image!("../../assets/images/living_room.jpg"),
            Image::Kitchen => image!("../../assets/images/kitchen.jpg"),
            Image::Bathroom => image!("../../assets/images/bathroom.jpg"),
            Image::Bedroom => image!("../../assets/images/bedroom.jpg"),
            Image::DiningRoom => image!("../../assets/images/dining_room.jpg"),
            Image::Sunset => image!("../../assets/images/sunset-blur.jpg"),
            Image::UnknownArtist => image!("../../assets/images/unknown_artist.jpg"),
        }
    }

    pub fn preload() {
        Self::LivingRoom.handle();
        Self::Kitchen.handle();
        Self::Bathroom.handle();
        Self::Bedroom.handle();
        Self::DiningRoom.handle();
    }
}

impl From<Image> for image::Handle {
    fn from(value: Image) -> Self {
        value.handle()
    }
}

pub fn darken_image(mut img: RgbaImage, factor: f32) -> RgbaImage {
    for px in img.pixels_mut() {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        px.apply_without_alpha(|v| (f32::from(v) * (1.0 - factor)).min(255.0) as u8);
    }

    eprintln!("darkened");

    img
}

#[allow(clippy::many_single_char_names)]
pub fn blur(img: &RgbaImage, radius: usize) -> RgbaImage {
    let width = img.width();
    let height = img.height();

    let mut raw = img
        .pixels()
        .map(|p| u32::from_be_bytes([p.0[3], p.0[0], p.0[1], p.0[2]]))
        .collect::<Vec<_>>();

    stackblur_iter::par_blur_srgb(
        &mut Img::new(
            &mut raw,
            width.try_into().unwrap(),
            height.try_into().unwrap(),
        ),
        radius,
    );

    let mut image = RgbaImage::new(width, height);
    for (i, &pixel) in raw.iter().enumerate() {
        let x = u32::try_from(i).unwrap_or(u32::MAX) % width;
        let y = u32::try_from(i).unwrap_or(u32::MAX) / width;
        let [a, r, g, b] = pixel.to_be_bytes();
        image.put_pixel(x, y, Rgba([r, g, b, a]));
    }

    image
}

pub fn trim_transparent_padding(mut image: RgbaImage) -> RgbaImage {
    let (width, height) = image.dimensions();
    let mut top = 0;
    let mut bottom = height;
    let mut left = 0;
    let mut right = width;

    'outer: for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            if pixel[3] != 0 {
                top = y;
                break 'outer;
            }
        }
    }

    'outer: for y in (top..height).rev() {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            if pixel[3] != 0 {
                bottom = y + 1;
                break 'outer;
            }
        }
    }

    'outer: for x in 0..width {
        for y in top..bottom {
            let pixel = image.get_pixel(x, y);
            if pixel[3] != 0 {
                left = x;
                break 'outer;
            }
        }
    }

    'outer: for x in (left..width).rev() {
        for y in top..bottom {
            let pixel = image.get_pixel(x, y);
            if pixel[3] != 0 {
                right = x + 1;
                break 'outer;
            }
        }
    }

    imageops::crop(&mut image, left, top, right - left, bottom - top).to_image()
}

/// Opacity, rotation and other transforms aren't available on iced's svg
/// primitive, so we'll draw the svg onto a canvas we can apply transforms
/// to instead.
pub struct IconCanvas {
    cache: Cache,
    svg: usvg::Tree,
    color: Color,
}

impl<M> canvas::Program<M, Renderer> for IconCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let frame = self.cache.draw(renderer, bounds.size(), |frame| {
            let scale = bounds.width / self.svg.size.width();
            let translate_x = (bounds.width - self.svg.size.width() * scale) / 2.0;
            let translate_y = (bounds.height - self.svg.size.height() * scale) / 2.0;

            let transform =
                Transform::from_translate(translate_x, translate_y).post_scale(scale, scale);

            for node in self.svg.root.children() {
                if let NodeKind::Path(ref path) = *node.borrow() {
                    let builder = Path::new(|builder| {
                        for segment in path.data.segments() {
                            match segment {
                                PathSegment::MoveTo(mut p) => {
                                    transform.map_point(&mut p);
                                    let usvg::tiny_skia_path::Point { x, y } = p;
                                    builder.move_to(Point::new(x, y));
                                }
                                PathSegment::LineTo(mut p) => {
                                    transform.map_point(&mut p);
                                    let usvg::tiny_skia_path::Point { x, y } = p;
                                    builder.line_to(Point::new(x, y));
                                }
                                PathSegment::Close => {
                                    builder.close();
                                }
                                PathSegment::QuadTo(mut p1, mut p2) => {
                                    transform.map_point(&mut p1);
                                    transform.map_point(&mut p2);
                                    builder.quadratic_curve_to(
                                        Point::new(p1.x, p1.y),
                                        Point::new(p2.x, p2.y),
                                    );
                                }
                                PathSegment::CubicTo(mut p1, mut p2, mut p3) => {
                                    transform.map_point(&mut p1);
                                    transform.map_point(&mut p2);
                                    transform.map_point(&mut p3);
                                    builder.bezier_curve_to(
                                        Point::new(p1.x, p1.y),
                                        Point::new(p2.x, p2.y),
                                        Point::new(p3.x, p3.y),
                                    );
                                }
                            }
                        }
                    });

                    let stroke = if let Some(stroke) = &path.stroke {
                        Stroke {
                            style: Style::Solid(self.color),
                            width: stroke.width.get(),
                            line_cap: match stroke.linecap {
                                usvg::LineCap::Butt => canvas::LineCap::Butt,
                                usvg::LineCap::Round => canvas::LineCap::Round,
                                usvg::LineCap::Square => canvas::LineCap::Square,
                            },
                            line_join: match stroke.linejoin {
                                usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => {
                                    canvas::LineJoin::Miter
                                }
                                usvg::LineJoin::Round => canvas::LineJoin::Round,
                                usvg::LineJoin::Bevel => canvas::LineJoin::Bevel,
                            },
                            line_dash: LineDash::default(),
                        }
                    } else {
                        Stroke::default()
                    };

                    frame.stroke(&builder, stroke);
                }
            }
        });

        vec![frame]
    }
}
