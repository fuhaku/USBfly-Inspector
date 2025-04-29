use iced::{Background, Color, Theme};

pub struct SelectedContainer;

impl iced::widget::container::StyleSheet for SelectedContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgb(0.2, 0.4, 0.8))),
            border_radius: 4.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

pub struct HintContainer;

impl iced::widget::container::StyleSheet for HintContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::from_rgb(0.0, 0.5, 0.0)),
            background: Some(Background::Color(Color::from_rgb(0.9, 1.0, 0.9))),
            border_radius: 4.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.8, 0.9, 0.8),
        }
    }
}
