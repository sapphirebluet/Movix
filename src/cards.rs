use iced::widget::{button, column, container, row, scrollable, text, Column, Row, Space};
use iced::{Border, Color, Element, Length, Padding, Shadow};

use crate::media::{
    section_id, ContentSection, MediaId, MediaItem, Message, Page, ScrollDirection, NETFLIX_RED,
    SURFACE_DARK_GRAY, TEXT_GRAY, TEXT_WHITE,
};
use crate::tmdb::ImageSize;
use crate::Movix;

const ICON_PLAY_FILL: char = '\u{F4F4}';
const ICON_PLUS_LG: char = '\u{F64D}';
const ICON_INFO_CIRCLE: char = '\u{F431}';
const ICON_FILM: char = '\u{F3A9}';
const ICON_CHEVRON_LEFT: char = '\u{F284}';
const ICON_CHEVRON_RIGHT: char = '\u{F285}';

const CARD_WIDTH: f32 = 150.0;
const CARD_HEIGHT: f32 = 225.0;
const EXPANDED_WIDTH: f32 = 400.0;
const EXPANDED_HEIGHT: f32 = 225.0;

fn icon(icon_char: char) -> iced::widget::Text<'static> {
    text(icon_char.to_string()).font(iced::Font {
        family: iced::font::Family::Name("bootstrap-icons"),
        ..Default::default()
    })
}

fn hidden_horizontal_scrollbar_style(
    _theme: &iced::Theme,
    _status: scrollable::Status,
) -> scrollable::Style {
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: iced::Background::Color(Color::TRANSPARENT),
                border: Border::default(),
            },
        },
        horizontal_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: iced::Background::Color(Color::TRANSPARENT),
                border: Border::default(),
            },
        },
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

impl Movix {
    pub fn view_content_sections(&self) -> Element<'_, Message> {
        let gradient_color = self.get_hero_gradient_color();

        let mut sections: Vec<Element<Message>> = Vec::new();

