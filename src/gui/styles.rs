use iced::{Background, Color, Theme, widget};

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
    pub const USB_YELLOW: Color = Color::from_rgb(0.9, 0.8, 0.0);    // Yellow for status packets
    pub const USB_CYAN: Color = Color::from_rgb(0.0, 0.8, 0.8);      // Cyan for isochronous
    pub const CODE_GREEN: Color = Color::from_rgb(0.0, 0.7, 0.2);    // Green for code
    #[allow(dead_code)]
    pub const PCB_GREEN: Color = Color::from_rgb(0.0, 0.5, 0.3);     // PCB color
    #[allow(dead_code)]
    pub const SIGNAL_BLUE: Color = Color::from_rgb(0.1, 0.6, 0.9);   // Signal trace blue
    #[allow(dead_code)]
    pub const COPPER: Color = Color::from_rgb(0.8, 0.5, 0.2);        // Copper traces
    
    // Cyberpunk/Hacker-themed dark mode colors
    pub mod dark {
        use iced::Color;
        
        // Main dark theme colors
        pub const PRIMARY: Color = Color::from_rgb(0.0, 0.8, 0.6);       // Cyan - Neon primary
        pub const PRIMARY_LIGHT: Color = Color::from_rgb(0.4, 0.9, 0.8); // Light cyan for highlights
        pub const PRIMARY_DARK: Color = Color::from_rgb(0.0, 0.5, 0.4);  // Dark cyan for active states
        
        // Secondary dark theme colors
        pub const SECONDARY: Color = Color::from_rgb(0.8, 0.2, 0.8);     // Magenta - Secondary neon
        #[allow(dead_code)]
        pub const SECONDARY_LIGHT: Color = Color::from_rgb(0.9, 0.5, 0.9); // Light magenta highlights
        
        // Accent colors
        #[allow(dead_code)]
        pub const ACCENT: Color = Color::from_rgb(1.0, 0.8, 0.0);        // Yellow - accent for warnings
        pub const ERROR: Color = Color::from_rgb(1.0, 0.2, 0.3);         // Red - for errors
        
        // Neutral colors
        pub const BACKGROUND: Color = Color::from_rgb(0.08, 0.08, 0.1);  // Very dark blue/black
        pub const SURFACE: Color = Color::from_rgb(0.13, 0.14, 0.18);    // Dark gray for surfaces
        pub const TEXT: Color = Color::from_rgb(0.9, 0.9, 0.95);         // Almost white
        pub const TEXT_SECONDARY: Color = Color::from_rgb(0.7, 0.7, 0.75); // Lighter text
        
        // Status colors
        #[allow(dead_code)]
        pub const SUCCESS: Color = Color::from_rgb(0.2, 0.9, 0.4);       // Green - success indicators
        #[allow(dead_code)]
        pub const WARNING: Color = ACCENT;                               // Yellow - warning indicators
        #[allow(dead_code)]
        pub const INFO: Color = PRIMARY;                                 // Cyan - information
        
        // Electronics-themed colors
        pub const USB_GREEN: Color = Color::from_rgb(0.2, 1.0, 0.6);     // Brighter USB logo green
        pub const USB_YELLOW: Color = Color::from_rgb(1.0, 0.9, 0.2);    // Yellow for status packets
        pub const USB_CYAN: Color = Color::from_rgb(0.2, 0.9, 1.0);      // Cyan for isochronous
        #[allow(dead_code)]
        pub const PCB_GREEN: Color = Color::from_rgb(0.1, 0.6, 0.4);     // PCB color
        #[allow(dead_code)]
        pub const SIGNAL_BLUE: Color = Color::from_rgb(0.3, 0.7, 1.0);   // Signal trace blue
        #[allow(dead_code)]
        pub const COPPER: Color = Color::from_rgb(0.9, 0.6, 0.3);        // Copper traces
        
        // Special dark mode colors
        #[allow(dead_code)]
        pub const GRID_LINES: Color = Color::from_rgba(0.3, 0.9, 0.8, 0.15); // Cyan grid lines
        #[allow(dead_code)]
        pub const GLOW: Color = Color::from_rgba(0.0, 0.8, 0.8, 0.3);    // Cyan glow effect
        pub const CODE_GREEN: Color = Color::from_rgb(0.0, 0.8, 0.3);    // Matrix-like green text
    }
}

// Consistent border radius throughout the app
pub const BORDER_RADIUS: f32 = 6.0;

/// Style for header containers
pub struct HeaderContainer;

impl iced::widget::container::StyleSheet for HeaderContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(color::PRIMARY_LIGHT)),
            border_radius: (BORDER_RADIUS * 0.7).into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

/// Style for child containers in tree view
pub struct ChildContainer;

impl iced::widget::container::StyleSheet for ChildContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::TEXT),
            background: Some(Background::Color(Color { r: 0.95, g: 0.95, b: 0.95, a: 1.0 })),
            border_radius: (BORDER_RADIUS * 0.5).into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

/// Style for child containers in tree view (dark mode)
pub struct DarkModeChildContainer;

impl iced::widget::container::StyleSheet for DarkModeChildContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::dark::TEXT),
            background: Some(Background::Color(Color { r: 0.15, g: 0.16, b: 0.20, a: 1.0 })),
            border_radius: (BORDER_RADIUS * 0.5).into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

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

pub struct DarkModeHintCategoryContainer;

impl iced::widget::container::StyleSheet for DarkModeHintCategoryContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(color::dark::PRIMARY)),
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

// Dark mode containers
pub struct DarkModeContainer;

