use iced::widget::{button, column, container, row, scrollable, text, Column, Row, Space};
use iced::{Border, Color, Element, Length, Padding, Shadow};

use crate::media::{
    DetailPopupData, MediaItem, MediaType, Message, NETFLIX_RED, SURFACE_DARK_GRAY, TEXT_GRAY,
    TEXT_WHITE,
};
use crate::tmdb::ImageSize;
use crate::Movix;

const POPUP_WIDTH: f32 = 920.0;
const MINI_HERO_HEIGHT: f32 = 420.0;

pub const ICON_X_LG: char = '\u{F659}';
pub const ICON_PLAY_FILL: char = '\u{F4F4}';
pub const ICON_PLUS_LG: char = '\u{F64D}';
pub const ICON_FILM: char = '\u{F3A9}';
pub const ICON_PERSON_FILL: char = '\u{F4DA}';
pub const ICON_GLOBE: char = '\u{F3EF}';

pub fn icon(icon_char: char) -> iced::widget::Text<'static> {
    text(icon_char.to_string()).font(iced::Font {
        family: iced::font::Family::Name("bootstrap-icons"),
        ..Default::default()
    })
}

pub fn format_full_date(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 || date_str.len() < 10 {
        return date_str.to_string();
    }
    format!("{}/{}/{}", parts[1], parts[2], parts[0])
}

pub fn format_rating_with_star(rating: f32) -> String {
    format!("{:.1}★", rating)
}

pub fn format_currency(amount: u64) -> String {
    if amount == 0 {
        return String::from("N/A");
    }
    let formatted = amount
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",");
    format!("${}", formatted)
}

pub fn format_genres(genres: &[crate::media::Genre]) -> String {
    genres
        .iter()
        .map(|g| g.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn format_episode_number(season: u32, episode: u32) -> String {
    format!("S{} E{}", season, episode)
}

fn format_runtime(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h {}m", h, m),
    }
}

pub fn hidden_scrollbar_style(
    _theme: &iced::Theme,
    _status: scrollable::Status,
) -> scrollable::Style {
    let transparent_rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: Border::default(),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: transparent_rail.clone(),
        horizontal_rail: transparent_rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

fn popup_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(
            0.078, 0.078, 0.078,
        ))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
            offset: iced::Vector::new(0.0, 25.0),
            blur_radius: 50.0,
        },
        ..Default::default()
    }
}

impl Movix {
    pub fn view_detail_popup_overlay(&self) -> Element<'_, Message> {
        let Some(data) = &self.detail_popup_data else {
            return self.view_detail_loading_popup();
        };

        let popup_with_close = iced::widget::stack![
            self.view_detail_popup_content(data),
            self.view_detail_close_button()
        ]
        .width(Length::Fixed(POPUP_WIDTH))
        .height(Length::Fill);

        let popup = container(popup_with_close)
            .max_width(POPUP_WIDTH)
            .clip(true)
            .style(popup_container_style);

        let popup_mouse_area = iced::widget::mouse_area(popup);

