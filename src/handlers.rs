use iced::Task;

use crate::detail_handlers;
use crate::media::{
    section_id, ApiError, Genre, LoadingState, MediaId, MediaTypeFilter, Message, NavItem, Page,
    ScrollDirection, SearchFilters, SortOption,
};
use crate::player_handlers;
use crate::tmdb::{fetch_image_bytes, load_hero_content, load_initial_content, ImageSize};
use crate::video::{select_best_trailer, TrailerVideo};
use crate::Movix;

pub fn handle_message(app: &mut Movix, message: Message) -> Task<Message> {
    match message {
        Message::Setup(_) => Task::none(),
        Message::NavigateTo(page) => handle_navigate(app, page),
        Message::SearchQueryChanged(query) => handle_search_query_changed(app, query),
        Message::SearchSubmit => handle_search_submit(app),
        Message::SearchResultsLoaded(result) => handle_search_results(app, result),
        Message::ToggleProfileMenu => {
            app.profile_menu_open = !app.profile_menu_open;
            Task::none()
        }
        Message::CloseProfileMenu => {
            app.profile_menu_open = false;
            Task::none()
        }
        Message::ProfileAction(_) => {
            app.profile_menu_open = false;
            Task::none()
        }
        Message::PlayContent(id) => player_handlers::handle_play_content(app, id),
        Message::ShowMoreInfo(id) => Task::done(Message::OpenDetailPopup(id)),
        Message::HoverCard(id) => handle_hover_card(app, id),
        Message::HoverCardDelayed(media_id) => handle_hover_card_delayed(app, media_id),
        Message::HoverSection(idx) => {
            if app.detail_popup_open || app.movie_player_active {
                return Task::none();
            }
            app.hovered_section = idx;
            Task::none()
        }
        Message::ContentLoaded(result) => handle_content_loaded(app, result),
        Message::HeroLoaded(result) => handle_hero_loaded(app, result),
        Message::ImageLoaded(url, result) => handle_image_loaded(app, url, result),
        Message::LogoLoaded(media_id, result) => handle_logo_loaded(app, media_id, result),
        Message::LoadImage(url) => handle_load_image(app, url),
        Message::RetryLoad => handle_retry_load(app),
        Message::ScrollSection(idx, dir) => handle_scroll_section(app, idx, dir),
        Message::AnimateScroll(idx) => handle_animate_scroll(app, idx),
        Message::SectionScrolled(idx, offset) => handle_section_scrolled(app, idx, offset),
        Message::TrailerVideosLoaded(id, result) => handle_trailer_videos_loaded(app, id, result),
        Message::TrailerStreamUrlPreloaded(id, result) => {
            if let Ok(url) = result {
                app.stream_url_cache.insert(id, url);
            }
            Task::none()
        }
        Message::TrailerStreamUrlLoaded(id, result) => {
            player_handlers::handle_trailer_stream_url_loaded(app, id, result)
        }
        Message::HeroFrameTick => player_handlers::handle_hero_frame_tick(app),
        Message::CardFrameTick => player_handlers::handle_card_frame_tick(app),
        Message::StopCardTrailer => player_handlers::handle_stop_card_trailer(app),
        Message::PlayCardTrailer(id) => player_handlers::handle_play_card_trailer(app, id),
        Message::PlayHeroTrailer(id) => player_handlers::handle_play_hero_trailer(app, id),
        Message::PlayDetailTrailer(id) => player_handlers::handle_play_detail_trailer(app, id),
        Message::PauseHeroTrailer => player_handlers::handle_pause_hero_trailer(app),
        Message::ResumeHeroTrailer => player_handlers::handle_resume_hero_trailer(app),
        Message::HeroVisibilityChanged(visible) => handle_hero_visibility(app, visible),
        Message::MainScrolled(offset) => handle_main_scrolled(app, offset),
        Message::ToggleHeroMute => player_handlers::handle_toggle_hero_mute(app),
        Message::ReplayHeroTrailer => player_handlers::handle_replay_hero_trailer(app),
        Message::HeroVideoEnded => {
            app.hero_ended = true;
            Task::none()
        }
        Message::MoviePlayerOpen(id, title) => {
            player_handlers::handle_movie_player_open(app, id, title)
        }
        Message::MoviePlayerStreamResolved(id, result) => {
            player_handlers::handle_movie_stream_resolved(app, id, result)
        }
        Message::MoviePlayerClose => player_handlers::handle_movie_player_close(app),
        Message::MoviePlayerTogglePlay => player_handlers::handle_movie_toggle_play(app),
        Message::MoviePlayerSeek(pos) => player_handlers::handle_movie_seek(app, pos),
        Message::MoviePlayerSeekRelative(delta) => {
            player_handlers::handle_movie_seek_relative(app, delta)
        }
        Message::MoviePlayerSetVolume(vol) => player_handlers::handle_movie_set_volume(app, vol),
        Message::MoviePlayerToggleMute => player_handlers::handle_movie_toggle_mute(app),
        Message::MoviePlayerToggleFullscreen => Task::none(),
        Message::MoviePlayerFrameTick => {
            player_handlers::handle_movie_frame_tick(app);
            Task::none()
        }
        Message::MoviePlayerShowControls => player_handlers::handle_movie_show_controls(app),
        Message::MoviePlayerHideControls => player_handlers::handle_movie_hide_controls(app),
        Message::OpenDetailPopup(id) => detail_handlers::handle_open_detail_popup(app, id),
        Message::CloseDetailPopup => detail_handlers::handle_close_detail_popup(app),
        Message::DetailDataLoaded(result) => {
            detail_handlers::handle_detail_data_loaded(app, result)
        }
        Message::DetailSelectSeason(season) => {
            detail_handlers::handle_detail_select_season(app, season)
        }
        Message::DetailEpisodesLoaded(result) => {
            detail_handlers::handle_detail_episodes_loaded(app, result)
        }
        Message::DetailHoverCard(id) => detail_handlers::handle_detail_hover_card(app, id),
        Message::DetailHoverCardDelayed(media_id) => {
            detail_handlers::handle_detail_hover_card_delayed(app, media_id)
        }
        Message::DetailFrameTick => detail_handlers::handle_detail_frame_tick(app),
        Message::DetailTrailerLoaded(id, result) => {
            detail_handlers::handle_detail_trailer_loaded(app, id, result)
        }
        Message::SearchDebounceTriggered => handle_search_debounce_triggered(app),
        Message::ClearSearch => handle_clear_search(app),
        Message::SetMediaTypeFilter(filter) => handle_set_media_type_filter(app, filter),
        Message::SetGenreFilter(genre_id) => handle_set_genre_filter(app, genre_id),
        Message::SetYearFrom(year) => handle_set_year_from(app, year),
        Message::SetYearTo(year) => handle_set_year_to(app, year),
        Message::SetMinRating(rating) => handle_set_min_rating(app, rating),
        Message::SetSortOption(sort) => handle_set_sort_option(app, sort),
        Message::ResetFilters => handle_reset_filters(app),
        Message::GenresLoaded(result) => handle_genres_loaded(app, result),
    }
}

