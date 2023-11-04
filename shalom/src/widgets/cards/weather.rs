use iced::{
    advanced::{
        layout::{Limits, Node},
        renderer::{Quad, Style},
        svg::Renderer as SvgRenderer,
        text::{LineHeight, Renderer as TextRenderer, Shaping},
        widget::Tree,
        Layout, Renderer as AdvancedRenderer, Text, Widget,
    },
    alignment::{Horizontal, Vertical},
    font::Weight,
    gradient::Linear,
    mouse::Cursor,
    Alignment, Background, Color, Degrees, Element, Font, Gradient, Length, Rectangle, Renderer,
    Size, Theme,
};
use time::OffsetDateTime;

use crate::oracle::Weather;

#[allow(clippy::module_name_repetitions)]
pub struct WeatherCard<M> {
    pub on_click: Option<M>,
    pub current_weather: Weather,
}

impl<M> WeatherCard<M> {
    pub fn new(current_weather: Weather) -> Self {
        Self {
            current_weather,
            on_click: None,
        }
    }

    fn build_temperature(&self) -> String {
        format!("{}°", self.current_weather.temperature)
    }

    fn build_conditions(&self) -> String {
        format!(
            "{}\nH:{}° L:{}°",
            self.current_weather.weather_condition(),
            self.current_weather.high,
            self.current_weather.low,
        )
    }
}

impl<M: Clone> Widget<M, Renderer> for WeatherCard<M> {
    fn width(&self) -> Length {
        Length::Fixed(192.)
    }

    fn height(&self) -> Length {
        Length::Fixed(192.)
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        let padding = 16.into();

        let limits = limits
            .height(self.height())
            .width(self.width())
            .pad(padding);
        let container_size = limits.resolve(Size::ZERO);

        let mut header_node = Node::new(renderer.measure(
            &self.build_temperature(),
            42.,
            LineHeight::default(),
            Font {
                weight: Weight::Normal,
                ..Font::with_name("Helvetica Neue")
            },
            container_size,
            Shaping::Basic,
        ));
        header_node.move_to([padding.top, padding.left].into());
        header_node.align(Alignment::Start, Alignment::Start, container_size);

        let mut icon_node =
            Node::new(Size::new(16., 16.)).translate([padding.left, -padding.bottom - 32.].into());
        icon_node.align(Alignment::Start, Alignment::End, container_size);

        let mut conditions_node = Node::new(renderer.measure(
            &self.build_conditions(),
            12.,
            LineHeight::default(),
            Font {
                weight: Weight::Bold,
                ..Font::with_name("Helvetica Neue")
            },
            container_size,
            Shaping::Basic,
        ))
        .translate([padding.left, -padding.bottom].into());
        conditions_node.align(Alignment::Start, Alignment::End, container_size);

        Node::with_children(
            container_size,
            vec![header_node, icon_node, conditions_node],
        )
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        // TODO: get sunrise/sunset from somewhere reasonable
        let day_time = matches!(OffsetDateTime::now_utc().hour(), 5..=19);

        let gradient = if day_time {
            Linear::new(Degrees(90.))
                .add_stop(0.0, Color::from_rgba8(104, 146, 190, 1.0))
                .add_stop(1.0, Color::from_rgba8(10, 54, 120, 1.0))
        } else {
            Linear::new(Degrees(90.))
                .add_stop(0.0, Color::from_rgba8(43, 44, 66, 1.0))
                .add_stop(1.0, Color::from_rgba8(15, 18, 27, 1.0))
        };

        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                border_radius: [20., 20., 20., 20.].into(),
                border_width: 0.,
                border_color: Color::WHITE,
            },
            Background::Gradient(Gradient::Linear(gradient)),
        );

        let mut children = layout.children();

        renderer.fill_text(Text {
            content: &self.build_temperature(),
            bounds: children.next().unwrap().bounds(),
            size: 42.,
            line_height: LineHeight::default(),
            color: Color::WHITE,
            font: Font {
                weight: Weight::Normal,
                ..Font::with_name("Helvetica Neue")
            },
            horizontal_alignment: Horizontal::Left,
            vertical_alignment: Vertical::Top,
            shaping: Shaping::Basic,
        });

        let icon_bounds = children.next().unwrap().bounds();
        if let Some(icon) = self.current_weather.weather_condition().icon(day_time) {
            renderer.draw(icon.handle(), None, icon_bounds);
        }

        renderer.fill_text(Text {
            content: &self.build_conditions(),
            bounds: children.next().unwrap().bounds(),
            size: 12.,
            line_height: LineHeight::default(),
            color: Color::WHITE,
            font: Font {
                weight: Weight::Bold,
                ..Font::with_name("Helvetica Neue")
            },
            horizontal_alignment: Horizontal::Left,
            vertical_alignment: Vertical::Top,
            shaping: Shaping::Basic,
        });
    }
}

impl<'a, M> From<WeatherCard<M>> for Element<'a, M>
where
    M: 'a + Clone,
{
    fn from(modal: WeatherCard<M>) -> Self {
        Element::new(modal)
    }
}
