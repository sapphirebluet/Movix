use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Column, Row, Space,
};
use iced::{Border, Color, Element, Length, Padding, Shadow};

use crate::media::{
    LoadingState, Message, NavItem, Page, ProfileAction, NETFLIX_RED, SURFACE_DARK_GRAY, TEXT_GRAY,
    TEXT_WHITE,
};
use crate::Movix;

const ICON_PERSON_FILL: char = '\u{F4DA}';
const ICON_SEARCH: char = '\u{F52A}';

fn icon(icon_char: char) -> iced::widget::Text<'static> {
    text(icon_char.to_string()).font(iced::Font {
        family: iced::font::Family::Name("bootstrap-icons"),
        ..Default::default()
    })
}

pub fn hidden_vertical_scrollbar_style(
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
    pub fn view_header(&self) -> Element<'_, Message> {
        let logo = self.view_logo();
        let navigation = self.view_navigation();
        let search_bar = self.view_search_bar();
        let profile_picker = self.view_profile_picker();

        let left_section = row![logo, navigation]
            .spacing(32)
            .align_y(iced::Alignment::Center);

        let right_section = row![search_bar, profile_picker]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        let header_content = row![
            left_section,
            Space::new().width(Length::Fill),
            right_section
        ]
        .padding(Padding::new(16.0).left(48.0).right(48.0))
        .align_y(iced::Alignment::Center);

        let scroll_offset = self.main_scroll_offset;
        let is_scrolled = scroll_offset > 0.0;

        container(header_content)
            .width(Length::Fill)
            .height(Length::Fixed(80.0))
            .style(move |_theme| {
                if !is_scrolled {
                    container::Style::default()
                } else {
                    container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            0.0, 0.0, 0.0, 0.5,
                        ))),
                        ..Default::default()
                    }
                }
            })
            .into()
    }

    pub fn view_logo(&self) -> Element<'_, Message> {
        text("Movix")
            .size(28)
            .color(TEXT_WHITE)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .into()
    }

    pub fn view_navigation(&self) -> Element<'_, Message> {
        let nav_items = [
            (NavItem::Series, "Series", Page::Series),
            (NavItem::Movies, "Movies", Page::Movies),
            (NavItem::MyList, "My List", Page::MyList),
        ];

        let nav_buttons: Vec<Element<Message>> = nav_items
            .into_iter()
            .map(|(nav_item, label, page)| {
                self.view_nav_button(nav_item, String::from(label), page)
            })
            .collect();

        Row::with_children(nav_buttons)
            .spacing(24)
            .align_y(iced::Alignment::Center)
            .into()
    }

    pub fn view_nav_button(
        &self,
        nav_item: NavItem,
        label: String,
        page: Page,
    ) -> Element<'_, Message> {
        let is_active = self.header_state.active_nav == nav_item;
        let text_color = if is_active { TEXT_WHITE } else { TEXT_GRAY };

        let button_content: Element<Message> = if is_active {
            let label_text = text(label)
                .size(14)
                .color(text_color)
                .shaping(text::Shaping::Advanced);
            let underline = container(Space::new().width(Length::Fill).height(2)).style(|_theme| {
                container::Style {
                    background: Some(iced::Background::Color(NETFLIX_RED)),
                    ..Default::default()
                }
            });
            column![label_text, underline]
                .spacing(4)
                .align_x(iced::Alignment::Center)
                .into()
        } else {
            text(label)
                .size(14)
                .color(text_color)
                .shaping(text::Shaping::Advanced)
                .into()
        };

        button(button_content)
            .padding(Padding::new(8.0).left(12.0).right(12.0))
            .style(move |_theme, status| {
                let final_color = match status {
                    button::Status::Hovered => TEXT_WHITE,
                    _ if is_active => TEXT_WHITE,
                    _ => TEXT_GRAY,
                };
                button::Style {
                    background: Some(iced::Background::Color(Color::TRANSPARENT)),
                    text_color: final_color,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: false,
                }
            })
            .on_press(Message::NavigateTo(page))
            .into()
    }

    pub fn view_search_bar(&self) -> Element<'_, Message> {
        let search_icon = icon(ICON_SEARCH).size(14).color(TEXT_GRAY);

        let search_input = text_input("Search...", &self.search_query)
            .on_input(Message::SearchQueryChanged)
            .on_submit(Message::SearchSubmit)
            .padding(8)
            .width(Length::Fixed(160.0))
            .style(|_theme, _status| text_input::Style {
                background: iced::Background::Color(Color::TRANSPARENT),
                border: Border::default(),
                icon: TEXT_GRAY,
                placeholder: TEXT_GRAY,
                value: TEXT_WHITE,
                selection: NETFLIX_RED,
            });

        let search_content = row![search_icon, search_input]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        container(search_content)
            .padding(Padding::new(4.0).left(12.0).right(8.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, 0.7,
                ))),
                border: Border {
                    color: TEXT_GRAY,
                    width: 1.0,
                    radius: 24.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    pub fn view_profile_picker(&self) -> Element<'_, Message> {
        let profile_icon = container(icon(ICON_PERSON_FILL).size(18).color(TEXT_WHITE))
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        button(profile_icon)
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .padding(0)
            .style(|_theme, _status| button::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                text_color: TEXT_WHITE,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 20.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            })
            .on_press(Message::ToggleProfileMenu)
            .into()
    }

    pub fn view_profile_dropdown(&self) -> Element<'_, Message> {
        let menu_items = [
            ("Settings", ProfileAction::OpenSettings),
            ("Profile Settings", ProfileAction::OpenProfileSettings),
            ("Switch Profile", ProfileAction::SwitchProfile(0)),
        ];

        let menu_buttons: Vec<Element<Message>> = menu_items
            .into_iter()
            .map(|(label, action)| {
                button(text(label).size(14).color(TEXT_WHITE))
                    .padding(Padding::new(12.0).left(16.0).right(16.0))
                    .width(Length::Fill)
                    .style(|_theme, status| {
                        let background_color = match status {
                            button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(iced::Background::Color(background_color)),
                            text_color: TEXT_WHITE,
                            border: Border::default(),
                            shadow: Shadow::default(),
                            snap: false,
                        }
                    })
                    .on_press(Message::ProfileAction(action))
                    .into()
            })
            .collect();

        container(Column::with_children(menu_buttons))
            .width(Length::Fixed(160.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 8.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 8.0,
                },
                ..Default::default()
            })
            .into()
    }

    pub fn view_header_with_dropdown(&self) -> Element<'_, Message> {
        container(self.view_header())
            .width(Length::Fill)
            .height(Length::Fixed(80.0))
            .into()
    }

    pub fn view_main_content(&self) -> Element<'_, Message> {
        match &self.loading_state {
            LoadingState::Loading => self.view_skeleton_ui(),
            LoadingState::Error(error_message) => self.view_error_state(error_message),
            LoadingState::Idle => self.view_idle_state(),
        }
    }

    fn view_error_state<'a>(&'a self, error_message: &'a str) -> Element<'a, Message> {
        let error_text = text(error_message).size(18).color(NETFLIX_RED);
        let retry_button = button(text("Retry").size(16).color(TEXT_WHITE))
            .padding(Padding::new(12.0).left(24.0).right(24.0))
            .style(|_theme, _status| button::Style {
                background: Some(iced::Background::Color(NETFLIX_RED)),
                text_color: TEXT_WHITE,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            })
            .on_press(Message::RetryLoad);

        container(
            column![error_text, retry_button]
                .spacing(16)
                .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn view_idle_state(&self) -> Element<'_, Message> {
        let header = self.view_header_with_dropdown();

        let main_column = if self.search_active {
            column![self.view_search_page()].width(Length::Fill)
        } else {
            let hero = self.view_hero_section();
            let content_sections = self.view_content_sections();
            column![hero, content_sections].width(Length::Fill)
        };

        let base_content = iced::widget::stack![
            scrollable(main_column)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(0).scroller_width(0),
                ))
                .on_scroll(|viewport| Message::MainScrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(hidden_vertical_scrollbar_style),
            header
        ];

        if self.profile_menu_open {
            let dropdown = self.view_profile_dropdown();
            let dropdown_positioned = container(dropdown)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .padding(Padding::new(0.0).top(80.0).right(24.0));

            iced::widget::stack![base_content, dropdown_positioned]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            base_content.width(Length::Fill).height(Length::Fill).into()
        }
    }
}

