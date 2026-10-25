//! Rosé Pine Moon theme for iced 0.14.
//!
//! This is the Rust counterpart of
//! `crates/ui-dioxus/assets/styling/rose-pine-moon.css`. Both files use the
//! same role names (`BASE`, `SURFACE`, `OVERLAY`, `IRIS`, …) and the same
//! semantic aliases (`BG`, `BORDER`, `ACCENT`, …) so the two UI experiments
//! can be compared against an identical palette.
//!
//! Palette source: <https://rosepinetheme.com/palette/> (Moon).
//!
//! # Usage
//!
//! ```no_run
//! # use iced::widget::{button, text};
//! # #[derive(Debug, Clone)] enum Message { Click }
//! # fn view(_: &()) -> iced::Element<'_, Message> {
//! button(text("Send"))
//!     .style(ui_iced::theme::style::button_primary)
//!     .into()
//! # }
//! # fn update(_: &mut (), _: Message) {}
//! # fn theme(_: &()) -> iced::Theme { ui_iced::theme::rose_pine_moon() }
//! # fn main() -> iced::Result {
//! iced::application(|| (), update, view)
//!     .title("chat")
//!     .theme(theme)
//!     .run()
//! # }
//! ```

use iced::{
    Color, Theme,
    theme::{
        Palette,
        palette::{Background, Danger, Extended, Pair, Primary, Secondary, Success, Warning},
    },
};

/// The name shown by iced for this theme.
pub const NAME: &str = "Rosé Pine Moon";

/// Builds a [`Color`] from a `0xRRGGBB` literal at compile time.
const fn rgb(hex: u32) -> Color {
    Color::from_rgb(
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    )
}

/// Linear interpolation between two colours.
const fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::from_rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Replaces the alpha channel of a colour.
const fn alpha(color: Color, alpha: f32) -> Color {
    Color::from_rgba(color.r, color.g, color.b, alpha)
}

const fn pair(color: Color, text: Color) -> Pair {
    Pair { color, text }
}

/// Palette and semantic colour tokens.
pub mod palette {
    use super::{Color, alpha, mix, rgb};

    // ---------------------------------------------------------------------
    // Rosé Pine Moon — official palette roles
    // ---------------------------------------------------------------------
    pub const BASE: Color = rgb(0x23_21_36);
    pub const SURFACE: Color = rgb(0x2a_27_3f);
    pub const OVERLAY: Color = rgb(0x39_35_52);
    pub const MUTED: Color = rgb(0x6e_6a_86);
    pub const SUBTLE: Color = rgb(0x90_8c_aa);
    pub const TEXT: Color = rgb(0xe0_de_f4);
    pub const LOVE: Color = rgb(0xeb_6f_92);
    pub const GOLD: Color = rgb(0xf6_c1_77);
    pub const ROSE: Color = rgb(0xea_9a_97);
    pub const PINE: Color = rgb(0x3e_8f_b0);
    pub const FOAM: Color = rgb(0x9c_cf_d8);
    pub const IRIS: Color = rgb(0xc4_a7_e7);
    pub const HIGHLIGHT_LOW: Color = rgb(0x2a_28_3e);
    pub const HIGHLIGHT_MED: Color = rgb(0x44_41_5a);
    pub const HIGHLIGHT_HIGH: Color = rgb(0x56_52_6e);

    // ---------------------------------------------------------------------
    // Semantic aliases — keep these in sync with the CSS custom properties
    // ---------------------------------------------------------------------
    pub const BG: Color = BASE;
    pub const SURFACE_RAISED: Color = OVERLAY;
    pub const SURFACE_HOVER: Color = HIGHLIGHT_LOW;
    pub const SURFACE_ACTIVE: Color = HIGHLIGHT_MED;

    pub const BORDER_SUBTLE: Color = HIGHLIGHT_LOW;
    pub const BORDER: Color = HIGHLIGHT_MED;
    pub const BORDER_STRONG: Color = HIGHLIGHT_HIGH;

    pub const TEXT_MUTED: Color = SUBTLE;
    pub const TEXT_FAINT: Color = MUTED;
    pub const TEXT_DISABLED: Color = MUTED;

