//! A tiny Rosé Pine Moon preview built with iced 0.13.
//!
//! It exercises every style exposed by [`ui_iced::theme::style`]: app shell,
//! sidebar, chat header, message bubbles, composer, text input, buttons and
//! the scrollbar. Use it as the iced reference when comparing UI toolkits.

use iced::alignment::Horizontal;
use iced::widget::{Space, button, column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use ui_iced::theme::{palette, radius, spacing, style, text_size};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("Rosé Pine Moon · iced")
        .theme(theme)
        .window_size((960.0, 640.0))
        .run()
}

fn theme(_state: &App) -> iced::Theme {
    ui_iced::theme::rose_pine_moon()
}

struct App {
    draft: String,
    messages: Vec<ChatLine>,
}

struct ChatLine {
    body: String,
    outgoing: bool,
    time: &'static str,
}

impl Default for App {
    fn default() -> Self {
        Self {
            draft: String::new(),
            messages: vec![
                ChatLine {
                    body: "Rosé Pine Moon is wired into iced through a custom Theme and a set of \
                           style functions."
                        .to_owned(),
                    outgoing: false,
                    time: "09:41",
                },
                ChatLine {
                    body: "Same tokens as the Dioxus CSS — only the toolkit changes.".to_owned(),
                    outgoing: true,
                    time: "09:42",
                },
            ],
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    DraftChanged(String),
    Send,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::DraftChanged(value) => self.draft = value,
            Message::Send => {
                let body = self.draft.trim().to_owned();
                if !body.is_empty() {
                    self.messages.push(ChatLine {
                        body,
                        outgoing: true,
                        time: "now",
                    });
                    self.draft.clear();
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = container(
            column![
                text("Rosé Pine Moon")
                    .size(text_size::LG)
                    .color(palette::TEXT),
                text("iced · theme preview")
                    .size(text_size::XS)
                    .color(palette::TEXT_FAINT),
                Space::new().height(spacing::XL),
                nav_item("Chat", true),
                nav_item("Components", false),
                nav_item("Settings", false),
                Space::new().height(Length::Fill),
                text("PALETTE").size(text_size::XS).color(palette::MUTED),
                swatches(),
            ]
            .spacing(spacing::SM)
            .padding(spacing::LG),
        )
        .width(Length::Fixed(240.0))
        .height(Length::Fill)
        .style(style::sidebar);

        let header = container(
            row![
                column![
                    text("# project-rose-pine")
                        .size(text_size::MD)
                        .color(palette::TEXT),
                    text("Theme comparison · 3 members")
                        .size(text_size::XS)
                        .color(palette::TEXT_FAINT),
                ]
                .spacing(2.0),
                Space::new().width(Length::Fill),
                text("MOON").size(text_size::XS).color(palette::ACCENT),
            ]
            .align_y(Alignment::Center),
        )
        .padding([spacing::MD, spacing::XL])
        .style(style::chat_header);

        let mut message_list = column![].spacing(spacing::MD).padding(spacing::XXL);
        for line in &self.messages {
            message_list = message_list.push(message_row(line));
        }

        let message_list = scrollable(message_list)
            .height(Length::Fill)
            .style(style::scrollable);

        let composer = container(
            row![
                text_input("Message #project-rose-pine", &self.draft)
                    .on_input(Message::DraftChanged)
                    .on_submit(Message::Send)
                    .padding(spacing::SM)
                    .style(style::text_input),
                button(text("Send").size(text_size::SM))
                    .padding([spacing::SM, spacing::LG])
                    .on_press(Message::Send)
                    .style(style::button_primary),
            ]
            .spacing(spacing::SM)
            .align_y(Alignment::Center),
        )
        .padding(spacing::LG)
        .style(style::chat_composer);

        let main = column![
            header,
            rule::horizontal(1).style(style::divider),
            message_list,
            rule::horizontal(1).style(style::divider),
            composer,
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(row![sidebar, rule::vertical(1).style(style::divider), main,])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::app)
            .into()
    }
}

fn nav_item(label: &'static str, active: bool) -> Element<'static, Message> {
    let color = if active {
        palette::ACCENT
    } else {
        palette::TEXT_MUTED
    };

    container(text(label).size(text_size::SM).color(color))
        .padding([spacing::SM, spacing::MD])
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: active.then_some(palette::ACCENT_SOFT.into()),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: radius::SM.into(),
            },
            ..Default::default()
        })
        .into()
}

fn message_row(line: &ChatLine) -> Element<'_, Message> {
    let bubble = container(text(&line.body).size(text_size::MD).color(palette::TEXT))
        .padding([spacing::SM, spacing::MD])
        .max_width(520.0)
        .style(if line.outgoing {
            style::bubble_outgoing
        } else {
            style::bubble_incoming
        });

    let meta = text(line.time)
        .size(text_size::XS)
        .color(palette::CHAT_TIMESTAMP);

    let mut body = column![bubble, meta].spacing(spacing::XS);
    if line.outgoing {
        body = body.align_x(Horizontal::Right);
    }

    let avatar = container(
        text(if line.outgoing { "ME" } else { "RP" })
            .size(text_size::XS)
            .color(palette::CHAT_AVATAR_TEXT),
    )
    .center(30.0)
    .style(style::avatar);

    let content = if line.outgoing {
        row![body, avatar]
    } else {
        row![avatar, body]
    };

    container(content)
        .width(Length::Fill)
        .align_x(if line.outgoing {
            Horizontal::Right
        } else {
            Horizontal::Left
        })
        .into()
}

fn swatches() -> Element<'static, Message> {
    row![
        swatch(palette::LOVE),
        swatch(palette::GOLD),
        swatch(palette::ROSE),
        swatch(palette::PINE),
        swatch(palette::FOAM),
        swatch(palette::IRIS),
    ]
    .spacing(spacing::XS)
    .into()
}

fn swatch(color: iced::Color) -> Element<'static, Message> {
    container(Space::new().width(Length::Fill))
        .width(18.0)
        .height(18.0)
        .style(move |_| iced::widget::container::Style {
            background: Some(color.into()),
            border: iced::Border {
                color: palette::BORDER_STRONG,
                width: 1.0,
                radius: radius::XS.into(),
            },
            ..Default::default()
        })
        .into()
}
