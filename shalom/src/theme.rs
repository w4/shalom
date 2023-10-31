use ::image::GenericImageView;
use iced::{
    advanced::svg::Handle,
    widget::{image, svg},
};
use once_cell::sync::Lazy;

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
            Self::Wind => image!("wind"),
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
