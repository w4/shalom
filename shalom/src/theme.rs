use ::image::{GenericImageView, Pixel, Rgba, RgbaImage};
use iced::{
    advanced::svg::Handle,
    widget::{image, svg},
};
use once_cell::sync::Lazy;
use stackblur_iter::imgref::Img;

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

    pub const SLATE_200: Color = colour!(226.0, 232.0, 240.0);
    pub const SLATE_300: Color = colour!(203.0, 213.0, 225.0);
    pub const SLATE_400: Color = colour!(148.0, 163.0, 184.0);
    pub const SLATE_600: Color = colour!(71.0, 85.0, 105.0);

    pub const SKY_400: Color = colour!(56.0, 189.0, 248.0);
    pub const SKY_500: Color = colour!(14.0, 165.0, 233.0);

    pub const AMBER_200: Color = colour!(253.0, 230.0, 138.0);
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
}

impl Icon {
    pub fn handle(self) -> svg::Handle {
        macro_rules! image {
            ($path:expr) => {{
                static FILE: &[u8] = include_bytes!(concat!("../../assets/icons/", $path, ".svg"));
                static HANDLE: Lazy<svg::Handle> = Lazy::new(|| svg::Handle::from_memory(FILE));
                (*HANDLE).clone()
            }};
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
        }
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