    pub const ACCENT: Color = IRIS;
    pub const ACCENT_HOVER: Color = mix(IRIS, BASE, 0.15);
    pub const ACCENT_ACTIVE: Color = mix(IRIS, BASE, 0.30);
    pub const ACCENT_SOFT: Color = alpha(IRIS, 0.14);
    pub const ACCENT_SOFT_HOVER: Color = alpha(IRIS, 0.22);
    pub const ON_ACCENT: Color = BASE;

    pub const SECONDARY: Color = ROSE;
    pub const SUCCESS: Color = PINE;
    pub const SUCCESS_SOFT: Color = alpha(PINE, 0.16);
    pub const WARNING: Color = GOLD;
    pub const WARNING_SOFT: Color = alpha(GOLD, 0.16);
    pub const DANGER: Color = LOVE;
    pub const DANGER_HOVER: Color = mix(LOVE, BASE, 0.15);
    pub const DANGER_ACTIVE: Color = mix(LOVE, BASE, 0.30);
    pub const DANGER_SOFT: Color = alpha(LOVE, 0.16);
    pub const ON_DANGER: Color = BASE;
    pub const INFO: Color = FOAM;
    pub const INFO_SOFT: Color = alpha(FOAM, 0.16);

    pub const FOCUS: Color = IRIS;
    pub const SELECTION: Color = alpha(IRIS, 0.28);
    pub const SHADOW: Color = alpha(rgb(0x0a_08_14), 0.45);
    pub const BACKDROP: Color = alpha(rgb(0x13_11_20), 0.72);

    // Chat-specific tokens.
    pub const CHAT_SIDEBAR_BG: Color = SURFACE;
    pub const CHAT_HEADER_BG: Color = BASE;
    pub const CHAT_COMPOSER_BG: Color = SURFACE;
    pub const CHAT_BUBBLE_INCOMING_BG: Color = SURFACE;
    pub const CHAT_BUBBLE_INCOMING_BORDER: Color = HIGHLIGHT_LOW;
    pub const CHAT_BUBBLE_OUTGOING_BG: Color = OVERLAY;
    pub const CHAT_BUBBLE_OUTGOING_BORDER: Color = HIGHLIGHT_MED;
    pub const CHAT_AVATAR_BG: Color = alpha(IRIS, 0.16);
    pub const CHAT_AVATAR_TEXT: Color = IRIS;
    pub const CHAT_TIMESTAMP: Color = MUTED;
}

/// Spacing scale (4px base), mirroring the CSS `--space-*` tokens.
pub mod spacing {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 20.0;
    pub const XXL: f32 = 24.0;
    pub const XXXL: f32 = 32.0;
}

/// Corner radii, mirroring the CSS `--radius-*` tokens.
pub mod radius {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 6.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 16.0;
    pub const XXL: f32 = 20.0;
    pub const BUBBLE: f32 = 16.0;
    pub const FULL: f32 = 999.0;
}

/// Text sizes in pixels, mirroring the CSS `--text-*` tokens.
pub mod text_size {
    pub const XS: f32 = 12.0;
    pub const SM: f32 = 13.0;
    pub const MD: f32 = 15.0;
    pub const LG: f32 = 18.0;
    pub const XL: f32 = 22.0;
    pub const XXL: f32 = 28.0;
    pub const XXXL: f32 = 36.0;
}

/// The simple iced [`Palette`] used by widgets that only need six colours.
pub const ICED_PALETTE: Palette = Palette {
    background: palette::BASE,
    text: palette::TEXT,
    primary: palette::IRIS,
    success: palette::PINE,
    warning: palette::GOLD,
    danger: palette::LOVE,
};