fn handle_navigate(app: &mut Movix, page: Page) -> Task<Message> {
    app.current_page = page.clone();
    app.profile_menu_open = false;
    app.header_state.active_nav = match page {
        Page::Home => NavItem::Home,
        Page::Series => NavItem::Series,
        Page::Movies => NavItem::Movies,
        Page::MostRecent => NavItem::MostRecent,
        Page::MyList => NavItem::MyList,
        Page::Detail(_) => app.header_state.active_nav.clone(),
    };
    Task::none()
}

fn handle_search_query_changed(app: &mut Movix, query: String) -> Task<Message> {
    app.search_query = query.clone();

    if query.trim().is_empty() {
        return Task::done(Message::ClearSearch);
    }

    app.search_active = true;
    app.search_debounce_timer = Some(std::time::Instant::now());
    Task::none()
}

fn handle_search_debounce_triggered(app: &mut Movix) -> Task<Message> {
    let Some(timer) = app.search_debounce_timer else {
        return Task::none();
    };

    if timer.elapsed() < std::time::Duration::from_millis(300) {
        return Task::none();
    }

    app.search_debounce_timer = None;

    if app.search_query.trim().is_empty() {
        return Task::done(Message::ClearSearch);
    }

    let Some(client) = &app.tmdb_client else {
        return Task::none();
    };

    let search_client = client.clone();
    let query = app.search_query.clone();
    Task::perform(
        async move { search_client.search(&query).await },
        Message::SearchResultsLoaded,
    )
}

fn handle_search_submit(app: &mut Movix) -> Task<Message> {
    if app.search_query.is_empty() {
        return Task::none();
    }
    let Some(client) = &app.tmdb_client else {
        return Task::none();
    };
    let search_client = client.clone();
    let query = app.search_query.clone();
    Task::perform(
        async move { search_client.search(&query).await },
        Message::SearchResultsLoaded,
    )
}

