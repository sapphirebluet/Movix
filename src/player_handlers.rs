use iced::Task;

use crate::media::{MediaId, Message};
use crate::movie_player::{MoviePlayer, VoeStreamResolver};
use crate::video::VideoPlayer;
use crate::Movix;

pub fn handle_play_content(app: &mut Movix, id: MediaId) -> Task<Message> {
    let item = app
        .content_sections
        .iter()
        .flat_map(|s| &s.items)
        .find(|i| i.id == id)
        .or_else(|| app.hero_content.as_ref().filter(|h| h.id == id));

    let Some(item) = item else {
        return Task::none();
    };
    let title = item.title.clone();

    app.movie_player_active = true;
    app.movie_player_media_id = Some(id);
    app.movie_player_title = Some(title.clone());
    app.movie_player_loading = true;
    app.movie_player_controls_visible = true;
    app.movie_player_error = None;
    app.hero_video_frame = None;
    app.card_video_frame = None;
    app.hovered_card = None;
    app.pending_hover_card = None;

    let hero_player = app.hero_player.clone();
    let card_player = app.card_player.clone();
    let stop_task = Task::perform(
        async move {
            if let Ok(mut h) = hero_player.try_lock() {
                h.stop();
            }
            if let Ok(mut c) = card_player.try_lock() {
                c.stop();
            }
        },
        |_| Message::HeroFrameTick,
    );

    let resolve_task = Task::perform(
        async move { VoeStreamResolver::get_download_url(&title).await },
        move |result| Message::MoviePlayerStreamResolved(id, result),
    );
    Task::batch([stop_task, resolve_task])
}

pub fn handle_trailer_stream_url_loaded(
    app: &mut Movix,
    media_id: MediaId,
    result: Result<String, String>,
) -> Task<Message> {
    let Ok(url) = result else {
        return Task::none();
    };
    app.stream_url_cache.insert(media_id, url.clone());

    if app.movie_player_active {
        return Task::none();
    }

    if app.detail_popup_open {
        let is_detail_media = app.detail_popup_media_id == Some(media_id);
        let is_detail_hovered = app.detail_hovered_card == Some(media_id);

        if is_detail_media || is_detail_hovered {
            let player = app.detail_player.clone();
            return Task::perform(
                async move {
                    let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                    let _ = p.play(media_id, &url);
                },
                |_| Message::DetailFrameTick,
            );
        }
        return Task::none();
    }

    let is_hero = app.hero_content.as_ref().map(|h| h.id) == Some(media_id);
    let is_hovered = app.hovered_card == Some(media_id);

    if is_hero && app.hero_visible {
        let player = app.hero_player.clone();
        return Task::perform(
            async move {
                let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                let _ = p.play(media_id, &url);
            },
            |_| Message::HeroFrameTick,
        );
    }
    if is_hovered {
        let player = app.card_player.clone();
        return Task::perform(
            async move {
                let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                let _ = p.play(media_id, &url);
            },
            |_| Message::CardFrameTick,
        );
    }
    Task::none()
}

pub fn handle_hero_frame_tick(app: &mut Movix) -> Task<Message> {
    if app.movie_player_active {
        return Task::none();
    }
    if let Ok(mut player) = app.hero_player.try_lock() {
        if player.check_ended() {
            app.hero_ended = true;
        }
        app.hero_muted = player.is_muted();
        if let Some(frame) = player.get_frame() {
            app.hero_video_frame = Some(iced::widget::image::Handle::from_rgba(
                frame.width,
                frame.height,
                frame.data,
            ));
        }
    }
    Task::none()
}

pub fn handle_card_frame_tick(app: &mut Movix) -> Task<Message> {
    if app.movie_player_active {
        return Task::none();
    }
    if let Ok(player) = app.card_player.try_lock() {
        if let Some(frame) = player.get_frame() {
            app.card_video_frame = Some(iced::widget::image::Handle::from_rgba(
                frame.width,
                frame.height,
                frame.data,
            ));
        }
    }
    Task::none()
}

pub fn handle_stop_card_trailer(app: &mut Movix) -> Task<Message> {
    app.card_video_frame = None;
    let player = app.card_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
            p.stop();
        },
        |_| Message::CardFrameTick,
    )
}

pub fn handle_pause_hero_trailer(app: &mut Movix) -> Task<Message> {
    let player = app.hero_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
            p.pause();
        },
        |_| Message::HeroFrameTick,
    )
}

pub fn handle_resume_hero_trailer(app: &mut Movix) -> Task<Message> {
    if app.movie_player_active || app.detail_popup_open {
        return Task::none();
    }
    if !app.hero_visible {
        return Task::none();
    }

    let hero_id = match app.hero_content.as_ref() {
        Some(hero) => hero.id,
        None => return Task::none(),
    };

    let has_pipeline = app
        .hero_player
        .try_lock()
        .map(|p: tokio::sync::MutexGuard<'_, VideoPlayer>| p.has_pipeline())
        .unwrap_or(false);

    if has_pipeline {
        let player = app.hero_player.clone();
        return Task::perform(
            async move {
                let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                p.resume();
            },
            |_| Message::HeroFrameTick,
        );
    }

    if let Some(url) = app.stream_url_cache.get(&hero_id).cloned() {
        let player = app.hero_player.clone();
        return Task::perform(
            async move {
                let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
                let _ = p.play(hero_id, &url);
            },
            |_| Message::HeroFrameTick,
        );
    }

    Task::none()
}

pub fn handle_toggle_hero_mute(app: &mut Movix) -> Task<Message> {
    let player = app.hero_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
            p.toggle_mute();
        },
        |_| Message::HeroFrameTick,
    )
}

