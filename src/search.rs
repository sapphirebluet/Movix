use iced::widget::{
    button, column, container, pick_list, row, slider, text, text_input, Column, Row, Space,
};
use iced::{Border, Color, Element, Length, Padding, Shadow};

use crate::media::{
    MediaTypeFilter, Message, SortOption, NETFLIX_RED, SURFACE_DARK_GRAY, TEXT_GRAY, TEXT_WHITE,
};
use crate::tmdb::ImageSize;
use crate::Movix;

impl Movix {
    pub fn view_search_page(&self) -> Element<'_, Message> {
        let search_header = self.view_search_header();
        let filter_panel = self.view_filter_panel();
        let search_results = self.view_search_results_grid();

        column![search_header, filter_panel, search_results]
            .spacing(24)
            .padding(Padding::new(100.0).left(48.0).right(48.0).bottom(48.0))
            .width(Length::Fill)
            .into()
    }

    pub fn view_search_header(&self) -> Element<'_, Message> {
        let title_text = format!("Search Results for \"{}\"", self.search_query);
        let title = text(title_text)
            .size(28)
            .color(TEXT_WHITE)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            });

        let result_count = self.filtered_results.len();
        let count_text = if result_count == 1 {
            String::from("1 result found")
        } else {
            format!("{} results found", result_count)
        };
        let count_label = text(count_text).size(16).color(TEXT_GRAY);

        column![title, count_label]
            .spacing(8)
            .width(Length::Fill)
            .into()
    }

    pub fn view_search_results_grid(&self) -> Element<'_, Message> {
        if self.filtered_results.is_empty() {
            return self.view_no_results();
        }

        let cards_per_row = 4;
        let mut rows: Vec<Element<Message>> = Vec::new();

        for chunk in self.filtered_results.chunks(cards_per_row) {
            let row_cards: Vec<Element<Message>> = chunk
                .iter()
                .map(|item| self.view_search_result_card(item))
                .collect();
            let row_element = Row::with_children(row_cards)
                .spacing(16)
                .align_y(iced::Alignment::Start);
            rows.push(row_element.into());
        }

        Column::with_children(rows)
            .spacing(16)
            .width(Length::Fill)
            .into()
    }

    fn view_search_result_card(
        &self,
        media_item: &crate::media::MediaItem,
    ) -> Element<'_, Message> {
        let media_id = media_item.id;
        let (w, h) = (276.0, 155.0);

        if self.hovered_card == Some(media_id) {
            return self.view_search_result_expanded_card(media_item, w, h);
        }

        let backdrop = self.view_search_card_backdrop(media_item, w, h);
        let title_overlay = self.view_search_card_title_overlay(media_item, false);

        let card = container(iced::widget::stack![backdrop, title_overlay])
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 8.0,
                },
                ..Default::default()
            });

        iced::widget::mouse_area(card)
            .on_enter(Message::HoverCard(Some(media_id)))
            .on_exit(Message::HoverCard(None))
            .on_press(Message::OpenDetailPopup(media_id))
            .into()
    }

    fn view_search_result_expanded_card(
        &self,
        media_item: &crate::media::MediaItem,
        w: f32,
        h: f32,
    ) -> Element<'_, Message> {
        let media_id = media_item.id;
        let backdrop = self.view_search_card_backdrop_with_video(media_item, w, h);
        let overlay = self.view_search_card_title_overlay(media_item, true);

        let card = container(iced::widget::stack![backdrop, overlay])
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    offset: iced::Vector::new(0.0, 6.0),
                    blur_radius: 12.0,
                },
                ..Default::default()
            });

        iced::widget::mouse_area(card)
            .on_enter(Message::HoverCard(Some(media_id)))
            .on_exit(Message::HoverCard(None))
            .on_press(Message::OpenDetailPopup(media_id))
            .into()
    }

    fn view_search_card_backdrop(
        &self,
        media_item: &crate::media::MediaItem,
        w: f32,
        h: f32,
    ) -> Element<'_, Message> {
        let handle = media_item.backdrop_path.as_ref().and_then(|path| {
            let url = self
                .tmdb_client
                .as_ref()?
                .image_url(path, ImageSize::Backdrop);
            self.image_cache.get(&url).cloned()
        });

        match handle {
            Some(h_img) => container(
                iced::widget::image(h_img)
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
                    .content_fit(iced::ContentFit::Cover),
            )
            .style(|_| container::Style {
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into(),
            None => container(text("🎬").size(32))
                .width(Length::Fixed(w))
                .height(Length::Fixed(h))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                    border: Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into(),
        }
    }

    fn view_search_card_backdrop_with_video(
        &self,
        media_item: &crate::media::MediaItem,
        w: f32,
        h: f32,
    ) -> Element<'_, Message> {
        if let Some(ref frame) = self.card_video_frame {
            if self.card_player.current_media_id() == Some(media_item.id) {
                return container(
                    iced::widget::image(frame.clone())
                        .width(Length::Fixed(w))
                        .height(Length::Fixed(h))
                        .content_fit(iced::ContentFit::Cover),
                )
                .style(|_| container::Style {
                    border: Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into();
            }
        }
        self.view_search_card_backdrop(media_item, w, h)
    }

    fn view_search_card_title_overlay(
        &self,
        media_item: &crate::media::MediaItem,
        expanded: bool,
    ) -> Element<'_, Message> {
        if expanded {
            self.view_search_card_expanded_overlay(media_item)
        } else {
            self.view_search_card_normal_overlay(media_item)
        }
    }

    fn view_search_card_normal_overlay(
        &self,
        media_item: &crate::media::MediaItem,
    ) -> Element<'_, Message> {
        let logo_handle = media_item.logo_path.as_ref().and_then(|path| {
            let url = self
                .tmdb_client
                .as_ref()?
                .image_url(path, ImageSize::Original);
            self.image_cache.get(&url).cloned()
        });

        let title_text = media_item.title.clone();
        let title: Element<Message> = match logo_handle {
            Some(h) => iced::widget::image(h)
                .width(Length::Fixed(120.0))
                .content_fit(iced::ContentFit::Contain)
                .into(),
            None => text(title_text)
                .size(14)
                .color(TEXT_WHITE)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .wrapping(text::Wrapping::Word)
                .into(),
        };

        container(container(title).padding(10.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom)
            .style(|_| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.7))
                        .add_stop(0.4, Color::from_rgba(0.0, 0.0, 0.0, 0.21))
                        .add_stop(0.6, Color::TRANSPARENT),
                ))),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn view_search_card_expanded_overlay(
        &self,
        media_item: &crate::media::MediaItem,
    ) -> Element<'_, Message> {
        let logo_handle = media_item.logo_path.as_ref().and_then(|path| {
            let url = self
                .tmdb_client
                .as_ref()?
                .image_url(path, ImageSize::Original);
            self.image_cache.get(&url).cloned()
        });

        let title_text = media_item.title.clone();
        let title: Element<Message> = match logo_handle {
            Some(h) => iced::widget::image(h)
                .width(Length::Fixed(100.0))
                .content_fit(iced::ContentFit::Contain)
                .into(),
            None => text(title_text)
                .size(13)
                .color(TEXT_WHITE)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .wrapping(text::Wrapping::Word)
                .into(),
        };

        let play_btn = self.search_play_button(media_item.id);
        let info_btn = self.search_info_button(media_item.id);

        let content = column![
            title,
            row![play_btn, info_btn]
                .spacing(6)
                .align_y(iced::Alignment::Center)
        ]
        .spacing(6)
        .padding(8.0);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom)
            .style(|_| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.85))
                        .add_stop(0.3, Color::from_rgba(0.0, 0.0, 0.0, 0.255))
                        .add_stop(0.5, Color::TRANSPARENT),
                ))),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn search_play_button(&self, media_id: u64) -> Element<'_, Message> {
        button(
            row![
                text("▶").size(10).color(TEXT_WHITE),
                text("Play").size(12).color(TEXT_WHITE)
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(6.0).left(10.0).right(12.0))
        .style(|_, status| {
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
        .on_press(Message::PlayContent(media_id))
        .into()
    }

    fn search_info_button(&self, media_id: u64) -> Element<'_, Message> {
        button(
            container(text("ⓘ").size(14).color(TEXT_WHITE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .padding(0)
        .style(|_, status| {
            let bg = if matches!(status, button::Status::Hovered) {
                Color::from_rgba(1.0, 1.0, 1.0, 0.25)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.5)
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
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
        .on_press(Message::OpenDetailPopup(media_id))
        .into()
    }

    pub fn view_no_results(&self) -> Element<'_, Message> {
        let message = text("No results found")
            .size(24)
            .color(TEXT_GRAY)
            .font(iced::Font {
                weight: iced::font::Weight::Medium,
                ..Default::default()
            });

        let suggestion = text("Try adjusting your search or filters")
            .size(16)
            .color(TEXT_GRAY);

        container(
            column![message, suggestion]
                .spacing(12)
                .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(300.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    pub fn view_filter_panel(&self) -> Element<'_, Message> {
        let media_type_filter = self.view_media_type_filter();
        let genre_dropdown = self.view_genre_dropdown();
        let year_range = self.view_year_range_inputs();
        let rating_slider = self.view_rating_slider();
        let sort_dropdown = self.view_sort_dropdown();
        let reset_button = self.view_reset_button();

        let filter_row = row![
            media_type_filter,
            genre_dropdown,
            year_range,
            rating_slider,
            sort_dropdown,
            Space::new().width(Length::Fill),
            reset_button
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center);

        container(filter_row)
            .width(Length::Fill)
            .padding(Padding::new(16.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    1.0, 1.0, 1.0, 0.05,
                ))),
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_media_type_filter(&self) -> Element<'_, Message> {
        let options = [
            (MediaTypeFilter::All, "All"),
            (MediaTypeFilter::Movies, "Movies"),
            (MediaTypeFilter::TvSeries, "Series"),
        ];

        let buttons: Vec<Element<Message>> = options
            .into_iter()
            .map(|(filter, label)| {
                let is_active = self.search_filters.media_type == filter;
                button(text(label).size(13).color(TEXT_WHITE))
                    .padding(Padding::new(8.0).left(16.0).right(16.0))
                    .style(move |_theme, status| {
                        let bg_alpha = if is_active {
                            0.3
                        } else if matches!(status, button::Status::Hovered) {
                            0.15
                        } else {
                            0.1
                        };
                        button::Style {
                            background: Some(iced::Background::Color(Color::from_rgba(
                                1.0, 1.0, 1.0, bg_alpha,
                            ))),
                            text_color: TEXT_WHITE,
                            border: Border {
                                color: if is_active {
                                    NETFLIX_RED
                                } else {
                                    Color::TRANSPARENT
                                },
                                width: if is_active { 1.0 } else { 0.0 },
                                radius: 4.0.into(),
                            },
                            shadow: Shadow::default(),
                            snap: false,
                        }
                    })
                    .on_press(Message::SetMediaTypeFilter(filter))
                    .into()
            })
            .collect();

        Row::with_children(buttons)
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn view_genre_dropdown(&self) -> Element<'_, Message> {
        let mut options: Vec<String> = vec![String::from("All Genres")];
        options.extend(self.genre_list.iter().map(|g| g.name.clone()));

        let selected = self
            .search_filters
            .genre_id
            .and_then(|id| self.genre_list.iter().find(|g| g.id == id))
            .map(|g| g.name.clone())
            .unwrap_or_else(|| String::from("All Genres"));

        let genre_list = self.genre_list.clone();
        pick_list(options, Some(selected), move |sel| {
            let genre_id = if sel == "All Genres" {
                None
            } else {
                genre_list.iter().find(|g| g.name == sel).map(|g| g.id)
            };
            Message::SetGenreFilter(genre_id)
        })
        .text_size(13)
        .padding(Padding::new(8.0).left(12.0).right(12.0))
        .style(|_, _| pick_list::Style {
            text_color: TEXT_WHITE,
            placeholder_color: TEXT_GRAY,
            handle_color: TEXT_WHITE,
            background: iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1)),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                width: 1.0,
                radius: 4.0.into(),
            },
        })
        .into()
    }

    fn view_year_range_inputs(&self) -> Element<'_, Message> {
        let year_from_value = self
            .search_filters
            .year_from
            .map(|y| y.to_string())
            .unwrap_or_default();

        let year_to_value = self
            .search_filters
            .year_to
            .map(|y| y.to_string())
            .unwrap_or_default();

        let year_input_style = |_theme: &iced::Theme, _status| text_input::Style {
            background: iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1)),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                width: 1.0,
                radius: 4.0.into(),
            },
            icon: TEXT_GRAY,
            placeholder: TEXT_GRAY,
            value: TEXT_WHITE,
            selection: NETFLIX_RED,
        };

        let year_from_input = text_input("From", &year_from_value)
            .on_input(|s| Message::SetYearFrom(s.parse::<u32>().ok()))
            .padding(8)
            .width(Length::Fixed(70.0))
            .style(year_input_style);

        let year_to_input = text_input("To", &year_to_value)
            .on_input(|s| Message::SetYearTo(s.parse::<u32>().ok()))
            .padding(8)
            .width(Length::Fixed(70.0))
            .style(year_input_style);

        row![
            text("Year:").size(13).color(TEXT_GRAY),
            year_from_input,
            text("-").size(13).color(TEXT_GRAY),
            year_to_input
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_rating_slider(&self) -> Element<'_, Message> {
        let rating_value = self.search_filters.min_rating;
        let rating_text = format!("{:.1}+", rating_value);

        let rating_slider_widget = slider(0.0..=10.0, rating_value, Message::SetMinRating)
            .width(Length::Fixed(100.0))
            .height(4.0)
            .step(0.5)
            .style(|_, _| slider::Style {
                rail: slider::Rail {
                    backgrounds: (
                        iced::Background::Color(NETFLIX_RED),
                        iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.2)),
                    ),
                    width: 4.0,
                    border: Border::default(),
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Circle { radius: 6.0 },
                    background: iced::Background::Color(TEXT_WHITE),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            });

        row![
            text("Rating:").size(13).color(TEXT_GRAY),
            rating_slider_widget,
            text(rating_text).size(13).color(TEXT_WHITE)
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_sort_dropdown(&self) -> Element<'_, Message> {
        let options = vec![
            SortOption::Popularity,
            SortOption::Rating,
            SortOption::ReleaseDate,
            SortOption::Alphabetical,
        ];

        pick_list(
            options,
            Some(self.search_filters.sort_by),
            Message::SetSortOption,
        )
        .text_size(13)
        .padding(Padding::new(8.0).left(12.0).right(12.0))
        .style(|_, _| pick_list::Style {
            text_color: TEXT_WHITE,
            placeholder_color: TEXT_GRAY,
            handle_color: TEXT_WHITE,
            background: iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1)),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                width: 1.0,
                radius: 4.0.into(),
            },
        })
        .into()
    }

    fn view_reset_button(&self) -> Element<'_, Message> {
        button(text("Reset").size(13).color(TEXT_WHITE))
            .padding(Padding::new(8.0).left(16.0).right(16.0))
            .style(|_theme, status| {
                let bg_alpha = if matches!(status, button::Status::Hovered) {
                    0.2
                } else {
                    0.1
                };
                button::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(
                        1.0, 1.0, 1.0, bg_alpha,
                    ))),
                    text_color: TEXT_WHITE,
                    border: Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    shadow: Shadow::default(),
                    snap: false,
                }
            })
            .on_press(Message::ResetFilters)
            .into()
    }
}