fn handle_search_results(
    app: &mut Movix,
    result: Result<Vec<crate::media::MediaItem>, ApiError>,
) -> Task<Message> {
    match result {
        Ok(items) => {
            app.search_results = items.clone();
            app.filtered_results = app.search_filters.apply(&app.search_results);
            load_search_result_images(app, &items)
        }
        Err(error) => {
            app.error_message = Some(format!("{:?}", error));
            Task::none()
        }
    }
}

fn load_search_result_images(app: &Movix, items: &[crate::media::MediaItem]) -> Task<Message> {
    let Some(client) = &app.tmdb_client else {
        return Task::none();
    };

    let mut tasks = Vec::new();

    for item in items.iter().take(20) {
        if let Some(backdrop_path) = &item.backdrop_path {
            let url = client.image_url(backdrop_path, ImageSize::Backdrop);
            if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }

        if item.logo_path.is_none() {
            let fetch_client = client.clone();
            let media_id = item.id;
            let media_type = item.media_type.clone();
            tasks.push(Task::perform(
                async move { fetch_client.fetch_media_images(media_id, &media_type).await },
                move |result| Message::LogoLoaded(media_id, result),
            ));
        } else if let Some(logo_path) = &item.logo_path {
            let url = client.image_url(logo_path, ImageSize::Original);
            if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }

        if !app.trailer_cache.contains_key(&item.id) {
            let fetch_client = client.clone();
            let media_id = item.id;
            let media_type = item.media_type.clone();
            tasks.push(Task::perform(
                async move { fetch_client.fetch_videos(media_id, &media_type).await },
                move |result| Message::TrailerVideosLoaded(media_id, result),
            ));
        }
    }

    Task::batch(tasks)
}

fn handle_hover_card(app: &mut Movix, id: Option<MediaId>) -> Task<Message> {
    if app.detail_popup_open || app.movie_player_active {
        return Task::none();
    }
    match id {
        Some(media_id) => {
            app.pending_hover_card = Some(media_id);
            Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                },
                move |_| Message::HoverCardDelayed(media_id),
            )
        }
        None => {
            app.pending_hover_card = None;
            let prev_hovered = app.hovered_card.take();
            if prev_hovered.is_some() {
                let stop_card = Task::done(Message::StopCardTrailer);
                if app.hero_visible {
                    return Task::batch([stop_card, Task::done(Message::ResumeHeroTrailer)]);
                }
                return stop_card;
            }
            Task::none()
        }
    }
}

fn handle_hover_card_delayed(app: &mut Movix, media_id: MediaId) -> Task<Message> {
    if app.detail_popup_open || app.movie_player_active {
        return Task::none();
    }
    if app.pending_hover_card != Some(media_id) {
        return Task::none();
    }
    app.hovered_card = Some(media_id);
    let image_task = app.load_hover_card_images(media_id);
    let trailer_task = app.load_trailer_for_hovered_card(media_id);
    Task::batch([image_task, trailer_task])
}

fn handle_content_loaded(
    app: &mut Movix,
    result: Result<Vec<crate::media::ContentSection>, ApiError>,
) -> Task<Message> {
    match result {
        Ok(sections) => {
            app.content_sections = sections.clone();
            app.loading_state = LoadingState::Idle;
            let image_task = app.load_content_images(&sections);
            let preload_task = app.preload_trailer_urls(&sections);
            Task::batch([image_task, preload_task])
        }
        Err(error) => {
            app.loading_state = LoadingState::Error(format!("{:?}", error));
            app.error_message = Some(format!("{:?}", error));
            Task::none()
        }
    }
}

fn handle_hero_loaded(
    app: &mut Movix,
    result: Box<Result<crate::media::MediaItem, ApiError>>,
) -> Task<Message> {
    match *result {
        Ok(item) => {
            app.hero_content = Some(item.clone());
            let image_task = app.load_hero_images(&item);
            let trailer_task = app.load_trailer_for_media(item.id, &item.media_type);
            Task::batch([image_task, trailer_task])
        }
        Err(error) => {
            app.error_message = Some(format!("{:?}", error));
            Task::none()
        }
    }
}

fn handle_image_loaded(
    app: &mut Movix,
    url: String,
    result: Result<iced::widget::image::Handle, String>,
) -> Task<Message> {
    if let Ok(handle) = result {
        app.image_cache.insert(url, handle);
    }
    Task::none()
}

