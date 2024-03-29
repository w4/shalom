use iced::{
    advanced::{
        image::Data,
        layout::{Limits, Node},
        overlay,
        renderer::Style,
        widget::{Operation, Tree},
        Clipboard, Layout, Shell, Widget,
    },
    event::Status,
    mouse::{Cursor, Interaction},
    widget::{image, image::FilterMethod},
    Alignment, ContentFit, Element, Event, Length, Point, Rectangle, Size, Vector,
};

pub fn image_background<'a, M: 'a, R>(
    handle: impl Into<image::Handle>,
    el: Element<'a, M, R>,
) -> ImageBackground<'a, M, R> {
    let image_handle = handle.into();

    ImageBackground {
        image_handle: image_handle.clone(),
        el,
        width: Length::FillPortion(1),
        height: Length::Fixed(128.0),
    }
}

pub struct ImageBackground<'a, M, R> {
    image_handle: image::Handle,
    el: Element<'a, M, R>,
    width: Length,
    height: Length,
}

impl<'a, M, R> ImageBackground<'a, M, R> {
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }
}

impl<
        'a,
        M: Clone,
        R: iced::advanced::Renderer
            + iced::advanced::image::Renderer<Handle = iced::advanced::image::Handle>,
    > Widget<M, R> for ImageBackground<'a, M, R>
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&self, _state: &mut Tree, _renderer: &R, limits: &Limits) -> Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.resolve(self.width, self.height, Size::ZERO);

        Node::new(size)
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut R,
        _theme: &<R as iced::advanced::Renderer>::Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        // The raw w/h of the underlying image, renderer.dimensions is _really_
        // slow so enforce the use of preparsed images from `theme`.
        #[allow(clippy::cast_precision_loss)]
        let image_size = match self.image_handle.data() {
            Data::Rgba { width, height, .. } => Size::new(*width as f32, *height as f32),
            Data::Path(_) | Data::Bytes(_) => panic!("only parsed images are supported"),
        };

        let adjusted_fit = ContentFit::Cover.fit(image_size, bounds.size());

        renderer.with_layer(bounds, |renderer| {
            let offset = Vector::new(
                (bounds.width - adjusted_fit.width).min(0.0) / 1.5,
                (bounds.height - adjusted_fit.height).min(0.0) / 1.5,
            );

            let drawing_bounds = Rectangle {
                width: adjusted_fit.width,
                height: adjusted_fit.height,
                ..bounds
            };

            renderer.draw(
                self.image_handle.clone(),
                FilterMethod::Linear,
                drawing_bounds + offset,
            );
        });
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.el)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.el]);
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &R,
    ) -> Option<iced::advanced::overlay::Element<'b, M, R>> {
        Some(overlay::Element::new(
            layout.position(),
            Box::new(Overlay {
                el: &mut self.el,
                tree: &mut state.children[0],
                size: layout.bounds().size(),
            }),
        ))
    }
}

struct Overlay<'a, 'b, M, R> {
    el: &'b mut Element<'a, M, R>,
    tree: &'b mut Tree,
    size: Size,
}

impl<'a, 'b, M: Clone, R: iced::advanced::Renderer> overlay::Overlay<M, R>
    for Overlay<'a, 'b, M, R>
{
    fn layout(
        &mut self,
        renderer: &R,
        _bounds: Size,
        position: Point,
        _translation: Vector,
    ) -> Node {
        let limits = Limits::new(Size::ZERO, self.size)
            .width(Length::Fill)
            .height(Length::Fill);

        let child = self
            .el
            .as_widget()
            .layout(self.tree, renderer, &limits)
            .align(Alignment::Start, Alignment::Start, limits.max());

        Node::with_children(self.size, vec![child]).move_to(position)
    }

    fn draw(
        &self,
        renderer: &mut R,
        theme: &<R as iced::advanced::Renderer>::Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        self.el.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
            &layout.bounds(),
        );
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
    ) -> Status {
        self.el.as_widget_mut().on_event(
            self.tree,
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &R,
    ) -> Interaction {
        self.el.as_widget().mouse_interaction(
            self.tree,
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &R, operation: &mut dyn Operation<M>) {
        self.el.as_widget().operate(
            self.tree,
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }
}

impl<'a, M, R> From<ImageBackground<'a, M, R>> for Element<'a, M, R>
where
    M: 'a + Clone,
    R: iced::advanced::Renderer
        + iced::advanced::image::Renderer<Handle = iced::advanced::image::Handle>
        + 'a,
{
    fn from(modal: ImageBackground<'a, M, R>) -> Self {
        Element::new(modal)
    }
}