/// The extended palette backing [`rose_pine_moon`].
const EXTENDED: Extended = Extended {
    background: Background {
        base: pair(palette::BASE, palette::TEXT),
        weakest: pair(mix(palette::BASE, palette::TEXT, 0.03), palette::TEXT),
        weaker: pair(mix(palette::BASE, palette::TEXT, 0.07), palette::TEXT),
        weak: pair(palette::SURFACE, palette::TEXT),
        neutral: pair(mix(palette::SURFACE, palette::OVERLAY, 0.50), palette::TEXT),
        strong: pair(palette::OVERLAY, palette::TEXT),
        stronger: pair(palette::HIGHLIGHT_MED, palette::TEXT),
        strongest: pair(palette::HIGHLIGHT_HIGH, palette::TEXT),
    },
    primary: Primary {
        base: pair(palette::IRIS, palette::BASE),
        weak: pair(palette::ACCENT_ACTIVE, palette::BASE),
        strong: pair(palette::ACCENT_HOVER, palette::BASE),
    },
    secondary: Secondary {
        base: pair(palette::OVERLAY, palette::TEXT),
        weak: pair(palette::HIGHLIGHT_MED, palette::TEXT),
        strong: pair(palette::HIGHLIGHT_HIGH, palette::TEXT),
    },
    success: Success {
        base: pair(palette::PINE, palette::BASE),
        weak: pair(mix(palette::PINE, palette::BASE, 0.30), palette::BASE),
        strong: pair(mix(palette::PINE, palette::BASE, 0.15), palette::BASE),
    },
    warning: Warning {
        base: pair(palette::GOLD, palette::BASE),
        weak: pair(mix(palette::GOLD, palette::BASE, 0.30), palette::BASE),
        strong: pair(mix(palette::GOLD, palette::BASE, 0.15), palette::BASE),
    },
    danger: Danger {
        base: pair(palette::LOVE, palette::BASE),
        weak: pair(palette::DANGER_ACTIVE, palette::BASE),
        strong: pair(palette::DANGER_HOVER, palette::BASE),
    },
    is_dark: true,
};

/// Creates the Rosé Pine Moon [`Theme`].
///
/// ```no_run
/// # fn update(_: &mut (), _: ()) {}
/// # fn view(_: &()) -> iced::Element<'_, ()> {
/// #     iced::widget::text("chat").into()
/// # }
/// # fn theme(_: &()) -> iced::Theme { ui_iced::theme::rose_pine_moon() }
/// iced::application(|| (), update, view)
///     .title("chat")
///     .theme(theme);
/// ```
pub fn rose_pine_moon() -> Theme {
    Theme::custom_with_fn(NAME.to_owned(), ICED_PALETTE, |_| EXTENDED)
}

/// Widget styles built from the Rosé Pine Moon tokens.
///
/// The functions in this module deliberately ignore the incoming [`Theme`] and
/// return fixed Moon colours. That keeps the iced experiment byte-for-byte
/// consistent with the CSS theme; switching palettes means swapping the
/// functions (or adding a sibling token set), not having widgets drift.
pub mod style {
    use super::{palette, radius};
    use iced::border::Radius;
    use iced::widget::{
        button, container, rule, scrollable as scrollable_widget, text_input as text_input_widget,
    };
    use iced::{Border, Shadow, Theme, Vector};

    fn border(color: iced::Color, width: f32) -> Border {
        Border {
            color,
            width,
            radius: radius::MD.into(),
        }
    }

    fn border_with_radius(color: iced::Color, width: f32, radius: f32) -> Border {
        Border {
            color,
            width,
            radius: radius.into(),
        }
    }

    fn no_border() -> Border {
        border(iced::Color::TRANSPARENT, 0.0)
    }

    fn button_style(background: iced::Color, text_color: iced::Color) -> button::Style {
        button::Style {
            background: Some(background.into()),
            text_color,
            border: no_border(),
            shadow: Shadow::default(),
            snap: true,
        }
    }

    fn button_style_bordered(
        background: iced::Color,
        text_color: iced::Color,
        border_color: iced::Color,
    ) -> button::Style {
        button::Style {
            background: Some(background.into()),
            text_color,
            border: border(border_color, 1.0),
            shadow: Shadow::default(),
            snap: true,
        }
    }

    // -----------------------------------------------------------------
    // Containers
    // -----------------------------------------------------------------

