use iced::Task;

use crate::media::{MediaId, Message};
use crate::movie_player::VoeStreamResolver;
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

    app.hero_player.stop();
    app.card_player.stop();

    Task::perform(
        async move { VoeStreamResolver::get_download_url(&title).await },
        move |result| Message::MoviePlayerStreamResolved(id, result),
    )
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
            return Task::done(Message::PlayDetailTrailer(media_id));
        }
        return Task::none();
    }

    let is_hero = app.hero_content.as_ref().map(|h| h.id) == Some(media_id);
    let is_hovered = app.hovered_card == Some(media_id);

    if is_hero && app.hero_visible {
        return Task::done(Message::PlayHeroTrailer(media_id));
    }
    if is_hovered {
        return Task::done(Message::PlayCardTrailer(media_id));
    }
    Task::none()
}

pub fn handle_play_hero_trailer(app: &mut Movix, media_id: MediaId) -> Task<Message> {
    if let Some(url) = app.stream_url_cache.get(&media_id).cloned() {
        let _ = app.hero_player.play(media_id, &url);
    }
    Task::none()
}

pub fn handle_play_card_trailer(app: &mut Movix, media_id: MediaId) -> Task<Message> {
    if let Some(url) = app.stream_url_cache.get(&media_id).cloned() {
        let _ = app.card_player.play(media_id, &url);
    }
    Task::none()
}

pub fn handle_play_detail_trailer(app: &mut Movix, media_id: MediaId) -> Task<Message> {
    if let Some(url) = app.stream_url_cache.get(&media_id).cloned() {
        let _ = app.detail_player.play(media_id, &url);
    }
    Task::none()
}

pub fn handle_hero_frame_tick(app: &mut Movix) -> Task<Message> {
    if app.movie_player_active {
        return Task::none();
    }
    if app.hero_player.check_ended() {
        app.hero_ended = true;
    }
    app.hero_muted = app.hero_player.is_muted();
    if let Some(frame) = app.hero_player.render_frame() {
        app.hero_video_frame = Some(iced::widget::image::Handle::from_rgba(
            frame.width,
            frame.height,
            frame.data,
        ));
    }
    Task::none()
}

pub fn handle_card_frame_tick(app: &mut Movix) -> Task<Message> {
    if app.movie_player_active {
        return Task::none();
    }
    if let Some(frame) = app.card_player.render_frame() {
        app.card_video_frame = Some(iced::widget::image::Handle::from_rgba(
            frame.width,
            frame.height,
            frame.data,
        ));
    }
    Task::none()
}

pub fn handle_stop_card_trailer(app: &mut Movix) -> Task<Message> {
    app.card_video_frame = None;
    app.card_player.stop();
    Task::none()
}

pub fn handle_pause_hero_trailer(app: &mut Movix) -> Task<Message> {
    app.hero_player.pause();
    Task::none()
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

    if app.hero_player.has_pipeline() {
        app.hero_player.resume();
        return Task::none();
    }

    if let Some(url) = app.stream_url_cache.get(&hero_id).cloned() {
        let _ = app.hero_player.play(hero_id, &url);
    }
    Task::none()
}

pub fn handle_toggle_hero_mute(app: &mut Movix) -> Task<Message> {
    app.hero_player.toggle_mute();
    Task::none()
}

pub fn handle_replay_hero_trailer(app: &mut Movix) -> Task<Message> {
    app.hero_ended = false;
    let _ = app.hero_player.replay();
    Task::none()
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
            let _ = app.movie_player.play(media_id, &url);
            if let Some(pos) = app.movie_player.get_stored_position(media_id) {
                app.movie_player.seek(pos);
            }
            Task::none()
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

    app.movie_player.save_progress_sync();
    app.movie_player.stop();

    if should_resume_hero {
        Task::done(Message::ResumeHeroTrailer)
    } else {
        Task::none()
    }
}

pub fn handle_movie_toggle_play(app: &mut Movix) -> Task<Message> {
    app.movie_player.toggle_play_pause();
    Task::none()
}

pub fn handle_movie_seek(app: &mut Movix, position: f64) -> Task<Message> {
    app.movie_player.seek(position);
    Task::none()
}

pub fn handle_movie_seek_relative(app: &mut Movix, delta: f64) -> Task<Message> {
    app.movie_player.seek_relative(delta);
    Task::none()
}

pub fn handle_movie_set_volume(app: &mut Movix, volume: f64) -> Task<Message> {
    app.movie_player_volume = volume;
    app.movie_player.set_volume(volume);
    Task::none()
}

pub fn handle_movie_toggle_mute(app: &mut Movix) -> Task<Message> {
    app.movie_player.toggle_mute();
    Task::none()
}

pub fn handle_movie_frame_tick(app: &mut Movix) {
    app.movie_player_position = app.movie_player.position();
    app.movie_player_duration = app.movie_player.duration();
    app.movie_player_playing = app.movie_player.is_playing();
    app.movie_player_muted = app.movie_player.is_muted();
    app.movie_player_volume = app.movie_player.volume();

    if let Some(frame) = app.movie_player.get_new_frame() {
        app.movie_player_frame = Some(iced::widget::image::Handle::from_rgba(
            frame.width,
            frame.height,
            frame.data,
        ));
    } else if app.movie_player_frame.is_none() {
        if let Some(frame) = app.movie_player.get_current_frame() {
            app.movie_player_frame = Some(iced::widget::image::Handle::from_rgba(
                frame.width,
                frame.height,
                frame.data,
            ));
        }
    }
    if app.movie_player.check_ended() {
        app.movie_player_playing = false;
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
