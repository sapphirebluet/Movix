use iced::Task;

use crate::media::{ApiError, MediaId, Message};
use crate::tmdb::ImageSize;
use crate::video::VideoPlayer;
use crate::Movix;

pub fn handle_open_detail_popup(app: &mut Movix, media_id: MediaId) -> Task<Message> {
    app.detail_popup_open = true;
    app.detail_popup_media_id = Some(media_id);
    app.detail_popup_data = None;
    app.detail_selected_season = None;
    app.detail_episodes.clear();
    app.detail_hovered_card = None;
    app.pending_detail_hover_card = None;
    app.detail_video_frame = None;

    let Some(client) = &app.tmdb_client else {
        return Task::done(Message::PauseHeroTrailer);
    };

    let item = app
        .content_sections
        .iter()
        .flat_map(|s| &s.items)
        .find(|i| i.id == media_id)
        .or_else(|| app.hero_content.as_ref().filter(|h| h.id == media_id));

    let media_type = item
        .map(|i| i.media_type.clone())
        .unwrap_or(crate::media::MediaType::Movie);

    let fetch_client = client.clone();
    let fetch_task = Task::perform(
        async move {
            fetch_client
                .fetch_detail_popup_data(media_id, &media_type)
                .await
        },
        Message::DetailDataLoaded,
    );

    let mut tasks = vec![
        Task::done(Message::PauseHeroTrailer),
        Task::done(Message::StopCardTrailer),
        fetch_task,
    ];

    if let Some(url) = app.stream_url_cache.get(&media_id) {
        let player = app.detail_player.clone();
        let url = url.clone();
        tasks.push(Task::perform(
            async move {
                let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                let _ = p.play(media_id, &url);
            },
            |_| Message::DetailFrameTick,
        ));
    } else if let Some(Some(youtube_id)) = app.trailer_cache.get(&media_id) {
        let manager = app.trailer_manager.clone();
        let yt_id = youtube_id.clone();
        tasks.push(Task::perform(
            async move { manager.get_stream_url(&yt_id).await },
            move |result| Message::DetailTrailerLoaded(media_id, result),
        ));
    }

    Task::batch(tasks)
}

pub fn handle_close_detail_popup(app: &mut Movix) -> Task<Message> {
    let was_hero_ended = app.hero_ended;
    let should_resume_hero = app.hero_visible && !app.movie_player_active;

    app.detail_popup_open = false;
    app.detail_popup_media_id = None;
    app.detail_popup_data = None;
    app.detail_selected_season = None;
    app.detail_episodes.clear();
    app.detail_hovered_card = None;
    app.pending_detail_hover_card = None;
    app.detail_video_frame = None;

    let detail_player = app.detail_player.clone();
    let stop_detail = Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = detail_player.lock().await;
            p.stop();
        },
        |_| Message::DetailFrameTick,
    );

    if !should_resume_hero {
        return stop_detail;
    }

    if was_hero_ended {
        let hero_player = app.hero_player.clone();
        let replay_hero = Task::perform(
            async move {
                let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = hero_player.lock().await;
                let _ = p.replay();
            },
            |_| Message::HeroFrameTick,
        );
        app.hero_ended = false;
        return Task::batch([stop_detail, replay_hero]);
    }

    let hero_player = app.hero_player.clone();
    let resume_hero = Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = hero_player.lock().await;
            p.resume();
        },
        |_| Message::HeroFrameTick,
    );
    Task::batch([stop_detail, resume_hero])
}

pub fn handle_detail_data_loaded(
    app: &mut Movix,
    result: Result<crate::media::DetailPopupData, ApiError>,
) -> Task<Message> {
    let Ok(data) = result else {
        return Task::none();
    };

    let Some(client) = &app.tmdb_client else {
        app.detail_popup_data = Some(data);
        return Task::none();
    };

    let mut tasks = Vec::new();

    for cast_member in data.cast.iter().take(10) {
        if let Some(profile_path) = &cast_member.profile_path {
            let url = client.image_url(profile_path, ImageSize::Poster);
            if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
    }

    for company in &data.production_companies {
        if let Some(logo_path) = &company.logo_path {
            let url = client.image_url(logo_path, ImageSize::Original);
            if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
    }

    for item in &data.similar {
        if let Some(backdrop_path) = &item.backdrop_path {
            let url = client.image_url(backdrop_path, ImageSize::Backdrop);
            if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
        if let Some(logo_path) = &item.logo_path {
            let url = client.image_url(logo_path, ImageSize::Original);
            if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
    }

    if let Some(collection) = &data.collection {
        for item in &collection.parts {
            if let Some(backdrop_path) = &item.backdrop_path {
                let url = client.image_url(backdrop_path, ImageSize::Backdrop);
                if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                    tasks.push(Task::done(Message::LoadImage(url)));
                }
            }
            if let Some(logo_path) = &item.logo_path {
                let url = client.image_url(logo_path, ImageSize::Original);
                if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                    tasks.push(Task::done(Message::LoadImage(url)));
                }
            }
        }
    }

    if let Some(backdrop_path) = &data.media_item.backdrop_path {
        let url = client.image_url(backdrop_path, ImageSize::Backdrop);
        if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
            tasks.push(Task::done(Message::LoadImage(url)));
        }
    }

    if let Some(logo_path) = &data.media_item.logo_path {
        let url = client.image_url(logo_path, ImageSize::Original);
        if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
            tasks.push(Task::done(Message::LoadImage(url)));
        }
    }

    let is_tv = matches!(
        data.media_item.media_type,
        crate::media::MediaType::TvSeries
    );
    let has_seasons = !data.seasons.is_empty();
    let media_id = data.media_item.id;

    app.detail_popup_data = Some(data);

    if is_tv && has_seasons {
        let Some(client) = &app.tmdb_client else {
            return Task::batch(tasks);
        };
        let fetch_client = client.clone();
        let episodes_task = Task::perform(
            async move { fetch_client.fetch_season_episodes(media_id, 1).await },
            Message::DetailEpisodesLoaded,
        );
        tasks.push(episodes_task);
    }

    Task::batch(tasks)
}