        for (index, section) in self.content_sections.iter().enumerate() {
            if index == 0 {
                let section_element = self.view_content_section_with_arrows(section, index);
                let with_gradient = container(section_element)
                    .width(Length::Fill)
                    .padding(iced::Padding::new(48.0).top(0.0).bottom(0.0))
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                            iced::gradient::Linear::new(std::f32::consts::PI)
                                .add_stop(0.0, gradient_color)
                                .add_stop(0.5, Color::from_rgba(0.0, 0.0, 0.0, 0.0)),
                        ))),
                        ..Default::default()
                    });
                sections.push(with_gradient.into());
            } else {
                sections.push(
                    container(self.view_content_section_with_arrows(section, index))
                        .padding(iced::Padding::new(0.0).left(48.0).right(48.0))
                        .into(),
                );
            }
        }

        Column::with_children(sections)
            .spacing(48)
            .padding(iced::Padding::new(32.0).left(0.0).right(0.0).top(0.0))
            .width(Length::Fill)
            .into()
    }

    pub fn view_content_section_with_arrows(
        &self,
        section: &ContentSection,
        section_index: usize,
    ) -> Element<'_, Message> {
        let section_title =
            text(section.title.clone())
                .size(24)
                .color(TEXT_WHITE)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                });

        let cards: Vec<Element<Message>> = section
            .items
            .iter()
            .take(20)
            .map(|item| self.view_movie_card(item))
            .collect();

        let cards_row = Row::with_children(cards)
            .spacing(16)
            .align_y(iced::Alignment::Start);

        let Some(section_id_str) = section_id(section_index) else {
            return self.view_content_section(section);
        };
        let scrollable_id = iced::widget::Id::new(section_id_str);
        let section_idx = section_index;
        let scrollable_cards = scrollable(cards_row)
            .id(scrollable_id)
            .on_scroll(move |viewport| {
                Message::SectionScrolled(section_idx, viewport.absolute_offset().x)
            })
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ))
            .width(Length::Fill)
            .style(hidden_horizontal_scrollbar_style);

        let is_hovered = self.hovered_section == Some(section_index);
        let scroll_offset = self
            .section_scroll_offsets
            .get(section_index)
            .copied()
            .unwrap_or(0.0);
        let scroll_target = self
            .section_scroll_targets
            .get(section_index)
            .copied()
            .unwrap_or(0.0);

        let card_count = section.items.len().min(20);
        let total_width = (card_count as f32) * (CARD_WIDTH + 12.0) - 12.0;
        let can_scroll_left = scroll_target > 0.0 || scroll_offset > 1.0;
        let can_scroll_right = total_width > 800.0 && scroll_target < total_width - 800.0;

        let cards_with_arrows = self.view_scrollable_with_arrows(
            scrollable_cards.into(),
            section_index,
            is_hovered,
            can_scroll_left,
            can_scroll_right,
        );

        let section_content = iced::widget::column![section_title, cards_with_arrows]
            .spacing(20)
            .width(Length::Fill);

        iced::widget::mouse_area(section_content)
            .on_enter(Message::HoverSection(Some(section_index)))
            .on_exit(Message::HoverSection(None))
            .into()
    }

    fn view_scrollable_with_arrows<'a>(
        &'a self,
        scrollable_content: Element<'a, Message>,
        section_index: usize,
        is_hovered: bool,
        can_scroll_left: bool,
        can_scroll_right: bool,
    ) -> Element<'a, Message> {
        let left_arrow: Element<'a, Message> = if is_hovered && can_scroll_left {
            self.view_scroll_arrow(section_index, ScrollDirection::Left)
        } else {
            container(Space::new().width(0).height(0)).into()
        };

        let right_arrow: Element<'a, Message> = if is_hovered && can_scroll_right {
            self.view_scroll_arrow(section_index, ScrollDirection::Right)
        } else {
            container(Space::new().width(0).height(0)).into()
        };

        let left_overlay = container(left_arrow)
            .width(Length::Fill)
            .height(Length::Fixed(CARD_HEIGHT))
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Center);

        let right_overlay = container(right_arrow)
            .width(Length::Fill)
            .height(Length::Fixed(CARD_HEIGHT))
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Center);

        iced::widget::stack![scrollable_content, left_overlay, right_overlay]
            .width(Length::Fill)
            .height(Length::Fixed(CARD_HEIGHT))
            .into()
    }

    fn view_scroll_arrow(
        &self,
        section_index: usize,
        direction: ScrollDirection,
    ) -> Element<'_, Message> {
        let icon_char = match direction {
            ScrollDirection::Left => ICON_CHEVRON_LEFT,
            ScrollDirection::Right => ICON_CHEVRON_RIGHT,
        };

        let arrow_button = button(
            container(icon(icon_char).size(24).color(TEXT_WHITE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::Fixed(48.0))
        .height(Length::Fixed(80.0))
        .padding(0)
        .style(|_theme, status| {
            let bg_alpha = match status {
                button::Status::Hovered => 0.9,
                _ => 0.7,
            };
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, bg_alpha,
                ))),
                text_color: TEXT_WHITE,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    offset: iced::Vector::new(0.0, 2.0),
                    blur_radius: 8.0,
                },
                snap: false,
            }
        })
        .on_press(Message::ScrollSection(section_index, direction));

        arrow_button.into()
    }

    pub fn view_content_section(&self, section: &ContentSection) -> Element<'_, Message> {
        let section_title =
            text(section.title.clone())
                .size(24)
                .color(TEXT_WHITE)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                });

        let cards: Vec<Element<Message>> = section
            .items
            .iter()
            .take(20)
            .map(|item| self.view_movie_card(item))
            .collect();

        let cards_row = Row::with_children(cards)
            .spacing(16)
            .align_y(iced::Alignment::Start);

        let scrollable_cards = scrollable(cards_row)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ))
            .width(Length::Fill)
            .style(hidden_horizontal_scrollbar_style);

        iced::widget::column![section_title, scrollable_cards]
            .spacing(20)
            .width(Length::Fill)
            .into()
    }

    pub fn view_movie_card(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let media_id = media_item.id;
        let is_hovered = self.hovered_card == Some(media_id);

        if is_hovered {
            return self.view_expanded_card(media_item);
        }

        let poster_content = self.view_card_poster(media_item, CARD_WIDTH, CARD_HEIGHT);

        let card_container = container(poster_content)
            .width(Length::Fixed(CARD_WIDTH))
            .height(Length::Fixed(CARD_HEIGHT))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 8.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 8.0,
                },
                ..Default::default()
            });

        iced::widget::mouse_area(card_container)
            .on_enter(Message::HoverCard(Some(media_id)))
            .on_exit(Message::HoverCard(None))
            .on_press(Message::NavigateTo(Page::Detail(media_id)))
            .into()
    }

    pub fn view_expanded_card(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let media_id = media_item.id;
        let backdrop_content = self.view_card_backdrop_with_load(media_item);
        let hover_overlay = self.view_expanded_hover_overlay(media_item);

        let stacked_content = iced::widget::stack![backdrop_content, hover_overlay];

        let card_container = container(stacked_content)
            .width(Length::Fixed(EXPANDED_WIDTH))
            .height(Length::Fixed(EXPANDED_HEIGHT))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 8.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    offset: iced::Vector::new(0.0, 6.0),
                    blur_radius: 12.0,
                },
                ..Default::default()
            });

        iced::widget::mouse_area(card_container)
            .on_enter(Message::HoverCard(Some(media_id)))
            .on_exit(Message::HoverCard(None))
            .on_press(Message::NavigateTo(Page::Detail(media_id)))
            .into()
    }

    pub fn view_card_backdrop_with_load(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let media_id = media_item.id;

        if let Some(ref frame_handle) = self.card_video_frame {
            if let Ok(player) = self.card_player.try_lock() {
                if player.current_media_id() == Some(media_id) {
                    return container(
                        iced::widget::image(frame_handle.clone())
                            .width(Length::Fixed(EXPANDED_WIDTH))
                            .height(Length::Fixed(EXPANDED_HEIGHT))
                            .content_fit(iced::ContentFit::Cover),
                    )
                    .style(|_theme| container::Style {
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into();
                }
            }
        }

        if let Some(backdrop_path) = &media_item.backdrop_path {
            if let Some(client) = &self.tmdb_client {
                let image_url = client.image_url(backdrop_path, ImageSize::Backdrop);
                if let Some(handle) = self.image_cache.get(&image_url) {
                    return container(
                        iced::widget::image(handle.clone())
                            .width(Length::Fixed(EXPANDED_WIDTH))
                            .height(Length::Fixed(EXPANDED_HEIGHT))
                            .content_fit(iced::ContentFit::Cover),
                    )
                    .style(|_theme| container::Style {
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into();
                }
            }
        }
        container(Space::new().width(EXPANDED_WIDTH).height(EXPANDED_HEIGHT))
            .width(Length::Fixed(EXPANDED_WIDTH))
            .height(Length::Fixed(EXPANDED_HEIGHT))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    pub fn view_expanded_hover_overlay(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let media_id = media_item.id;
        let title_element = self.view_expanded_card_title(media_item);

        let play_button = self.view_expanded_play_button(media_id);
        let add_button = self.view_expanded_action_button(media_id, ICON_PLUS_LG, false);
        let info_button = self.view_expanded_action_button(media_id, ICON_INFO_CIRCLE, true);

        let action_buttons = row![play_button, add_button, info_button]
            .spacing(6)
            .align_y(iced::Alignment::Center);

        let content_column = column![title_element, action_buttons]
            .spacing(8)
            .padding(Padding::new(10.0));

        let content_container = container(content_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom);

        let bottom_gradient = container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(std::f32::consts::PI)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.85))
                        .add_stop(0.25, Color::from_rgba(0.0, 0.0, 0.0, 0.4))
                        .add_stop(0.45, Color::from_rgba(0.0, 0.0, 0.0, 0.1))
                        .add_stop(0.6, Color::TRANSPARENT),
                ))),
                ..Default::default()
            });

        let left_gradient = container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.6))
                        .add_stop(0.25, Color::from_rgba(0.0, 0.0, 0.0, 0.2))
                        .add_stop(0.45, Color::from_rgba(0.0, 0.0, 0.0, 0.05))
                        .add_stop(0.6, Color::TRANSPARENT),
                ))),
                ..Default::default()
            });

        container(
            iced::widget::stack![bottom_gradient, left_gradient, content_container]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fixed(EXPANDED_WIDTH))
        .height(Length::Fixed(EXPANDED_HEIGHT))
        .style(|_theme| container::Style {
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    pub fn view_expanded_card_title(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let handle = media_item.logo_path.as_ref().and_then(|logo_path| {
            let client = self.tmdb_client.as_ref()?;
            let logo_url = client.image_url(logo_path, ImageSize::Original);
            self.image_cache.get(&logo_url)
        });

        match handle {
            Some(h) => iced::widget::image(h.clone())
                .width(Length::Fixed(140.0))
                .content_fit(iced::ContentFit::Contain)
                .into(),
            None => text(media_item.title.clone())
                .size(14)
                .color(TEXT_WHITE)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .wrapping(text::Wrapping::Word)
                .into(),
        }
    }

    pub fn view_expanded_play_button(&self, media_id: MediaId) -> Element<'_, Message> {
        button(
            row![
                icon(ICON_PLAY_FILL).size(14).color(TEXT_WHITE),
                text("Play").size(13).color(TEXT_WHITE)
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(10.0).left(14.0).right(16.0))
        .style(|_theme, status| {
            let bg_color = match status {
                button::Status::Hovered => Color::from_rgb(0.698, 0.027, 0.063),
                _ => NETFLIX_RED,
            };
            button::Style {
                background: Some(iced::Background::Color(bg_color)),
                text_color: TEXT_WHITE,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 6.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::PlayContent(media_id))
        .into()
    }

    pub fn view_expanded_action_button(
        &self,
        media_id: MediaId,
        icon_char: char,
        is_info: bool,
    ) -> Element<'_, Message> {
        let button_size = 36.0;
        let message = if is_info {
            Message::ShowMoreInfo(media_id)
        } else {
            Message::HoverCard(Some(media_id))
        };

        button(
            container(icon(icon_char).size(16).color(TEXT_WHITE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::Fixed(button_size))
        .height(Length::Fixed(button_size))
        .padding(0)
        .style(move |_theme, status| {
            let bg_color = match status {
                button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.25),
                _ => Color::from_rgba(0.0, 0.0, 0.0, 0.5),
            };
            button::Style {
                background: Some(iced::Background::Color(bg_color)),
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
        .on_press(message)
        .into()
    }

    pub fn view_card_poster(
        &self,
        media_item: &MediaItem,
        width: f32,
        height: f32,
    ) -> Element<'_, Message> {
        let handle = media_item.poster_path.as_ref().and_then(|poster_path| {
            let client = self.tmdb_client.as_ref()?;
            let image_url = client.image_url(poster_path, ImageSize::Poster);
            self.image_cache.get(&image_url)
        });

        match handle {
            Some(h) => iced::widget::image(h.clone())
                .width(Length::Fixed(width))
                .height(Length::Fixed(height))
                .content_fit(iced::ContentFit::Cover)
                .into(),
            None => self.view_card_placeholder(width, height),
        }
    }

    pub fn view_card_placeholder(&self, width: f32, height: f32) -> Element<'_, Message> {
        container(icon(ICON_FILM).size(48).color(TEXT_GRAY))
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                ..Default::default()
            })
            .into()
    }
}