fn handle_logo_loaded(
    app: &mut Movix,
    media_id: MediaId,
    result: Result<Option<String>, ApiError>,
) -> Task<Message> {
    let Ok(Some(logo_path)) = result else {
        return Task::none();
    };
    for section in &mut app.content_sections {
        for item in &mut section.items {
            if item.id == media_id {
                item.logo_path = Some(logo_path.clone());
            }
        }
    }
    let Some(client) = &app.tmdb_client else {
        return Task::none();
    };
    let url = client.image_url(&logo_path, ImageSize::Original);
    if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
        return Task::done(Message::LoadImage(url));
    }
    Task::none()
}

fn handle_load_image(app: &mut Movix, url: String) -> Task<Message> {
    if app.image_cache.get(&url).is_some() || app.image_cache.is_pending(&url) {
        return Task::none();
    }
    app.image_cache.mark_pending(url.clone());
    let image_url = url.clone();
    let cache_path = app.image_cache.get_cache_path(&url);

    Task::perform(
        async move {
            if let Some(ref path) = cache_path {
                if path.exists() {
                    if let Ok(bytes) = tokio::fs::read(path).await {
                        return (image_url, Ok(bytes), cache_path, true);
                    }
                }
            }
            let bytes = fetch_image_bytes(image_url.clone()).await;
            (image_url, bytes, cache_path, false)
        },
        |(url, result, cache_path, from_cache)| match result {
            Ok(bytes) => {
                if !from_cache {
                    if let Some(path) = cache_path {
                        let bytes_clone = bytes.clone();
                        std::thread::spawn(move || {
                            let _ = std::fs::write(path, &bytes_clone);
                        });
                    }
                }
                Message::ImageLoaded(url, Ok(iced::widget::image::Handle::from_bytes(bytes)))
            }
            Err(error) => Message::ImageLoaded(url, Err(error)),
        },
    )
}

fn handle_retry_load(app: &mut Movix) -> Task<Message> {
    app.loading_state = LoadingState::Loading;
    app.error_message = None;
    let Some(client) = &app.tmdb_client else {
        return Task::none();
    };

    let content_client = client.clone();
    let hero_client = client.clone();
    Task::batch([
        Task::perform(load_initial_content(content_client), Message::ContentLoaded),
        Task::perform(load_hero_content(hero_client), |r| {
            Message::HeroLoaded(Box::new(r))
        }),
    ])
}

fn handle_scroll_section(
    app: &mut Movix,
    section_index: usize,
    direction: ScrollDirection,
) -> Task<Message> {
    let scroll_amount = 500.0;
    while app.section_scroll_offsets.len() <= section_index {
        app.section_scroll_offsets.push(0.0);
    }
    while app.section_scroll_targets.len() <= section_index {
        app.section_scroll_targets.push(0.0);
    }

    let current_target = app.section_scroll_targets[section_index];
    let new_target = match direction {
        ScrollDirection::Left => (current_target - scroll_amount).max(0.0),
        ScrollDirection::Right => current_target + scroll_amount,
    };
    app.section_scroll_targets[section_index] = new_target;
    Task::done(Message::AnimateScroll(section_index))
}

fn handle_animate_scroll(app: &mut Movix, section_index: usize) -> Task<Message> {
    if section_index >= app.section_scroll_offsets.len()
        || section_index >= app.section_scroll_targets.len()
    {
        return Task::none();
    }

    let current = app.section_scroll_offsets[section_index];
    let target = app.section_scroll_targets[section_index];
    let diff = target - current;

    if diff.abs() < 1.0 {
        app.section_scroll_offsets[section_index] = target;
        let Some(section_id_str) = section_id(section_index) else {
            return Task::none();
        };
        let id = iced::widget::Id::new(section_id_str);
        let offset = iced::widget::scrollable::AbsoluteOffset { x: target, y: 0.0 };
        return iced::widget::operation::scroll_to(id, offset);
    }

    let new_offset = current + diff * 0.15;
    app.section_scroll_offsets[section_index] = new_offset;

    let Some(section_id_str) = section_id(section_index) else {
        return Task::none();
    };
    let id = iced::widget::Id::new(section_id_str);
    let offset = iced::widget::scrollable::AbsoluteOffset {
        x: new_offset,
        y: 0.0,
    };

    Task::batch([
        iced::widget::operation::scroll_to(id, offset),
        Task::perform(
            async { tokio::time::sleep(std::time::Duration::from_millis(16)).await },
            move |_| Message::AnimateScroll(section_index),
        ),
    ])
}

