use std::{any::TypeId, collections::BTreeMap, convert::identity, sync::Arc};

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    futures::StreamExt,
    subscription,
    widget::{column, container, image, scrollable, text, vertical_space, Column, Row},
    Font, Renderer, Subscription,
};
use itertools::Itertools;
use time::OffsetDateTime;
use url::Url;

use crate::{
    oracle::{Oracle, Weather},
    subscriptions::download_image,
    theme::Image,
    widgets::image_card,
};

#[derive(Debug)]
pub struct Omni {
    oracle: Arc<Oracle>,
    weather: Weather,
    cameras: BTreeMap<&'static str, CameraImage>,
}

#[derive(Debug)]
pub enum CameraImage {
    Unresolved(Url, Option<iced::widget::image::Handle>),
    Resolved(Url, iced::widget::image::Handle),
}

impl Omni {
    pub fn new(oracle: Arc<Oracle>) -> Self {
        Self {
            weather: oracle.current_weather(),
            cameras: oracle
                .cameras()
                .into_iter()
                .map(|(k, v)| (k, CameraImage::Unresolved(v.entity_picture, None)))
                .collect(),
            oracle,
        }
    }
}

impl Omni {
    #[allow(
        clippy::unnecessary_wraps,
        clippy::needless_pass_by_value,
        clippy::unused_self
    )]
    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::OpenRoom(room) => Some(Event::OpenRoom(room)),
            Message::UpdateWeather => {
                self.weather = self.oracle.current_weather();
                None
            }
            Message::UpdateCameras => {
                self.cameras = self
                    .oracle
                    .cameras()
                    .into_iter()
                    .map(|(k, v)| match self.cameras.remove(k) {
                        Some(CameraImage::Resolved(old_url, old_handle))
                            if old_url != v.entity_picture =>
                        {
                            (
                                k,
                                CameraImage::Unresolved(v.entity_picture, Some(old_handle)),
                            )
                        }
                        Some(CameraImage::Unresolved(old_url, old_handle))
                            if old_url != v.entity_picture =>
                        {
                            (k, CameraImage::Unresolved(v.entity_picture, old_handle))
                        }
                        Some(v) => (k, v),
                        None => (k, CameraImage::Unresolved(v.entity_picture, None)),
                    })
                    .collect();
                None
            }
            Message::CameraImageDownloaded(id, url, handle) => {
                self.cameras.insert(id, CameraImage::Resolved(url, handle));
                None
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let greeting = match OffsetDateTime::now_utc().hour() {
            5..=11 => "Good morning!",
            12..=16 => "Good afternoon!",
            17..=23 | 0..=4 => "Good evening!",
            _ => "Hello!",
        };

        let greeting = text(greeting).size(60).font(Font {
            weight: Weight::Bold,
            stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let room = |id, room, image| {
            image_card::image_card(image, room).on_press(Message::OpenRoom(id))
            // .height(Length::Fixed(128.0))
            // .width(Length::FillPortion(1))
        };

        let cameras = self
            .cameras
            .values()
            .map(|v| match v {
                CameraImage::Unresolved(_, Some(handle)) | CameraImage::Resolved(_, handle) => {
                    Element::from(image(handle.clone()).width(512.).height(288.))
                }
                CameraImage::Unresolved(..) => {
                    Element::from(container(vertical_space(0)).width(512.).height(288.))
                }
            })
            .chunks(2)
            .into_iter()
            .map(|children| children.into_iter().fold(Row::new(), Row::push))
            .fold(Column::new(), Column::push);

        let rooms = self
            .oracle
            .rooms()
            .map(|(id, r)| room(id, r.name.as_ref(), determine_image(&r.name)))
            .chunks(2)
            .into_iter()
            .map(|children| children.into_iter().fold(Row::new().spacing(10), Row::push))
            .fold(Column::new().spacing(10), Column::push);

        scrollable(
            column![
                greeting,
                crate::widgets::cards::weather::WeatherCard::new(self.weather),
                rooms,
                cameras,
            ]
            .spacing(20)
            .padding(40),
        )
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        pub struct WeatherSubscription;
        pub struct CameraSubscription;

        let weather_subscription = subscription::run_with_id(
            TypeId::of::<WeatherSubscription>(),
            self.oracle
                .subscribe_weather()
                .map(|()| Message::UpdateWeather),
        );

        let camera_subscription = subscription::run_with_id(
            TypeId::of::<CameraSubscription>(),
            self.oracle
                .subscribe_all_cameras()
                .map(|()| Message::UpdateCameras),
        );

        let camera_image_downloads =
            Subscription::batch(self.cameras.iter().filter_map(|(k, v)| {
                if let CameraImage::Unresolved(url, _) = v {
                    let k = *k;
                    let url = url.clone();

                    Some(download_image(url.clone(), identity, move |handle| {
                        Message::CameraImageDownloaded(k, url, handle)
                    }))
                } else {
                    None
                }
            }));

        Subscription::batch([
            weather_subscription,
            camera_subscription,
            camera_image_downloads,
        ])
    }
}

fn determine_image(name: &str) -> Image {
    match name {
        "Kitchen" => Image::Kitchen,
        "Bathroom" => Image::Bathroom,
        "Bedroom" => Image::Bedroom,
        "Dining Room" => Image::DiningRoom,
        _ => Image::LivingRoom,
    }
}

#[derive(Default, Hash)]
pub struct State {}

#[derive(Clone, Debug)]
pub enum Event {
    OpenRoom(&'static str),
}

#[derive(Clone, Debug)]
pub enum Message {
    OpenRoom(&'static str),
    UpdateWeather,
    UpdateCameras,
    CameraImageDownloaded(&'static str, Url, iced::widget::image::Handle),
}