impl iced::widget::container::StyleSheet for DarkModeContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::dark::TEXT),
            background: Some(Background::Color(color::dark::SURFACE)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
        }
    }
}

pub struct DarkModeSelectedContainer;

impl iced::widget::container::StyleSheet for DarkModeSelectedContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(color::dark::PRIMARY)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

pub struct DarkModeTreeNodeContainer;

impl iced::widget::container::StyleSheet for DarkModeTreeNodeContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::dark::TEXT),
            background: Some(Background::Color(Color::from_rgba(0.2, 0.9, 0.7, 0.1))),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

/// Dark mode style for header containers
pub struct DarkModeHeaderContainer;

impl iced::widget::container::StyleSheet for DarkModeHeaderContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(color::dark::PRIMARY_DARK)),
            border_radius: (BORDER_RADIUS * 0.7).into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

// Removed duplicate DarkModeChildContainer definition

// Dark mode buttons
pub struct DarkModePrimaryButton;

impl iced::widget::button::StyleSheet for DarkModePrimaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 1.0),
            background: Some(Background::Color(color::dark::PRIMARY)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(color::dark::PRIMARY_LIGHT)),
            shadow_offset: iced::Vector::new(0.0, 2.0),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(color::dark::PRIMARY_DARK)),
            shadow_offset: iced::Vector::new(0.0, 0.0),
            ..active
        }
    }
}

// Dark mode text input style
pub struct DarkModeTextInput;

impl iced::widget::text_input::StyleSheet for DarkModeTextInput {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: Background::Color(color::dark::SURFACE),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: color::dark::PRIMARY,
            icon_color: color::dark::TEXT,
        }
    }

    fn focused(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        let active = self.active(style);
        iced::widget::text_input::Appearance {
            border_color: color::dark::PRIMARY_LIGHT,
            ..active
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> iced::Color {
        color::dark::TEXT_SECONDARY
    }

    fn value_color(&self, _style: &Self::Style) -> iced::Color {
        color::dark::TEXT
    }

    fn selection_color(&self, _style: &Self::Style) -> iced::Color {
        color::dark::PRIMARY_DARK
    }

    fn disabled(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: Background::Color(Color::from_rgb(0.15, 0.15, 0.2)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.2, 0.2, 0.25),
            icon_color: Color::from_rgb(0.4, 0.4, 0.5),
        }
    }

    fn disabled_color(&self, _style: &Self::Style) -> iced::Color {
        Color::from_rgb(0.4, 0.4, 0.5)
    }
}

// Dark mode scrollable style
pub struct DarkModeScrollable;

impl widget::scrollable::StyleSheet for DarkModeScrollable {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> widget::scrollable::Scrollbar {
        widget::scrollable::Scrollbar {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.0))),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: widget::scrollable::Scroller {
                color: color::dark::TEXT_SECONDARY,
                border_radius: BORDER_RADIUS.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }
        }
    }

    fn hovered(
        &self, 
        style: &Self::Style,
        is_mouse_over_scrollbar: bool,
    ) -> widget::scrollable::Scrollbar {
        let active = self.active(style);
        
        if is_mouse_over_scrollbar {
            widget::scrollable::Scrollbar {
                scroller: widget::scrollable::Scroller {
                    color: color::dark::PRIMARY,
                    ..active.scroller
                },
                ..active
            }
        } else {
            active
        }
    }

    fn dragging(&self, style: &Self::Style) -> widget::scrollable::Scrollbar {
        let active = self.active(style);
        
        widget::scrollable::Scrollbar {
            scroller: widget::scrollable::Scroller {
                color: color::dark::PRIMARY_DARK,
                ..active.scroller
            },
            ..active
        }
    }
}

// Dark mode application container
pub struct DarkModeApplicationContainer;

impl iced::widget::container::StyleSheet for DarkModeApplicationContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(color::dark::TEXT),
            background: Some(Background::Color(color::dark::BACKGROUND)),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }
}

pub struct DarkModeSecondaryButton;

impl iced::widget::button::StyleSheet for DarkModeSecondaryButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 1.0),
            background: Some(Background::Color(color::dark::SECONDARY)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.9, 0.3, 0.9))),
            shadow_offset: iced::Vector::new(0.0, 2.0),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.7, 0.1, 0.7))),
            shadow_offset: iced::Vector::new(0.0, 0.0),
            ..active
        }
    }
}

pub struct DarkModeDestructiveButton;

impl iced::widget::button::StyleSheet for DarkModeDestructiveButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 1.0),
            background: Some(Background::Color(color::dark::ERROR)),
            border_radius: BORDER_RADIUS.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.3, 0.4))),
            shadow_offset: iced::Vector::new(0.0, 2.0),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.8, 0.1, 0.2))),
            shadow_offset: iced::Vector::new(0.0, 0.0),
            ..active
        }
    }
}

// TreeNode helper styles
pub struct TreeNodeButton;

impl iced::widget::button::StyleSheet for TreeNodeButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 0.0),
            background: Some(Background::Color(Color::TRANSPARENT)),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: color::TEXT,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.05))),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.1))),
            ..active
        }
    }
}

pub struct DarkModeTreeNodeButton;

impl iced::widget::button::StyleSheet for DarkModeTreeNodeButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            shadow_offset: iced::Vector::new(0.0, 0.0),
            background: Some(Background::Color(Color::TRANSPARENT)),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: color::dark::TEXT,
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.8, 0.8, 0.15))),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let active = self.active(style);
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.8, 0.8, 0.25))),
            ..active
        }
    }
}
