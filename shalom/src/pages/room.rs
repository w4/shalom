use std::collections::HashMap;

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    widget::{column, component, container, row, text, Component},
    Font, Renderer,
};

use crate::{theme::Icon, widgets, ActiveContextMenu};

pub struct Room<M> {
    name: &'static str,
    open_context_menu: fn(ActiveContextMenu) -> M,
}

impl<M> Room<M> {
    pub fn new(name: &'static str, open_context_menu: fn(ActiveContextMenu) -> M) -> Self {
        Self {
            name,
            open_context_menu,
        }
    }
}

impl<M: Clone> Component<M, Renderer> for Room<M> {
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            Event::LightToggle(name) => {
                let x = state.lights.entry(name).or_default();
                if *x == 0 {
                    *x = 1;
                } else {
                    *x = 0;
                }

                None
            }
            Event::OpenLightOptions(name) => Some((self.open_context_menu)(
                ActiveContextMenu::LightOptions(name),
            )),
            Event::UpdateLightAmount(name, v) => {
                let x = state.lights.entry(name).or_default();
                *x = v;
                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let header = text(self.name).size(60).font(Font {
            weight: Weight::Bold,
            stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let light = |name| {
            widgets::toggle_card::toggle_card(
                name,
                state.lights.get(name).copied().unwrap_or_default() > 0,
            )
            .icon(Icon::Bulb)
            .on_press(Event::LightToggle(name))
            .on_long_press(Event::OpenLightOptions(name))
        };

        column![
            header,
            container(widgets::media_player::media_player()).padding([12, 0, 24, 0]),
            row![light("Main"), light("Lamp"), light("TV")].spacing(10),
        ]
        .spacing(20)
        .padding(40)
        .into()
    }
}

#[derive(Default)]
pub struct State {
    lights: HashMap<&'static str, u8>,
}

#[derive(Clone)]
pub enum Event {
    LightToggle(&'static str),
    OpenLightOptions(&'static str),
    UpdateLightAmount(&'static str, u8),
}

impl<'a, M> From<Room<M>> for Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(card: Room<M>) -> Self {
        component(card)
    }
}
