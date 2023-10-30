use std::collections::BTreeMap;

use internment::Intern;

use crate::hass_client::{responses::AreaRegistryList, HassRequestKind};

#[derive(Debug)]
pub struct Oracle {
    client: crate::hass_client::Client,
    rooms: BTreeMap<Intern<str>, Room>,
}

impl Oracle {
    pub async fn new(hass_client: crate::hass_client::Client) -> Self {
        let (rooms,) = tokio::join!(
            hass_client.request::<AreaRegistryList<'_>>(HassRequestKind::AreaRegistry)
        );

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
