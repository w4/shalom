use std::{collections::BTreeMap, str::FromStr};

use internment::Intern;

use crate::hass_client::{
    responses::{AreaRegistryList, StateAttributes, StatesList, WeatherCondition},
    HassRequestKind,
};

#[derive(Debug)]
pub struct Oracle {
    client: crate::hass_client::Client,
    rooms: BTreeMap<Intern<str>, Room>,
    pub weather: Weather,
}

impl Oracle {
    pub async fn new(hass_client: crate::hass_client::Client) -> Self {
        let (rooms, states) = tokio::join!(
            hass_client.request::<AreaRegistryList<'_>>(HassRequestKind::AreaRegistry),
            hass_client.request::<StatesList<'_>>(HassRequestKind::GetStates),
        );

        let states = states.get();

        let rooms = rooms
            .get()
            .0
            .iter()
            .map(|room| {
                (
                    Intern::from(room.area_id.as_ref()),
                    Room {
                        name: Intern::from(room.name.as_ref()),
                    },
                )
            })
            .collect();

        Self {
            client: hass_client,
            rooms,
            weather: Weather::parse_from_states(states),
        }
    }

    pub fn rooms(&self) -> impl Iterator<Item = &'_ Room> + '_ {
        self.rooms.values()
    }
}

#[derive(Debug)]
pub struct Room {
    pub name: Intern<str>,
}

#[derive(Debug)]
pub struct Weather {
    pub temperature: i16,
    pub high: i16,
    pub low: i16,
    pub condition: WeatherCondition,
}

impl Weather {
    fn parse_from_states(states: &StatesList) -> Self {
        let (state, weather) = states
            .0
            .iter()
            .filter_map(|v| match &v.attributes {
                StateAttributes::Weather(attr) => Some((&v.state, attr)),
                _ => None,
            })
            .next()
            .unwrap();

        let condition = WeatherCondition::from_str(&state).unwrap_or(WeatherCondition::Unknown);

        let (high, low) =
            weather
                .forecast
                .iter()
                .fold((i16::MIN, i16::MAX), |(high, low), curr| {
                    let temp = curr.temperature.round() as i16;

                    (high.max(temp), low.min(temp))
                });

        Self {
            temperature: weather.temperature.round() as i16,
            condition,
            high,
            low,
        }
    }
}