    /// Root application surface.
    pub fn app(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::TEXT),
            background: Some(palette::BG.into()),
            ..container::Style::default()
        }
    }

    /// Sidebar / navigation panel.
    pub fn sidebar(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::TEXT),
            background: Some(palette::CHAT_SIDEBAR_BG.into()),
            ..container::Style::default()
        }
    }

    /// Elevated card with a soft shadow.
    pub fn card(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::TEXT),
            background: Some(palette::SURFACE.into()),
            border: border_with_radius(palette::BORDER_SUBTLE, 1.0, radius::LG),
            shadow: Shadow {
                color: palette::SHADOW,
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            snap: true,
        }
    }

    /// Chat header.
    pub fn chat_header(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::TEXT),
            background: Some(palette::CHAT_HEADER_BG.into()),
            ..container::Style::default()
        }
    }

    /// Message composer strip.
    pub fn chat_composer(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::TEXT),
            background: Some(palette::CHAT_COMPOSER_BG.into()),
            ..container::Style::default()
        }
    }

    /// Incoming message bubble (tail on the bottom-left).
    pub fn bubble_incoming(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::TEXT),
            background: Some(palette::CHAT_BUBBLE_INCOMING_BG.into()),
            border: Border {
                color: palette::CHAT_BUBBLE_INCOMING_BORDER,
                width: 1.0,
                radius: Radius {
                    top_left: radius::BUBBLE,
                    top_right: radius::BUBBLE,
                    bottom_right: radius::BUBBLE,
                    bottom_left: radius::XS,
                },
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }

    /// Outgoing message bubble (tail on the bottom-right).
    pub fn bubble_outgoing(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::TEXT),
            background: Some(palette::CHAT_BUBBLE_OUTGOING_BG.into()),
            border: Border {
                color: palette::CHAT_BUBBLE_OUTGOING_BORDER,
                width: 1.0,
                radius: Radius {
                    top_left: radius::BUBBLE,
                    top_right: radius::BUBBLE,
                    bottom_right: radius::XS,
                    bottom_left: radius::BUBBLE,
                },
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }

    /// Circular avatar chip.
    pub fn avatar(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(palette::CHAT_AVATAR_TEXT),
            background: Some(palette::CHAT_AVATAR_BG.into()),
            border: border_with_radius(iced::Color::TRANSPARENT, 0.0, radius::FULL),
            shadow: Shadow::default(),
            snap: true,
        }
    }

    /// Modal backdrop.
    pub fn backdrop(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: None,
            background: Some(palette::BACKDROP.into()),
            ..container::Style::default()
        }
    }

    // -----------------------------------------------------------------
    // Buttons
    // -----------------------------------------------------------------

    /// Filled accent button.
    pub fn button_primary(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Active => button_style(palette::ACCENT, palette::ON_ACCENT),
            button::Status::Hovered => button_style(palette::ACCENT_HOVER, palette::ON_ACCENT),
            button::Status::Pressed => button_style(palette::ACCENT_ACTIVE, palette::ON_ACCENT),
            button::Status::Disabled => {
                button_style(palette::SURFACE_ACTIVE, palette::TEXT_DISABLED)
            }
        }
    }

    /// Subtle raised button.
    pub fn button_secondary(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Active => {
                button_style_bordered(palette::SURFACE_RAISED, palette::TEXT, palette::BORDER)
            }
            button::Status::Hovered => button_style_bordered(
                palette::SURFACE_ACTIVE,
                palette::TEXT,
                palette::BORDER_STRONG,
            ),
            button::Status::Pressed => button_style_bordered(
                palette::HIGHLIGHT_HIGH,
                palette::TEXT,
                palette::BORDER_STRONG,
            ),
            button::Status::Disabled => button_style_bordered(
                palette::SURFACE,
                palette::TEXT_DISABLED,
                palette::BORDER_SUBTLE,
            ),
        }
    }

    /// Transparent button used for icons and quiet actions.
    pub fn button_ghost(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Active => button_style(iced::Color::TRANSPARENT, palette::TEXT_MUTED),
            button::Status::Hovered => button_style(palette::SURFACE_HOVER, palette::TEXT),
            button::Status::Pressed => button_style(palette::SURFACE_ACTIVE, palette::TEXT),
            button::Status::Disabled => {
                button_style(iced::Color::TRANSPARENT, palette::TEXT_DISABLED)
            }
        }
    }

    /// Destructive action button.
    pub fn button_danger(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Active => button_style(palette::DANGER, palette::ON_DANGER),
            button::Status::Hovered => button_style(palette::DANGER_HOVER, palette::ON_DANGER),
            button::Status::Pressed => button_style(palette::DANGER_ACTIVE, palette::ON_DANGER),
            button::Status::Disabled => {
                button_style(palette::SURFACE_ACTIVE, palette::TEXT_DISABLED)
            }
        }
    }

    // -----------------------------------------------------------------
    // Inputs & misc
    // -----------------------------------------------------------------

    /// Text input used by the chat composer.
    pub fn text_input(
        _theme: &Theme,
        status: text_input_widget::Status,
    ) -> text_input_widget::Style {
        let (background, border_color, value, placeholder, icon) = match status {
            text_input_widget::Status::Active => (
                palette::BG,
                palette::BORDER,
                palette::TEXT,
                palette::MUTED,
                palette::SUBTLE,
            ),
            text_input_widget::Status::Hovered => (
                palette::BG,
                palette::BORDER_STRONG,
                palette::TEXT,
                palette::MUTED,
                palette::SUBTLE,
            ),
            text_input_widget::Status::Focused { .. } => (
                palette::BG,
                palette::FOCUS,
                palette::TEXT,
                palette::MUTED,
                palette::SUBTLE,
            ),
            text_input_widget::Status::Disabled => (
                palette::SURFACE,
                palette::BORDER_SUBTLE,
                palette::MUTED,
                palette::MUTED,
                palette::MUTED,
            ),
        };

        text_input_widget::Style {
            background: background.into(),
            border: border_with_radius(border_color, 1.0, radius::MD),
            icon,
            placeholder,
            value,
            selection: palette::SELECTION,
        }
    }

    /// Slim, low-contrast scrollbar for the message list.
    pub fn scrollable(
        _theme: &Theme,
        status: scrollable_widget::Status,
    ) -> scrollable_widget::Style {
        let thumb = match status {
            scrollable_widget::Status::Active { .. } => palette::HIGHLIGHT_MED,
            _ => palette::HIGHLIGHT_HIGH,
        };

        let rail = scrollable_widget::Rail {
            background: Some(iced::Color::TRANSPARENT.into()),
            border: Border::default(),
            scroller: scrollable_widget::Scroller {
                background: thumb.into(),
                border: border_with_radius(iced::Color::TRANSPARENT, 0.0, radius::FULL),
            },
        };

        scrollable_widget::Style {
            container: container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            auto_scroll: scrollable_widget::AutoScroll {
                background: iced::Color::TRANSPARENT.into(),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: palette::SUBTLE,
            },
        }
    }

    /// One-pixel divider that follows the low-contrast border token.
    pub fn divider(_theme: &Theme) -> rule::Style {
        rule::Style {
            color: palette::BORDER_SUBTLE,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        }
    }
}

#[cfg(test)]
mod tests {
    /// Both UI experiments must expose the exact same Rosé Pine Moon values.
    /// If this test fails, update `rose-pine-moon.css` or the constants above.
    #[test]
    fn palette_matches_the_dioxus_css() {
        const CSS: &str = include_str!("../../ui-dioxus/assets/styling/rose-pine-moon.css");

        let roles = [
            ("base", "232136"),
            ("surface", "2a273f"),
            ("overlay", "393552"),
            ("muted", "6e6a86"),
            ("subtle", "908caa"),
            ("text", "e0def4"),
            ("love", "eb6f92"),
            ("gold", "f6c177"),
            ("rose", "ea9a97"),
            ("pine", "3e8fb0"),
            ("foam", "9ccfd8"),
            ("iris", "c4a7e7"),
            ("highlight-low", "2a283e"),
            ("highlight-med", "44415a"),
            ("highlight-high", "56526e"),
        ];

        for (role, hex) in roles {
            let declaration = format!("--rp-{role}: #{hex};");
            assert!(
                CSS.contains(&declaration),
                "the CSS theme is missing `{declaration}`"
            );
        }
    }
}