impl Movix {
    pub fn view_skeleton_ui(&self) -> Element<'_, Message> {
        let skeleton_header = self.view_skeleton_header();
        let skeleton_hero = self.view_skeleton_hero();
        let skeleton_sections = self.view_skeleton_sections();

        scrollable(
            column![skeleton_header, skeleton_hero, skeleton_sections]
                .spacing(0)
                .width(Length::Fill),
        )
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(0).scroller_width(0),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(hidden_vertical_scrollbar_style)
        .into()
    }

    pub fn view_skeleton_header(&self) -> Element<'_, Message> {
        let logo_skeleton =
            container(Space::new().width(80.0).height(28.0)).style(skeleton_style(4.0));

        let nav_skeletons: Vec<Element<Message>> = (0..5)
            .map(|_| {
                container(Space::new().width(60.0).height(14.0))
                    .style(skeleton_style(4.0))
                    .into()
            })
            .collect();

        let nav_row = Row::with_children(nav_skeletons)
            .spacing(16)
            .align_y(iced::Alignment::Center);

        let left_section = row![logo_skeleton, nav_row]
            .spacing(32)
            .align_y(iced::Alignment::Center);

        let search_skeleton =
            container(Space::new().width(200.0).height(36.0)).style(skeleton_style(24.0));
        let profile_skeleton =
            container(Space::new().width(40.0).height(40.0)).style(skeleton_style(20.0));

        let right_section = row![search_skeleton, profile_skeleton]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        let header_content = row![
            left_section,
            Space::new().width(Length::Fill),
            right_section
        ]
        .padding(Padding::new(16.0).left(48.0).right(48.0))
        .align_y(iced::Alignment::Center);

        container(header_content)
            .width(Length::Fill)
            .height(Length::Fixed(80.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(std::f32::consts::PI)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.95))
                        .add_stop(0.4, Color::from_rgba(0.0, 0.0, 0.0, 0.85))
                        .add_stop(0.7, Color::from_rgba(0.0, 0.0, 0.0, 0.5))
                        .add_stop(1.0, Color::from_rgba(0.0, 0.0, 0.0, 0.2)),
                ))),
                ..Default::default()
            })
            .into()
    }

    pub fn view_skeleton_hero(&self) -> Element<'_, Message> {
        let title_skeleton =
            container(Space::new().width(300.0).height(48.0)).style(skeleton_style_alpha(4.0, 0.6));
        let desc_line_one =
            container(Space::new().width(400.0).height(16.0)).style(skeleton_style_alpha(4.0, 0.4));
        let desc_line_two =
            container(Space::new().width(350.0).height(16.0)).style(skeleton_style_alpha(4.0, 0.4));
        let button_skeleton_one =
            container(Space::new().width(100.0).height(44.0)).style(skeleton_style(4.0));
        let button_skeleton_two =
            container(Space::new().width(120.0).height(44.0)).style(skeleton_style(4.0));

        let button_row = row![button_skeleton_one, button_skeleton_two]
            .spacing(12)
            .align_y(iced::Alignment::Center);

        let hero_text_content = column![title_skeleton, desc_line_one, desc_line_two, button_row]
            .spacing(16)
            .max_width(500.0)
            .padding(Padding::new(48.0).left(48.0));

        let hero_overlay = container(hero_text_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.95))
                        .add_stop(0.35, Color::from_rgba(0.0, 0.0, 0.0, 0.8))
                        .add_stop(0.55, Color::from_rgba(0.0, 0.0, 0.0, 0.3))
                        .add_stop(0.75, Color::from_rgba(0.0, 0.0, 0.0, 0.0)),
                ))),
                ..Default::default()
            });

        let backdrop_skeleton = container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(500.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE_DARK_GRAY)),
                ..Default::default()
            });

        iced::widget::stack![backdrop_skeleton, hero_overlay]
            .width(Length::Fill)
            .height(Length::Fixed(500.0))
            .into()
    }

    pub fn view_skeleton_sections(&self) -> Element<'_, Message> {
        let sections: Vec<Element<Message>> =
            (0..4).map(|_| self.view_skeleton_section()).collect();

        Column::with_children(sections)
            .spacing(24)
            .padding(Padding::new(24.0).left(24.0).right(24.0))
            .width(Length::Fill)
            .into()
    }

    pub fn view_skeleton_section(&self) -> Element<'_, Message> {
        let title_skeleton =
            container(Space::new().width(150.0).height(24.0)).style(skeleton_style_alpha(4.0, 0.6));

        let card_skeletons: Vec<Element<Message>> =
            (0..6).map(|_| self.view_skeleton_card()).collect();

        let cards_row = Row::with_children(card_skeletons)
            .spacing(12)
            .align_y(iced::Alignment::Start);

        column![title_skeleton, cards_row]
            .spacing(16)
            .width(Length::Fill)
            .into()
    }

    pub fn view_skeleton_card(&self) -> Element<'_, Message> {
        container(Space::new().width(150.0).height(225.0))
            .width(Length::Fixed(150.0))
            .height(Length::Fixed(225.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.2, 0.2, 0.2, 0.5,
                ))),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

fn skeleton_style(radius: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.2, 0.2, 0.2, 0.5,
        ))),
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn skeleton_style_alpha(radius: f32, alpha: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.2, 0.2, 0.2, alpha,
        ))),
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