        let overlay_bg = iced::widget::mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(
                        0.0, 0.0, 0.0, 0.85,
                    ))),
                    ..Default::default()
                }),
        )
        .on_press(Message::CloseDetailPopup);

        let centered_popup = container(popup_mouse_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .padding(Padding::new(40.0));

        iced::widget::stack![overlay_bg, centered_popup]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_detail_loading_popup(&self) -> Element<'_, Message> {
        let skeleton_hero = container(Space::new().width(Length::Fill).height(MINI_HERO_HEIGHT))
            .width(Length::Fill)
            .height(Length::Fixed(MINI_HERO_HEIGHT))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                border: Border {
                    radius: 16.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let skeleton_title =
            container(Space::new().width(200).height(32)).style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let skeleton_meta =
            container(Space::new().width(150).height(16)).style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.18))),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let skeleton_desc1 =
            container(Space::new().width(Length::Fill).height(14)).style(|_theme| {
                container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        let skeleton_desc2 =
            container(Space::new().width(300).height(14)).style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let content_skeleton = column![
            skeleton_title,
            skeleton_meta,
            skeleton_desc1,
            skeleton_desc2
        ]
        .spacing(12)
        .padding(Padding::new(24.0))
        .width(Length::FillPortion(2));

        let cast_skeleton: Vec<Element<Message>> = (0..4)
            .map(|_| {
                row![
                    container(Space::new().width(50).height(50)).style(|_theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgb(
                            0.18, 0.18, 0.18
                        ))),
                        border: Border {
                            radius: 25.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    column![
                        container(Space::new().width(80).height(14)).style(|_theme| {
                            container::Style {
                                background: Some(iced::Background::Color(Color::from_rgb(
                                    0.2, 0.2, 0.2,
                                ))),
                                border: Border {
                                    radius: 4.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        }),
                        container(Space::new().width(60).height(12)).style(|_theme| {
                            container::Style {
                                background: Some(iced::Background::Color(Color::from_rgb(
                                    0.15, 0.15, 0.15,
                                ))),
                                border: Border {
                                    radius: 4.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })
                    ]
                    .spacing(4)
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center)
                .into()
            })
            .collect();

        let cast_section = column![
            container(Space::new().width(80).height(16)).style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            Column::with_children(cast_skeleton).spacing(12)
        ]
        .spacing(12)
        .width(Length::FillPortion(1));

        let body = row![content_skeleton, cast_section]
            .spacing(32)
            .padding(Padding::new(32.0));

        let popup_content = scrollable(column![skeleton_hero, body].width(Length::Fill))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(hidden_scrollbar_style);

        let close_btn = self.view_detail_close_button();

        let popup_with_close = iced::widget::stack![popup_content, close_btn]
            .width(Length::Fixed(POPUP_WIDTH))
            .height(Length::Fill);

        let popup = container(popup_with_close)
            .max_width(POPUP_WIDTH)
            .clip(true)
            .style(popup_container_style);

        let popup_mouse_area = iced::widget::mouse_area(popup);

        let overlay_bg = iced::widget::mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(
                        0.0, 0.0, 0.0, 0.85,
                    ))),
                    ..Default::default()
                }),
        )
        .on_press(Message::CloseDetailPopup);

        let centered_popup = container(popup_mouse_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .padding(Padding::new(48.0));

        iced::widget::stack![overlay_bg, centered_popup]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_detail_close_button(&self) -> Element<'_, Message> {
        let btn = button(
            container(icon(ICON_X_LG).size(20).color(TEXT_WHITE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(36.0))
        .padding(0)
        .style(|_theme, status| {
            let alpha = if matches!(status, button::Status::Hovered) {
                0.8
            } else {
                0.6
            };
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, alpha,
                ))),
                text_color: TEXT_WHITE,
                border: Border {
                    radius: 18.0.into(),
                    ..Default::default()
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::CloseDetailPopup);

        container(btn)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .padding(Padding::new(20.0))
            .into()
    }

    fn view_detail_popup_content(&self, data: &DetailPopupData) -> Element<'_, Message> {
        let mut sections: Vec<Element<Message>> = vec![self.view_detail_mini_hero(data)];

        if matches!(data.media_item.media_type, MediaType::TvSeries) && !data.seasons.is_empty() {
            sections.push(self.view_detail_seasons_section(data));
        }

        sections.push(self.view_detail_content_and_cast(data));

        if let Some(collection) = &data.collection {
            sections.push(self.view_detail_collection_section(collection));
        }
        if !data.similar.is_empty() {
            sections.push(self.view_detail_similar_section(&data.similar));
        }

        sections.push(self.view_detail_advanced_info(data));

        scrollable(Column::with_children(sections).width(Length::Fill))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(hidden_scrollbar_style)
            .into()
    }

    pub fn view_detail_mini_hero(&self, data: &DetailPopupData) -> Element<'_, Message> {
        let backdrop = self.view_detail_backdrop(&data.media_item);
        let gradient = container(self.view_detail_hero_content(&data.media_item))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(std::f32::consts::PI)
                        .add_stop(0.0, Color::TRANSPARENT)
                        .add_stop(0.4, Color::from_rgba(0.0, 0.0, 0.0, 0.3))
                        .add_stop(0.6, Color::from_rgba(0.078, 0.078, 0.078, 0.7))
                        .add_stop(1.0, Color::from_rgba(0.078, 0.078, 0.078, 1.0)),
                ))),
                border: Border {
                    radius: 16.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        container(
            iced::widget::stack![backdrop, gradient]
                .width(Length::Fill)
                .height(Length::Fixed(MINI_HERO_HEIGHT)),
        )
        .clip(true)
        .style(|_theme| container::Style {
            border: Border {
                radius: 16.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    fn view_detail_backdrop(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let media_id = media_item.id;

        if let Some(ref frame_handle) = self.detail_video_frame {
            if let Ok(player) = self.detail_player.try_lock() {
                if player.current_media_id() == Some(media_id) {
                    return iced::widget::image(frame_handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fixed(MINI_HERO_HEIGHT))
                        .content_fit(iced::ContentFit::Cover)
                        .border_radius(16.0)
                        .into();
                }
            }
        }

        let handle = media_item.backdrop_path.as_ref().and_then(|path| {
            let url = self
                .tmdb_client
                .as_ref()?
                .image_url(path, ImageSize::Backdrop);
            self.image_cache.get(&url)
        });

        match handle {
            Some(h) => iced::widget::image(h.clone())
                .width(Length::Fill)
                .height(Length::Fixed(MINI_HERO_HEIGHT))
                .content_fit(iced::ContentFit::Cover)
                .border_radius(16.0)
                .into(),
            None => container(Space::new().width(Length::Fill).height(MINI_HERO_HEIGHT))
                .width(Length::Fill)
                .height(Length::Fixed(MINI_HERO_HEIGHT))
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                    border: Border {
                        radius: 16.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into(),
        }
    }

    fn view_detail_hero_content(&self, media_item: &MediaItem) -> Element<'_, Message> {
        column![
            self.view_detail_title(media_item),
            self.view_detail_hero_metadata(media_item),
            self.view_detail_hero_buttons(media_item.id)
        ]
        .spacing(16)
        .padding(Padding::new(32.0))
        .width(Length::Fill)
        .into()
    }

    fn view_detail_title(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let handle = media_item.logo_path.as_ref().and_then(|path| {
            let url = self
                .tmdb_client
                .as_ref()?
                .image_url(path, ImageSize::Original);
            self.image_cache.get(&url)
        });

        match handle {
            Some(h) => iced::widget::image(h.clone())
                .width(Length::Fixed(350.0))
                .content_fit(iced::ContentFit::Contain)
                .into(),
            None => text(media_item.title.clone())
                .size(32)
                .color(TEXT_WHITE)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .into(),
        }
    }

    fn view_detail_hero_metadata(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        if let Some(ref date) = media_item.release_date {
            if date.len() >= 4 {
                items.push(text(date[..4].to_string()).size(14).color(TEXT_GRAY).into());
            }
        }

        if media_item.vote_average > 0.0 {
            if !items.is_empty() {
                items.push(text("•").size(14).color(TEXT_GRAY).into());
            }
            items.push(
                text(format_rating_with_star(media_item.vote_average))
                    .size(14)
                    .color(Color::from_rgb(1.0, 0.84, 0.0))
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
        } else if let Some(episodes) = media_item.number_of_episodes {
            if !items.is_empty() {
                items.push(text("•").size(14).color(TEXT_GRAY).into());
            }
            items.push(
                text(format!("{} Episodes", episodes))
                    .size(14)
                    .color(TEXT_GRAY)
                    .into(),
            );
        }

        Row::with_children(items)
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn view_detail_hero_buttons(&self, media_id: u64) -> Element<'_, Message> {
        let play = button(
            row![
                icon(ICON_PLAY_FILL).size(16).color(TEXT_WHITE),
                text("Play").size(16).color(TEXT_WHITE)
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(12.0).left(24.0).right(24.0))
        .style(|_theme, status| {
            let bg = if matches!(status, button::Status::Hovered) {
                Color::from_rgb(0.698, 0.027, 0.063)
            } else {
                NETFLIX_RED
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: TEXT_WHITE,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::PlayContent(media_id));

        let list = button(
            row![
                icon(ICON_PLUS_LG).size(16).color(TEXT_WHITE),
                text("My List").size(16).color(TEXT_WHITE)
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(12.0).left(24.0).right(24.0))
        .style(|_theme, status| {
            let alpha = if matches!(status, button::Status::Hovered) {
                0.15
            } else {
                0.1
            };
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    1.0, 1.0, 1.0, alpha,
                ))),
                text_color: TEXT_WHITE,
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.3),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::HoverCard(None));

        row![play, list]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn view_detail_content_and_cast(&self, data: &DetailPopupData) -> Element<'_, Message> {
        row![
            self.view_detail_content_section(data),
            self.view_detail_cast_section(&data.cast)
        ]
        .spacing(32)
        .padding(Padding::new(32.0))
        .width(Length::Fill)
        .into()
    }

    pub fn view_detail_content_section(&self, data: &DetailPopupData) -> Element<'_, Message> {
        let media = &data.media_item;
        let mut items: Vec<Element<Message>> = vec![self.view_detail_content_metadata(media)];

        if let Some(ref tagline) = media.tagline {
            if !tagline.is_empty() {
                items.push(
                    text(format!("\"{}\"", tagline))
                        .size(20)
                        .color(TEXT_WHITE)
                        .font(iced::Font {
                            style: iced::font::Style::Italic,
                            ..Default::default()
                        })
                        .into(),
                );
            }
        }

        if !media.overview.is_empty() {
            items.push(
                text(media.overview.clone())
                    .size(16)
                    .color(TEXT_GRAY)
                    .into(),
            );
        }

        Column::with_children(items)
            .spacing(20)
            .width(Length::FillPortion(2))
            .into()
    }

    fn view_detail_content_metadata(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        if let Some(ref date) = media_item.release_date {
            items.push(
                text(format_full_date(date))
                    .size(14)
                    .color(TEXT_GRAY)
                    .into(),
            );
        }

        if !media_item.genres.is_empty() {
            if !items.is_empty() {
                items.push(text("•").size(14).color(TEXT_GRAY).into());
            }
            items.push(
                text(format_genres(&media_item.genres))
                    .size(14)
                    .color(TEXT_GRAY)
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

        if media_item.vote_average > 0.0 {
            if !items.is_empty() {
                items.push(text("•").size(14).color(TEXT_GRAY).into());
            }
            items.push(
                text(format_rating_with_star(media_item.vote_average))
                    .size(14)
                    .color(Color::from_rgb(1.0, 0.84, 0.0))
                    .into(),
            );
        }

        row(items)
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    }
}