pub fn handle_detail_select_season(app: &mut Movix, season: Option<u32>) -> Task<Message> {
    app.detail_selected_season = season;

    let Some(season_number) = season else {
        app.detail_episodes.clear();
        return Task::none();
    };

    let Some(media_id) = app.detail_popup_media_id else {
        return Task::none();
    };

    let Some(client) = &app.tmdb_client else {
        return Task::none();
    };

    let fetch_client = client.clone();
    Task::perform(
        async move {
            fetch_client
                .fetch_season_episodes(media_id, season_number)
                .await
        },
        Message::DetailEpisodesLoaded,
    )
}

pub fn handle_detail_episodes_loaded(
    app: &mut Movix,
    result: Result<Vec<crate::media::Episode>, ApiError>,
) -> Task<Message> {
    let Ok(episodes) = result else {
        return Task::none();
    };

    let Some(client) = &app.tmdb_client else {
        app.detail_episodes = episodes;
        return Task::none();
    };

    let mut tasks = Vec::new();
    for episode in &episodes {
        if let Some(still_path) = &episode.still_path {
            let url = client.image_url(still_path, ImageSize::Backdrop);
            if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
    }

    app.detail_episodes = episodes;
    Task::batch(tasks)
}

pub fn handle_detail_hover_card(app: &mut Movix, id: Option<MediaId>) -> Task<Message> {
    match id {
        Some(media_id) => {
            app.pending_detail_hover_card = Some(media_id);
            let player = app.detail_player.clone();
            let stop_task = Task::perform(
                async move {
                    let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                    p.stop();
                },
                |_| Message::DetailFrameTick,
            );
            let delay_task = Task::perform(
                async { tokio::time::sleep(std::time::Duration::from_millis(300)).await },
                move |_| Message::DetailHoverCardDelayed(media_id),
            );
            app.detail_video_frame = None;
            Task::batch([stop_task, delay_task])
        }
        None => {
            app.pending_detail_hover_card = None;
            app.detail_hovered_card = None;
            app.detail_video_frame = None;
            let player = app.detail_player.clone();
            Task::perform(
                async move {
                    let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                    p.stop();
                },
                |_| Message::DetailFrameTick,
            )
        }
    }
}

pub fn handle_detail_hover_card_delayed(app: &mut Movix, media_id: MediaId) -> Task<Message> {
    if app.pending_detail_hover_card != Some(media_id) {
        return Task::none();
    }
    app.detail_hovered_card = Some(media_id);

    let data = app.detail_popup_data.as_ref();
    let item = data.and_then(|d| {
        d.similar.iter().find(|i| i.id == media_id).or_else(|| {
            d.collection
                .as_ref()?
                .parts
                .iter()
                .find(|i| i.id == media_id)
        })
    });

    let Some(item) = item else {
        return Task::none();
    };

    let Some(client) = &app.tmdb_client else {
        return Task::none();
    };

    let mut tasks = Vec::new();

    if let Some(backdrop_path) = &item.backdrop_path {
        let url = client.image_url(backdrop_path, ImageSize::Backdrop);
        if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
            tasks.push(Task::done(Message::LoadImage(url)));
        }
    }

    if let Some(logo_path) = &item.logo_path {
        let url = client.image_url(logo_path, ImageSize::Original);
        if app.image_cache.get(&url).is_none() && !app.image_cache.is_pending(&url) {
            tasks.push(Task::done(Message::LoadImage(url)));
        }
    }

    if let Some(url) = app.stream_url_cache.get(&media_id) {
        let player = app.detail_player.clone();
        let url = url.clone();
        tasks.push(Task::perform(
            async move {
                let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                let _ = p.play(media_id, &url);
            },
            |_| Message::DetailFrameTick,
        ));
    } else if let Some(Some(youtube_id)) = app.trailer_cache.get(&media_id) {
        let manager = app.trailer_manager.clone();
        let yt_id = youtube_id.clone();
        tasks.push(Task::perform(
            async move { manager.get_stream_url(&yt_id).await },
            move |result| Message::DetailTrailerLoaded(media_id, result),
        ));
    } else {
        let fetch_client = client.clone();
        let media_type = item.media_type.clone();
        tasks.push(Task::perform(
            async move { fetch_client.fetch_videos(media_id, &media_type).await },
            move |result| Message::TrailerVideosLoaded(media_id, result),
        ));
    }

    Task::batch(tasks)
}

pub fn handle_detail_frame_tick(app: &mut Movix) -> Task<Message> {
    if let Ok(player) = app.detail_player.try_lock() {
        if let Some(frame) = player.get_frame() {
            app.detail_video_frame = Some(iced::widget::image::Handle::from_rgba(
                frame.width,
                frame.height,
                frame.data,
            ));
        }
    }
    Task::none()
}

pub fn handle_detail_trailer_loaded(
    app: &mut Movix,
    media_id: MediaId,
    result: Result<String, String>,
) -> Task<Message> {
    let Ok(url) = result else {
        return Task::none();
    };
    app.stream_url_cache.insert(media_id, url.clone());

    let is_detail_main = app.detail_popup_media_id == Some(media_id);
    let is_detail_hovered = app.detail_hovered_card == Some(media_id);

    if !is_detail_main && !is_detail_hovered {
        return Task::none();
    }

    let player = app.detail_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
            let _ = p.play(media_id, &url);
        },
        |_| Message::DetailFrameTick,
    )
}
