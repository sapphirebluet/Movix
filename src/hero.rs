use iced::widget::{button, column, container, row, text, Space};
use iced::{Border, Color, Element, Length, Padding, Shadow};

use crate::media::{
    truncate_description, MediaId, MediaItem, Message, NETFLIX_RED, SURFACE_DARK_GRAY, TEXT_GRAY,
    TEXT_WHITE,
};
use crate::tmdb::ImageSize;
use crate::Movix;

const HERO_HEIGHT: f32 = 620.0;
const ICON_PLAY_FILL: char = '\u{F4F4}';
const ICON_INFO_CIRCLE: char = '\u{F431}';
const ICON_VOLUME_UP_FILL: char = '\u{F611}';
const ICON_VOLUME_MUTE_FILL: char = '\u{F608}';
const ICON_ARROW_CLOCKWISE: char = '\u{F130}';

fn format_runtime(minutes: u32) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    match (h, m) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h {}m", h, m),
    }
}

fn icon(icon_char: char) -> iced::widget::Text<'static> {
    text(icon_char.to_string()).font(iced::Font {
        family: iced::font::Family::Name("bootstrap-icons"),
        ..Default::default()
    })
}

impl Movix {
    pub fn view_hero_section(&self) -> Element<'_, Message> {
        match &self.hero_content {
            Some(media_item) => self.view_hero_with_content(media_item),
            None => self.view_hero_placeholder(),
        }
    }

    pub fn view_hero_placeholder(&self) -> Element<'_, Message> {
        container(
            text("No featured content available")
                .size(24)
                .color(TEXT_GRAY),
        )
        .width(Length::Fill)
        .height(Length::Fixed(HERO_HEIGHT))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
            ..Default::default()
        })
        .into()
    }

    pub fn view_hero_with_content(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let hero_title = self.view_hero_title(media_item);
        let metadata_row = self.view_hero_metadata(media_item);
        let truncated_description = truncate_description(&media_item.overview, 200);
        let hero_description =
            container(text(truncated_description).size(16).color(TEXT_GRAY)).max_width(500.0);

        let media_id = media_item.id;
        let play_button = self.view_hero_play_button(media_id);
        let more_info_button = self.view_hero_more_info_button(media_id);
        let video_control = self.view_hero_video_control();

        let button_row = row![
            play_button,
            more_info_button,
            Space::new().width(Length::Fill),
            video_control
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let hero_text_content = column![hero_title, metadata_row, hero_description, button_row]
            .spacing(20)
            .padding(Padding::new(64.0).left(64.0).right(64.0));

        let hero_left_gradient = container(hero_text_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.99))
                        .add_stop(0.3, Color::from_rgba(0.0, 0.0, 0.0, 0.9))
                        .add_stop(0.5, Color::from_rgba(0.0, 0.0, 0.0, 0.6))
                        .add_stop(0.7, Color::from_rgba(0.0, 0.0, 0.0, 0.25))
                        .add_stop(0.9, Color::TRANSPARENT),
                ))),
                ..Default::default()
            });

        let hero_top_gradient = container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0_f32.to_radians())
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.7))
                        .add_stop(0.15, Color::from_rgba(0.0, 0.0, 0.0, 0.4))
                        .add_stop(0.3, Color::TRANSPARENT),
                ))),
                ..Default::default()
            });

        let hero_bottom_gradient = container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(std::f32::consts::PI)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.15))
                        .add_stop(0.06, Color::from_rgba(0.0, 0.0, 0.0, 0.05))
                        .add_stop(0.12, Color::TRANSPARENT),
                ))),
                ..Default::default()
            });

        let backdrop_element = self.view_hero_backdrop(media_item);

        iced::widget::stack![
            backdrop_element,
            hero_top_gradient,
            hero_bottom_gradient,
            hero_left_gradient
        ]
        .width(Length::Fill)
        .height(Length::Fixed(HERO_HEIGHT))
        .into()
    }

    pub fn get_hero_gradient_color(&self) -> Color {
        Color::BLACK
    }

    pub fn view_hero_title(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let Some(logo_path) = &media_item.logo_path else {
            return self.view_hero_title_text(media_item);
        };
        let Some(client) = &self.tmdb_client else {
            return self.view_hero_title_text(media_item);
        };
        let logo_url = client.image_url(logo_path, ImageSize::Original);
        let Some(handle) = self.image_cache.get(&logo_url) else {
            return self.view_hero_title_text(media_item);
        };
        iced::widget::image(handle.clone())
            .width(Length::Fixed(300.0))
            .content_fit(iced::ContentFit::Contain)
            .into()
    }

    fn view_hero_title_text(&self, media_item: &MediaItem) -> Element<'_, Message> {
        text(media_item.title.clone())
            .size(48)
            .color(TEXT_WHITE)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .into()
    }

    fn view_hero_metadata(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        if let Some(ref date) = media_item.release_date {
            if date.len() >= 4 {
                let year = date[..4].to_string();
                items.push(text(year).size(14).color(TEXT_GRAY).into());
            }
        }

        if let Some(ref cert) = media_item.certification {
            if !items.is_empty() {
                items.push(text("•").size(14).color(TEXT_GRAY).into());
            }
            let cert_text = cert.clone();
            items.push(
                container(text(cert_text).size(12).color(TEXT_WHITE))
                    .padding(Padding::new(2.0).left(6.0).right(6.0))
                    .style(|_theme| container::Style {
                        border: Border {
                            color: TEXT_GRAY,
                            width: 1.0,
                            radius: 2.0.into(),
                        },
                        ..Default::default()
                    })
                    .into(),
            );
        }

        if let Some(runtime) = media_item.runtime {
            if !items.is_empty() {
                items.push(text("•").size(14).color(TEXT_GRAY).into());
            }
            items.push(
                text(format_runtime(runtime))
                    .size(14)
                    .color(TEXT_GRAY)
                    .into(),
            );
        }

        row(items)
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    }

    pub fn view_hero_backdrop(&self, media_item: &MediaItem) -> Element<'_, Message> {
        if let Some(ref frame_handle) = self.hero_video_frame {
            if self.hero_player.current_media_id() == Some(media_item.id) {
                return iced::widget::image(frame_handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fixed(HERO_HEIGHT))
                    .content_fit(iced::ContentFit::Cover)
                    .into();
            }
        }

        let Some(backdrop_path) = &media_item.backdrop_path else {
            return self.view_hero_backdrop_placeholder();
        };
        let Some(client) = &self.tmdb_client else {
            return self.view_hero_backdrop_placeholder();
        };
        let image_url = client.image_url(backdrop_path, ImageSize::Backdrop);
        let Some(handle) = self.image_cache.get(&image_url) else {
            return self.view_hero_backdrop_placeholder();
        };
        iced::widget::image(handle.clone())
            .width(Length::Fill)
            .height(Length::Fixed(HERO_HEIGHT))
            .content_fit(iced::ContentFit::Cover)
            .into()
    }

    fn view_hero_backdrop_placeholder(&self) -> Element<'_, Message> {
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(HERO_HEIGHT))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                ..Default::default()
            })
            .into()
    }

    pub fn view_hero_play_button(&self, media_id: MediaId) -> Element<'_, Message> {
        button(
            row![
                icon(ICON_PLAY_FILL).size(14).color(TEXT_WHITE),
                text("Play").size(16).color(TEXT_WHITE)
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(12.0).left(24.0).right(24.0))
        .style(|_theme, status| {
            let background_color = match status {
                button::Status::Hovered => Color::from_rgb(0.698, 0.027, 0.063),
                _ => NETFLIX_RED,
            };
            button::Style {
                background: Some(iced::Background::Color(background_color)),
                text_color: TEXT_WHITE,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::PlayContent(media_id))
        .into()
    }

    pub fn view_hero_more_info_button(&self, media_id: MediaId) -> Element<'_, Message> {
        button(
            row![
                icon(ICON_INFO_CIRCLE).size(14).color(TEXT_WHITE),
                text("More Info").size(16).color(TEXT_WHITE)
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(12.0).left(24.0).right(24.0))
        .style(|_theme, status| {
            let background_color = match status {
                button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(iced::Background::Color(background_color)),
                text_color: TEXT_WHITE,
                border: Border {
                    color: TEXT_WHITE,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::ShowMoreInfo(media_id))
        .into()
    }

    pub fn view_hero_video_control(&self) -> Element<'_, Message> {
        let has_video = self.hero_video_frame.is_some();
        if !has_video {
            return Space::new().width(0).height(0).into();
        }

        let (icon_char, message) = if self.hero_ended {
            (ICON_ARROW_CLOCKWISE, Message::ReplayHeroTrailer)
        } else if self.hero_muted {
            (ICON_VOLUME_MUTE_FILL, Message::ToggleHeroMute)
        } else {
            (ICON_VOLUME_UP_FILL, Message::ToggleHeroMute)
        };

        button(
            container(icon(icon_char).size(20).color(TEXT_WHITE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .padding(0)
        .style(|_theme, status| {
            let bg_alpha = match status {
                button::Status::Hovered => 0.6,
                _ => 0.4,
            };
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, bg_alpha,
                ))),
                text_color: TEXT_WHITE,
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.3),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(message)
        .into()
    }
}
