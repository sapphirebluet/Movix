use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, Column, Row, Space,
};
use iced::{Border, Color, Element, Length, Padding, Shadow};

use crate::detail_popup::{
    format_episode_number, format_full_date, hidden_scrollbar_style, icon, ICON_FILM, ICON_GLOBE,
    ICON_PERSON_FILL, ICON_PLAY_FILL,
};
use crate::media::{
    CastMember, Collection, Episode, ExternalIds, Keyword, MediaItem, Message, ProductionCompany,
    SURFACE_DARK_GRAY, TEXT_GRAY, TEXT_WHITE,
};
use crate::tmdb::ImageSize;
use crate::Movix;

const ICON_INFO_CIRCLE: char = '\u{F431}';

fn rounded_style(radius: f32, bg: Option<Color>) -> container::Style {
    container::Style {
        background: bg.map(iced::Background::Color),
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn pill_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
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
            radius: 4.0.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

impl Movix {
    fn bold_text(s: impl ToString, size: u16, color: Color) -> iced::widget::Text<'static> {
        text(s.to_string())
            .size(size as u32)
            .color(color)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
    }

    fn horizontal_scroll(content: Row<'_, Message>) -> Element<'_, Message> {
        scrollable(content)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ))
            .width(Length::Fill)
            .style(hidden_scrollbar_style)
            .into()
    }

    fn get_cached_image(
        &self,
        path: Option<&String>,
        size: ImageSize,
    ) -> Option<iced::widget::image::Handle> {
        let url = self.tmdb_client.as_ref()?.image_url(path?, size);
        self.image_cache.get(&url).cloned()
    }

    fn image_or_placeholder<'a>(
        handle: Option<iced::widget::image::Handle>,
        width: f32,
        height: f32,
        radius: f32,
        placeholder: Element<'a, Message>,
    ) -> Element<'a, Message> {
        match handle {
            Some(h) => container(
                iced::widget::image(h)
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(height))
                    .content_fit(iced::ContentFit::Cover),
            )
            .style(move |_| rounded_style(radius, None))
            .into(),
            None => container(placeholder)
                .width(Length::Fixed(width))
                .height(Length::Fixed(height))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(move |_| rounded_style(radius, Some(Color::from_rgba(0.2, 0.2, 0.2, 0.5))))
                .into(),
        }
    }

    pub fn view_detail_seasons_section(
        &self,
        data: &crate::media::DetailPopupData,
    ) -> Element<'_, Message> {
        let season_numbers: Vec<(String, Option<u32>)> =
            std::iter::once((String::from("All Seasons"), None))
                .chain(
                    data.seasons
                        .iter()
                        .map(|s| (s.name.clone(), Some(s.season_number))),
                )
                .collect();

        let options: Vec<String> = season_numbers.iter().map(|(n, _)| n.clone()).collect();
        let selected = self
            .detail_selected_season
            .and_then(|num| data.seasons.iter().find(|s| s.season_number == num))
            .map(|s| s.name.clone())
            .unwrap_or_else(|| String::from("All Seasons"));

        let picker = pick_list(options, Some(selected), move |sel| {
            let num = season_numbers
                .iter()
                .find(|(n, _)| *n == sel)
                .and_then(|(_, n)| *n);
            Message::DetailSelectSeason(num)
        })
        .text_size(14)
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
        });

        let header = row![
            Self::bold_text("Seasons", 18, TEXT_WHITE),
            Space::new().width(Length::Fill),
            picker
        ]
        .align_y(iced::Alignment::Center);

        let episodes: Element<Message> = if self.detail_episodes.is_empty() {
            container(text("No episodes available").size(14).color(TEXT_GRAY))
                .padding(16.0)
                .into()
        } else {
            let cards: Vec<Element<Message>> = self
                .detail_episodes
                .iter()
                .map(|ep| self.view_detail_episode_card(ep))
                .collect();
            Self::horizontal_scroll(
                Row::with_children(cards)
                    .spacing(12)
                    .align_y(iced::Alignment::Start),
            )
        };

        container(column![header, episodes].spacing(20).width(Length::Fill))
            .width(Length::Fill)
            .padding(Padding::new(32.0))
            .style(|_| rounded_style(0.0, Some(Color::from_rgba(1.0, 1.0, 1.0, 0.03))))
            .into()
    }

    fn view_detail_episode_card(&self, episode: &Episode) -> Element<'_, Message> {
        let handle = self.get_cached_image(episode.still_path.as_ref(), ImageSize::Backdrop);
        let still = Self::image_or_placeholder(
            handle,
            160.0,
            90.0,
            4.0,
            icon(ICON_FILM).size(24).color(TEXT_GRAY).into(),
        );

        let air_date = episode
            .air_date
            .as_ref()
            .map(|d| format_full_date(d))
            .unwrap_or_default();
        let meta = row![
            Self::bold_text(
                format_episode_number(episode.season_number, episode.episode_number),
                13,
                TEXT_WHITE
            ),
            text(air_date).size(12).color(TEXT_GRAY)
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let title = container(
            text(episode.name.clone())
                .size(14)
                .color(TEXT_WHITE)
                .wrapping(text::Wrapping::Word),
        )
        .max_width(148.0);

        container(
            column![still, meta, title]
                .spacing(6)
                .width(Length::Fixed(160.0)),
        )
        .width(Length::Fixed(160.0))
        .into()
    }

    pub fn view_detail_cast_section(&self, cast: &[CastMember]) -> Element<'_, Message> {
        let list: Vec<Element<Message>> = cast
            .iter()
            .take(4)
            .map(|m| {
                let handle = self.get_cached_image(m.profile_path.as_ref(), ImageSize::Poster);
                let profile = Self::image_or_placeholder(
                    handle,
                    50.0,
                    50.0,
                    25.0,
                    icon(ICON_PERSON_FILL).size(20).color(TEXT_GRAY).into(),
                );
                row![
                    profile,
                    column![
                        Self::bold_text(&m.name, 14, TEXT_WHITE),
                        text(m.character.clone()).size(12).color(TEXT_GRAY)
                    ]
                    .spacing(2)
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center)
                .into()
            })
            .collect();

        column![
            Self::bold_text("Top Cast", 16, TEXT_WHITE),
            Column::with_children(list).spacing(16)
        ]
        .spacing(16)
        .width(Length::FillPortion(1))
        .into()
    }

    pub fn view_detail_collection_section(&self, collection: &Collection) -> Element<'_, Message> {
        self.view_detail_media_row_section(&collection.name, &collection.parts)
    }

    pub fn view_detail_similar_section(&self, similar: &[MediaItem]) -> Element<'_, Message> {
        self.view_detail_media_row_section("Similar Titles", similar)
    }

    fn view_detail_media_row_section(
        &self,
        title: &str,
        items: &[MediaItem],
    ) -> Element<'_, Message> {
        let cards: Vec<Element<Message>> = items
            .iter()
            .take(3)
            .map(|item| self.view_detail_section_card(item))
            .collect();

        container(
            column![
                Self::bold_text(title, 18, TEXT_WHITE),
                Row::with_children(cards)
                    .spacing(16)
                    .align_y(iced::Alignment::Start)
            ]
            .spacing(20)
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .padding(Padding::new(32.0))
        .into()
    }

    fn card_style(shadow_alpha: f32, blur: f32) -> container::Style {
        container::Style {
            background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, shadow_alpha),
                offset: iced::Vector::new(0.0, if blur > 10.0 { 6.0 } else { 4.0 }),
                blur_radius: blur,
            },
            ..Default::default()
        }
    }

    fn gradient_overlay(strength: f32) -> Element<'static, Message> {
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, strength))
                        .add_stop(
                            if strength > 0.8 { 0.3 } else { 0.4 },
                            Color::from_rgba(0.0, 0.0, 0.0, strength * 0.3),
                        )
                        .add_stop(if strength > 0.8 { 0.5 } else { 0.6 }, Color::TRANSPARENT),
                ))),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn view_detail_section_card(&self, media_item: &MediaItem) -> Element<'_, Message> {
        let media_id = media_item.id;
        let (w, h) = (276.0, 155.0);

        if self.detail_hovered_card == Some(media_id) {
            return self.view_detail_section_expanded_card(media_item, w, h);
        }

        let backdrop = self.view_card_backdrop(media_item, w, h);
        let title_overlay = self.view_card_title_overlay(media_item, false);

        let card = container(iced::widget::stack![backdrop, title_overlay])
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .style(|_| Self::card_style(0.3, 8.0));

        iced::widget::mouse_area(card)
            .on_enter(Message::DetailHoverCard(Some(media_id)))
            .on_exit(Message::DetailHoverCard(None))
            .on_press(Message::OpenDetailPopup(media_id))
            .into()
    }

    fn view_card_title_overlay(
        &self,
        media_item: &MediaItem,
        expanded: bool,
    ) -> Element<'_, Message> {
        let title: Element<Message> =
            match self.get_cached_image(media_item.logo_path.as_ref(), ImageSize::Original) {
                Some(h) => iced::widget::image(h)
                    .width(Length::Fixed(if expanded { 100.0 } else { 120.0 }))
                    .content_fit(iced::ContentFit::Contain)
                    .into(),
                None => Self::bold_text(
                    &media_item.title,
                    if expanded { 13 } else { 14 },
                    TEXT_WHITE,
                )
                .wrapping(text::Wrapping::Word)
                .into(),
            };

        let content: Element<Message> = if expanded {
            let play_btn = self.play_button(media_item.id);
            let info_btn = self.info_button(media_item.id);
            column![
                title,
                row![play_btn, info_btn]
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
            ]
            .spacing(6)
            .padding(8.0)
            .into()
        } else {
            container(title).padding(10.0).into()
        };

        let content_container = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom);

        container(
            iced::widget::stack![
                Self::gradient_overlay(if expanded { 0.85 } else { 0.7 }),
                content_container
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| rounded_style(8.0, None))
        .into()
    }

    fn view_detail_section_expanded_card(
        &self,
        media_item: &MediaItem,
        w: f32,
        h: f32,
    ) -> Element<'_, Message> {
        let media_id = media_item.id;
        let backdrop = self.view_card_backdrop_with_video(media_item, w, h);
        let overlay = self.view_card_title_overlay(media_item, true);

        let card = container(iced::widget::stack![backdrop, overlay])
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .style(|_| Self::card_style(0.5, 12.0));

        iced::widget::mouse_area(card)
            .on_enter(Message::DetailHoverCard(Some(media_id)))
            .on_exit(Message::DetailHoverCard(None))
            .on_press(Message::OpenDetailPopup(media_id))
            .into()
    }

    fn view_card_backdrop(&self, media_item: &MediaItem, w: f32, h: f32) -> Element<'_, Message> {
        match self.get_cached_image(media_item.backdrop_path.as_ref(), ImageSize::Backdrop) {
            Some(h_img) => container(
                iced::widget::image(h_img)
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
                    .content_fit(iced::ContentFit::Cover),
            )
            .style(|_| rounded_style(8.0, None))
            .into(),
            None => container(icon(ICON_FILM).size(32).color(TEXT_GRAY))
                .width(Length::Fixed(w))
                .height(Length::Fixed(h))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| rounded_style(8.0, Some(SURFACE_DARK_GRAY)))
                .into(),
        }
    }

    fn view_card_backdrop_with_video(
        &self,
        media_item: &MediaItem,
        w: f32,
        h: f32,
    ) -> Element<'_, Message> {
        if let Some(ref frame) = self.detail_video_frame {
            if self.detail_player.current_media_id() == Some(media_item.id) {
                return container(
                    iced::widget::image(frame.clone())
                        .width(Length::Fixed(w))
                        .height(Length::Fixed(h))
                        .content_fit(iced::ContentFit::Cover),
                )
                .style(|_| rounded_style(8.0, None))
                .into();
            }
        }
        self.view_card_backdrop(media_item, w, h)
    }

    fn play_button(&self, media_id: u64) -> Element<'_, Message> {
        button(
            row![
                icon(ICON_PLAY_FILL).size(12).color(TEXT_WHITE),
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
                crate::media::NETFLIX_RED
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

    fn info_button(&self, media_id: u64) -> Element<'_, Message> {
        button(
            container(icon(ICON_INFO_CIRCLE).size(14).color(TEXT_WHITE))
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

    pub fn view_detail_advanced_info(
        &self,
        data: &crate::media::DetailPopupData,
    ) -> Element<'_, Message> {
        let mut sections: Vec<Element<'_, Message>> = vec![
            self.view_detail_social_links(&data.external_ids),
            self.view_detail_info_grid(data),
        ];
        if !data.keywords.is_empty() {
            sections.push(self.view_detail_keywords(&data.keywords));
        }
        if !data.production_companies.is_empty() {
            sections.push(self.view_detail_production_companies(&data.production_companies));
        }
        container(
            Column::with_children(sections)
                .spacing(28)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .padding(Padding::new(32.0))
        .style(|_| rounded_style(0.0, Some(Color::from_rgba(1.0, 1.0, 1.0, 0.03))))
        .into()
    }

    pub fn view_detail_social_links(&self, ids: &ExternalIds) -> Element<'_, Message> {
        let mut links: Vec<Element<'_, Message>> = Vec::new();
        if ids.imdb_id.is_some() {
            links.push(self.social_link_button("IMDB", None));
        }
        if ids.facebook_id.is_some() {
            links.push(self.social_link_button("Facebook", None));
        }
        if ids.twitter_id.is_some() {
            links.push(self.social_link_button("Twitter", None));
        }
        if ids.instagram_id.is_some() {
            links.push(self.social_link_button("Instagram", None));
        }
        if ids.homepage.is_some() {
            links.push(self.social_link_button("Homepage", Some(ICON_GLOBE)));
        }

        if links.is_empty() {
            return Space::new().width(0).height(0).into();
        }
        Row::with_children(links)
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn social_link_button(&self, label: &'static str, ic: Option<char>) -> Element<'_, Message> {
        let content: Element<Message> = match ic {
            Some(c) => row![
                icon(c).size(14).color(TEXT_WHITE),
                text(label).size(13).color(TEXT_WHITE)
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into(),
            None => text(label).size(13).color(TEXT_WHITE).into(),
        };
        button(content)
            .padding(Padding::new(8.0).left(16.0).right(16.0))
            .style(pill_button_style)
            .on_press(Message::HoverCard(None))
            .into()
    }

    fn view_detail_info_grid(&self, data: &crate::media::DetailPopupData) -> Element<'_, Message> {
        let media = &data.media_item;
        let mut items: Vec<(&'static str, String)> = Vec::new();

        if let Some(ref status) = media.status {
            items.push(("Status", status.clone()));
        }
        if let Some(ref lang) = media.original_language {
            items.push(("Original Language", lang.to_uppercase()));
        }
        if let Some(b) = media.budget.filter(|&b| b > 0) {
            items.push(("Budget", crate::detail_popup::format_currency(b)));
        }
        if let Some(r) = media.revenue.filter(|&r| r > 0) {
            items.push(("Revenue", crate::detail_popup::format_currency(r)));
        }

        if items.is_empty() {
            return Space::new().width(0).height(0).into();
        }

        let mut rows: Vec<Element<Message>> = Vec::new();
        let mut current_row: Vec<Element<Message>> = Vec::new();
        for (label, value) in items {
            current_row.push(
                column![
                    text(label).size(12).color(TEXT_GRAY),
                    text(value).size(14).color(TEXT_WHITE)
                ]
                .spacing(4)
                .width(Length::Fixed(180.0))
                .into(),
            );
            if current_row.len() == 4 {
                rows.push(
                    Row::with_children(std::mem::take(&mut current_row))
                        .spacing(24)
                        .align_y(iced::Alignment::Start)
                        .into(),
                );
            }
        }
        if !current_row.is_empty() {
            rows.push(
                Row::with_children(current_row)
                    .spacing(24)
                    .align_y(iced::Alignment::Start)
                    .into(),
            );
        }

        Column::with_children(rows).spacing(16).into()
    }

    pub fn view_detail_keywords(&self, keywords: &[Keyword]) -> Element<'_, Message> {
        let pills: Vec<Element<Message>> = keywords
            .iter()
            .take(15)
            .map(|kw| {
                container(text(kw.name.clone()).size(12).color(TEXT_WHITE))
                    .padding(Padding::new(6.0).left(12.0).right(12.0))
                    .style(|_| rounded_style(12.0, Some(Color::from_rgb(0.2, 0.2, 0.2))))
                    .into()
            })
            .collect();

        column![
            Self::bold_text("Keywords", 14, TEXT_GRAY),
            Self::horizontal_scroll(
                Row::with_children(pills)
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
            )
        ]
        .spacing(12)
        .into()
    }

    pub fn view_detail_production_companies(
        &self,
        companies: &[ProductionCompany],
    ) -> Element<'_, Message> {
        let items: Vec<Element<Message>> = companies
            .iter()
            .take(6)
            .map(|c| {
                let logo: Element<Message> =
                    match self.get_cached_image(c.logo_path.as_ref(), ImageSize::Original) {
                        Some(h) => container(
                            iced::widget::image(h)
                                .height(Length::Fixed(40.0))
                                .content_fit(iced::ContentFit::Contain),
                        )
                        .height(Length::Fixed(40.0))
                        .into(),
                        None => container(
                            text(c.name.chars().next().unwrap_or('?'))
                                .size(20)
                                .color(TEXT_GRAY),
                        )
                        .width(Length::Fixed(60.0))
                        .height(Length::Fixed(40.0))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .style(|_| rounded_style(4.0, Some(Color::from_rgba(0.2, 0.2, 0.2, 0.5))))
                        .into(),
                    };
                column![logo, text(c.name.clone()).size(11).color(TEXT_GRAY)]
                    .spacing(6)
                    .align_x(iced::Alignment::Center)
                    .into()
            })
            .collect();

        column![
            Self::bold_text("Production Companies", 14, TEXT_GRAY),
            Row::with_children(items)
                .spacing(24)
                .align_y(iced::Alignment::Center)
        ]
        .spacing(12)
        .into()
    }
}
