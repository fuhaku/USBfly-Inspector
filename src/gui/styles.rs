use iced::{Background, Color, Theme};

// Define a consistent color palette for the entire application
pub mod color {
    use iced::Color;
    
    // Main brand colors
    pub const PRIMARY: Color = Color::from_rgb(0.0, 0.4, 0.8);       // Blue - Brand primary
    pub const PRIMARY_LIGHT: Color = Color::from_rgb(0.4, 0.6, 0.9); // Light blue for highlights
    pub const PRIMARY_DARK: Color = Color::from_rgb(0.0, 0.3, 0.6);  // Dark blue for active states
    
    // Secondary brand colors
    pub const SECONDARY: Color = Color::from_rgb(0.0, 0.7, 0.4);     // Teal - Brand secondary
    pub const SECONDARY_LIGHT: Color = Color::from_rgb(0.7, 0.9, 0.8); // Light teal for highlights
    
    // Accent colors
    pub const ACCENT: Color = Color::from_rgb(1.0, 0.6, 0.0);        // Orange - accent for warnings
    pub const ERROR: Color = Color::from_rgb(0.9, 0.2, 0.2);         // Red - for errors
    
    // Neutral colors
    pub const BACKGROUND: Color = Color::from_rgb(0.95, 0.97, 1.0);  // Off-white with blue tint
    pub const SURFACE: Color = Color::from_rgb(1.0, 1.0, 1.0);       // Pure white for card surfaces
    pub const TEXT: Color = Color::from_rgb(0.1, 0.1, 0.15);         // Almost black
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.4, 0.4, 0.5); // Lighter text
    
    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.0, 0.7, 0.3);       // Green - success indicators
    pub const WARNING: Color = ACCENT;                               // Orange - warning indicators
    pub const INFO: Color = PRIMARY;                                 // Blue - information
    
    // Electronics-themed colors
    pub const USB_GREEN: Color = Color::from_rgb(0.0, 0.8, 0.4);     // USB logo green
    #[allow(dead_code)]
    pub const PCB_GREEN: Color = Color::from_rgb(0.0, 0.5, 0.3);     // PCB color
    #[allow(dead_code)]
    pub const SIGNAL_BLUE: Color = Color::from_rgb(0.1, 0.6, 0.9);   // Signal trace blue
    #[allow(dead_code)]
    pub const COPPER: Color = Color::from_rgb(0.8, 0.5, 0.2);        // Copper traces
}

// Consistent border radius throughout the app
pub const BORDER_RADIUS: f32 = 6.0;

/// Style for selected containers/items (active selection)
pub struct SelectedContainer;

impl iced::widget::container::StyleSheet for SelectedContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(color::PRIMARY)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

/// Style for hint containers/tooltips
pub struct HintContainer;

impl iced::widget::container::StyleSheet for HintContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::SUCCESS),
            background: Some(Background::Color(color::SECONDARY_LIGHT)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: color::SECONDARY,
        }
    }
}

/// Style for hint category headers
pub struct HintCategoryContainer;

impl iced::widget::container::StyleSheet for HintCategoryContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(color::USB_GREEN)),
            border_radius: (BORDER_RADIUS * 0.7).into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

/// Style for title text
pub struct TitleContainer;

impl iced::widget::container::StyleSheet for TitleContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::PRIMARY_DARK),
            background: Some(Background::Color(color::BACKGROUND)),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

/// Style for card-like containers
pub struct CardContainer;

impl iced::widget::container::StyleSheet for CardContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::TEXT),
            background: Some(Background::Color(color::SURFACE)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
        }
    }
}

/// Style for information/status messages
pub struct InfoContainer;

impl iced::widget::container::StyleSheet for InfoContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::INFO),
            background: Some(Background::Color(Color::from_rgba(0.9, 0.95, 1.0, 0.5))),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: color::INFO,
        }
    }
}

/// Style for warning messages
pub struct WarningContainer;

impl iced::widget::container::StyleSheet for WarningContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::WARNING),
            background: Some(Background::Color(Color::from_rgba(1.0, 0.95, 0.9, 0.5))),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: color::WARNING,
        }
    }
}

/// Style for error messages
pub struct ErrorContainer;

impl iced::widget::container::StyleSheet for ErrorContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::ERROR),
            background: Some(Background::Color(Color::from_rgba(1.0, 0.9, 0.9, 0.5))),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: color::ERROR,
        }
    }
}

/// Style for buttons that perform primary actions
pub struct PrimaryButton;

impl iced::widget::button::StyleSheet for PrimaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 1.0),
            background: Some(Background::Color(color::PRIMARY)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(color::PRIMARY_LIGHT)),
            shadow_offset: iced::Vector::new(0.0, 2.0),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(color::PRIMARY_DARK)),
            shadow_offset: iced::Vector::new(0.0, 0.0),
            ..active
        }
    }
}

/// Style for secondary action buttons
pub struct SecondaryButton;

impl iced::widget::button::StyleSheet for SecondaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 1.0),
            background: Some(Background::Color(color::SECONDARY)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.1, 0.8, 0.5))),
            shadow_offset: iced::Vector::new(0.0, 2.0),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.0, 0.6, 0.3))),
            shadow_offset: iced::Vector::new(0.0, 0.0),
            ..active
        }
    }
}

/// Ghost button style with just an outline and no background
pub struct GhostButton;

impl iced::widget::button::StyleSheet for GhostButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 0.0),
            background: Some(Background::Color(Color::TRANSPARENT)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: color::PRIMARY,
            text_color: color::PRIMARY,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.4, 0.8, 0.1))),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.4, 0.8, 0.2))),
            ..active
        }
    }
}

// HintCategoryContainer is defined above