fn handle_section_scrolled(app: &mut Movix, section_index: usize, offset: f32) -> Task<Message> {
    while app.section_scroll_offsets.len() <= section_index {
        app.section_scroll_offsets.push(0.0);
    }
    app.section_scroll_offsets[section_index] = offset;
    app.load_visible_images(section_index, offset)
}

fn handle_trailer_videos_loaded(
    app: &mut Movix,
    media_id: MediaId,
    result: Result<Vec<TrailerVideo>, ApiError>,
) -> Task<Message> {
    match result {
        Ok(videos) => {
            if let Some(trailer) = select_best_trailer(&videos) {
                let youtube_id = trailer.key.clone();
                app.trailer_cache.insert(media_id, Some(youtube_id.clone()));

                let is_hero = app.hero_content.as_ref().map(|h| h.id) == Some(media_id);
                let is_hovered = app.hovered_card == Some(media_id);
                let is_detail_hovered = app.detail_hovered_card == Some(media_id);

                if is_hero || is_hovered || is_detail_hovered {
                    return app.fetch_trailer_stream_url(media_id, youtube_id);
                }

                let manager = app.trailer_manager.clone();
                return Task::perform(
                    async move { manager.get_stream_url(&youtube_id).await },
                    move |result| Message::TrailerStreamUrlPreloaded(media_id, result),
                );
            }
            app.trailer_cache.insert(media_id, None);
        }
        Err(_) => {
            app.trailer_cache.insert(media_id, None);
        }
    }
    Task::none()
}

fn handle_hero_visibility(app: &mut Movix, visible: bool) -> Task<Message> {
    app.hero_visible = visible;
    if app.movie_player_active {
        return Task::none();
    }
    if !visible {
        return Task::done(Message::PauseHeroTrailer);
    }
    Task::done(Message::ResumeHeroTrailer)
}

fn handle_main_scrolled(app: &mut Movix, offset: f32) -> Task<Message> {
    app.main_scroll_offset = offset;
    let hero_height = 620.0;
    let was_visible = app.hero_visible;
    app.hero_visible = offset < hero_height * 0.5;

    if app.movie_player_active {
        return Task::none();
    }
    if was_visible && !app.hero_visible {
        return Task::done(Message::PauseHeroTrailer);
    }
    if !was_visible && app.hero_visible && app.hovered_card.is_none() {
        return Task::done(Message::ResumeHeroTrailer);
    }
    Task::none()
}

fn handle_clear_search(app: &mut Movix) -> Task<Message> {
    app.search_active = false;
    app.search_query.clear();
    app.search_results.clear();
    app.filtered_results.clear();
    app.search_filters = SearchFilters::default();
    app.search_debounce_timer = None;
    Task::none()
}

fn handle_set_media_type_filter(app: &mut Movix, filter: MediaTypeFilter) -> Task<Message> {
    app.search_filters.media_type = filter;
    app.filtered_results = app.search_filters.apply(&app.search_results);
    Task::none()
}

fn handle_set_genre_filter(app: &mut Movix, genre_id: Option<u64>) -> Task<Message> {
    app.search_filters.genre_id = genre_id;
    app.filtered_results = app.search_filters.apply(&app.search_results);
    Task::none()
}

fn handle_set_year_from(app: &mut Movix, year: Option<u32>) -> Task<Message> {
    app.search_filters.year_from = year;
    app.filtered_results = app.search_filters.apply(&app.search_results);
    Task::none()
}

fn handle_set_year_to(app: &mut Movix, year: Option<u32>) -> Task<Message> {
    app.search_filters.year_to = year;
    app.filtered_results = app.search_filters.apply(&app.search_results);
    Task::none()
}

fn handle_set_min_rating(app: &mut Movix, rating: f32) -> Task<Message> {
    app.search_filters.min_rating = rating;
    app.filtered_results = app.search_filters.apply(&app.search_results);
    Task::none()
}

fn handle_set_sort_option(app: &mut Movix, sort: SortOption) -> Task<Message> {
    app.search_filters.sort_by = sort;
    app.filtered_results = app.search_filters.apply(&app.search_results);
    Task::none()
}

fn handle_reset_filters(app: &mut Movix) -> Task<Message> {
    app.search_filters = SearchFilters::default();
    app.filtered_results = app.search_filters.apply(&app.search_results);
    Task::none()
}

fn handle_genres_loaded(app: &mut Movix, result: Result<Vec<Genre>, ApiError>) -> Task<Message> {
    if let Ok(genres) = result {
        app.genre_list = genres;
    }
    Task::none()
}
