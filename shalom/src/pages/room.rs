use std::sync::Arc;

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    futures::StreamExt,
    subscription,
    widget::{container, image::Handle, row, text, Column},
    Font, Renderer, Subscription,
};
use internment::Intern;
use url::Url;

use crate::{
    oracle::{MediaPlayerSpeaker, Oracle},
    subscriptions::download_image,
    theme::Icon,
    widgets,
};

#[derive(Debug)]
pub struct Room {
    oracle: Arc<Oracle>,
    room: crate::oracle::Room,
    speaker: Option<MediaPlayerSpeaker>,
    now_playing_image: Option<Handle>,
}

impl Room {
    pub fn new(id: &'static str, oracle: Arc<Oracle>) -> Self {
        let room = oracle.room(id).clone();
        let speaker = room.speaker(&oracle);

        Self {
            oracle,
            room,
            speaker,
            now_playing_image: None,
        }
    }

    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::LightToggle(_name) => {
                // let x = state.lights.entry(name).or_default();
                // if *x == 0 {
                //     *x = 1;
                // } else {
                //     *x = 0;
                // }
                //
                None
            }
            Message::OpenLightOptions(name) => Some(Event::OpenLightContextMenu(name)),
            Message::UpdateLightAmount(_name, _v) => {
                // let x = state.lights.entry(name).or_default();
                // *x = v;
                None
            }
            Message::NowPlayingImageLoaded(url, handle) => {
                if self
                    .speaker
                    .as_ref()
                    .and_then(|v| v.entity_picture.as_ref())
                    == Some(&url)
                {
                    self.now_playing_image = Some(handle);
                }

                None
            }
            Message::UpdateSpeaker => {
                let new = self.room.speaker(&self.oracle);

                if self
                    .speaker
                    .as_ref()
                    .and_then(|v| v.entity_picture.as_ref())
                    != new
                        .as_ref()
                        .as_ref()
                        .and_then(|v| v.entity_picture.as_ref())
                {
                    self.now_playing_image = None;
                }

                self.speaker = new;

                None
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let header = text(self.room.name.as_ref()).size(60).font(Font {
            weight: Weight::Bold,
            stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let light = |name| {
            widgets::toggle_card::toggle_card(name, false)
                .icon(Icon::Bulb)
                .on_press(Message::LightToggle(name))
                .on_long_press(Message::OpenLightOptions(name))
        };

        let mut col = Column::new().spacing(20).padding(40).push(header);

        if let Some(speaker) = self.speaker.clone() {
            col = col.push(
                container(widgets::media_player::media_player(
                    speaker,
                    self.now_playing_image.clone(),
                ))
                .padding([12, 0, 24, 0]),
            );
        }

        col = col.push(row![light("Main"), light("Lamp"), light("TV")].spacing(10));

        col.into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let image_subscription = if let (Some(uri), None) = (
            self.speaker
                .as_ref()
                .and_then(|v| v.entity_picture.as_ref()),
            &self.now_playing_image,
        ) {
            download_image(uri.clone(), uri.clone(), Message::NowPlayingImageLoaded)
        } else {
            Subscription::none()
        };

        let speaker_subscription =
            if let Some(speaker_id) = self.room.speaker_id.map(Intern::as_ref) {
                subscription::run_with_id(
                    speaker_id,
                    self.oracle
                        .subscribe_id(speaker_id)
                        .map(|()| Message::UpdateSpeaker),
                )
            } else {
                Subscription::none()
            };

        Subscription::batch([image_subscription, speaker_subscription])
    }
}

pub enum Event {
    OpenLightContextMenu(&'static str),
}

#[derive(Clone, Debug)]
pub enum Message {
    NowPlayingImageLoaded(Url, Handle),
    LightToggle(&'static str),
    OpenLightOptions(&'static str),
    UpdateLightAmount(&'static str, u8),
    UpdateSpeaker,
}
