use std::{collections::BTreeMap, sync::Arc};

use iced::{
    futures::StreamExt, subscription, widget::Row, Element, Length, Renderer, Subscription,
};

use crate::{
    oracle::{Light, Oracle, Room},
    theme::Icon,
    widgets::{self, colour_picker::colour_from_hsb},
};

#[derive(Debug)]
pub struct Lights {
    lights: BTreeMap<&'static str, Light>,
    oracle: Arc<Oracle>,
}

impl Lights {
    pub fn new(oracle: Arc<Oracle>, room: &Room) -> Self {
        let lights = room.lights(&oracle);

        Self { lights, oracle }
    }

    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::SetLightState(id, state) => {
                // give instant feedback before we get the event back from hass
                if let Some(light) = self.lights.get_mut(id) {
                    light.on = Some(state);
                }

                Some(Event::SetLightState(id, state))
            }
            Message::OpenLightOptions(id) => Some(Event::OpenLightContextMenu(id)),
            Message::UpdateLight(entity_id) => {
                if let Some(light) = self.oracle.fetch_light(entity_id) {
                    self.lights.insert(entity_id, light);
                }

                None
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let light = |id, light: &Light| {
            let mut toggle_card = widgets::toggle_card::toggle_card(
                &light.friendly_name,
                light.on.unwrap_or_default(),
                light.on.is_none(),
            )
            .icon(if light.on.is_none() {
                Icon::Dead
            } else {
                Icon::Bulb
            })
            .width(Length::Shrink)
            .active_icon_colour(
                light
                    .hs_color
                    .zip(light.brightness)
                    .map(|((h, s), b)| colour_from_hsb(h, s, b / 255.)),
            );

            if let Some(state) = light.on {
                toggle_card = toggle_card
                    .on_press(Message::SetLightState(id, !state))
                    .on_long_press(Message::OpenLightOptions(id));
            }

            toggle_card
        };

        Row::with_children(
            self.lights
                .iter()
                .map(|(id, item)| light(*id, item))
                .map(Element::from)
                .collect::<Vec<_>>(),
        )
        .spacing(10)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(self.lights.keys().copied().map(|key| {
            subscription::run_with_id(
                key,
                self.oracle
                    .subscribe_id(key)
                    .map(|()| Message::UpdateLight(key)),
            )
        }))
    }
}

#[derive(Copy, Clone)]
pub enum Event {
    OpenLightContextMenu(&'static str),
    SetLightState(&'static str, bool),
}

#[derive(Clone, Debug, Copy)]
pub enum Message {
    SetLightState(&'static str, bool),
    UpdateLight(&'static str),
    OpenLightOptions(&'static str),
}