pub fn handle_replay_hero_trailer(app: &mut Movix) -> Task<Message> {
    app.hero_ended = false;
    let player = app.hero_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, VideoPlayer> = player.lock().await;
            let _ = p.replay();
        },
        |_| Message::HeroFrameTick,
    )
}

pub fn handle_movie_player_open(
    app: &mut Movix,
    media_id: MediaId,
    title: String,
) -> Task<Message> {
    app.movie_player_active = true;
    app.movie_player_media_id = Some(media_id);
    app.movie_player_title = Some(title.clone());
    app.movie_player_loading = true;

    Task::perform(
        async move { VoeStreamResolver::get_download_url(&title).await },
        move |result| Message::MoviePlayerStreamResolved(media_id, result),
    )
}

pub fn handle_movie_stream_resolved(
    app: &mut Movix,
    media_id: MediaId,
    result: Result<String, String>,
) -> Task<Message> {
    app.movie_player_loading = false;
    match result {
        Ok(url) => {
            let player = app.movie_player.clone();
            let progress_store = app.progress_store.clone();
            Task::perform(
                async move {
                    let mut p: tokio::sync::MutexGuard<'_, MoviePlayer> = player.lock().await;
                    let _ = p.play(media_id, &url);
                    let stored_pos = progress_store.lock().await.get(media_id);
                    if let Some(pos) = stored_pos {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        p.seek(pos);
                    }
                },
                |_| Message::MoviePlayerFrameTick,
            )
        }
        Err(error) => {
            app.movie_player_error = Some(error);
            Task::none()
        }
    }
}

pub fn handle_movie_player_close(app: &mut Movix) -> Task<Message> {
    app.movie_player_active = false;
    app.movie_player_frame = None;
    app.movie_player_error = None;

    let should_resume_hero = app.hero_visible && !app.detail_popup_open;

    let player = app.movie_player.clone();
    let stop_task = Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, MoviePlayer> = player.lock().await;
            p.save_progress().await;
            p.stop();
        },
        |_| Message::HeroFrameTick,
    );

    if should_resume_hero {
        Task::batch([stop_task, Task::done(Message::ResumeHeroTrailer)])
    } else {
        stop_task
    }
}

pub fn handle_movie_toggle_play(app: &mut Movix) -> Task<Message> {
    let player = app.movie_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, MoviePlayer> = player.lock().await;
            p.toggle_play_pause();
        },
        |_| Message::MoviePlayerFrameTick,
    )
}

pub fn handle_movie_seek(app: &mut Movix, position: f64) -> Task<Message> {
    let player = app.movie_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, MoviePlayer> = player.lock().await;
            p.seek(position);
        },
        |_| Message::MoviePlayerFrameTick,
    )
}

pub fn handle_movie_seek_relative(app: &mut Movix, delta: f64) -> Task<Message> {
    let player = app.movie_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, MoviePlayer> = player.lock().await;
            p.seek_relative(delta);
        },
        |_| Message::MoviePlayerFrameTick,
    )
}

pub fn handle_movie_set_volume(app: &mut Movix, volume: f64) -> Task<Message> {
    app.movie_player_volume = volume;
    let player = app.movie_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, MoviePlayer> = player.lock().await;
            p.set_volume(volume);
        },
        |_| Message::MoviePlayerFrameTick,
    )
}

pub fn handle_movie_toggle_mute(app: &mut Movix) -> Task<Message> {
    let player = app.movie_player.clone();
    Task::perform(
        async move {
            let mut p: tokio::sync::MutexGuard<'_, MoviePlayer> = player.lock().await;
            p.toggle_mute();
        },
        |_| Message::MoviePlayerFrameTick,
    )
}

pub fn handle_movie_frame_tick(app: &mut Movix) {
    if let Ok(mut player) = app.movie_player.try_lock() {
        app.movie_player_position = player.position();
        app.movie_player_duration = player.duration();
        app.movie_player_playing = player.is_playing();
        app.movie_player_muted = player.is_muted();
        app.movie_player_volume = player.volume();

        if let Some(frame) = player.get_new_frame() {
            app.movie_player_frame = Some(iced::widget::image::Handle::from_rgba(
                frame.width,
                frame.height,
                frame.data,
            ));
        } else if app.movie_player_frame.is_none() {
            if let Some(frame) = player.get_current_frame() {
                app.movie_player_frame = Some(iced::widget::image::Handle::from_rgba(
                    frame.width,
                    frame.height,
                    frame.data,
                ));
            }
        }
        if player.check_ended() {
            app.movie_player_playing = false;
        }
    }

    if let Some(timer) = app.movie_player_controls_timer {
        if timer.elapsed() > std::time::Duration::from_secs(3) {
            app.movie_player_controls_visible = false;
            app.movie_player_controls_timer = None;
        }
    }
}

pub fn handle_movie_show_controls(app: &mut Movix) -> Task<Message> {
    if !app.movie_player_controls_visible {
        app.movie_player_controls_visible = true;
    }
    app.movie_player_controls_timer = Some(std::time::Instant::now());
    Task::none()
}

pub fn handle_movie_hide_controls(app: &mut Movix) -> Task<Message> {
    app.movie_player_controls_visible = false;
    app.movie_player_controls_timer = None;
    Task::none()
}
