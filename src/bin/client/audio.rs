//! audio systems split out of the monolithic client (plan.md Phase 1 item 8).
#![allow(unused_imports)]
pub(crate) use super::input::*;
pub(crate) use super::net::*;
pub(crate) use super::scene::*;
pub(crate) use super::ui::*;
pub(crate) use super::vfx::*;
use super::*;

pub(crate) fn play_sfx_queue(
    mut commands: Commands,
    mut queue: ResMut<SfxQueue>,
    assets: Res<AudioAssets>,
    settings: Res<ClientSettings>,
) {
    for sfx in queue.queue.drain(..) {
        let gain = sfx.gain() * settings.effective_volume();
        if gain <= 0.0 {
            continue;
        }
        commands.spawn((
            AudioPlayer::new(sfx.handle(&assets).clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(gain)),
        ));
    }
}

/// Mute toggle (V) and, while the settings overlay is open, live volume,
/// fullscreen, and resolution controls (plan.md Phase 1 item 10).
pub(crate) fn volume_toggle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ClientSettings>,
    mut overlay: ResMut<SettingsOverlay>,
    mut window_query: Query<&mut Window>,
) {
    if keys.just_pressed(KeyCode::KeyV) {
        settings.muted = !settings.muted;
        settings.save();
    }
    if keys.just_pressed(KeyCode::KeyO) {
        overlay.open = !overlay.open;
    }
    if !overlay.open {
        return;
    }

    let mut changed = false;
    if keys.just_pressed(KeyCode::Comma) {
        settings.master_volume = (settings.master_volume - 0.1).clamp(0.0, 1.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Period) {
        settings.master_volume = (settings.master_volume + 0.1).clamp(0.0, 1.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        settings.muted = !settings.muted;
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        settings.fullscreen = !settings.fullscreen;
        changed = true;
        if let Ok(mut window) = window_query.single_mut() {
            window.mode = if settings.fullscreen {
                bevy::window::WindowMode::BorderlessFullscreen(
                    bevy::window::MonitorSelection::Current,
                )
            } else {
                bevy::window::WindowMode::Windowed
            };
        }
    }
    if keys.just_pressed(KeyCode::KeyN) {
        settings.resolution_index = (settings.resolution_index + 1) % RESOLUTION_PRESETS.len();
        changed = true;
        let (width, height) = settings.resolution();
        if let Ok(mut window) = window_query.single_mut() {
            window.mode = bevy::window::WindowMode::Windowed;
            window.resolution = bevy::window::WindowResolution::new(width as u32, height as u32);
        }
    }
    if changed {
        settings.save();
    }
}

/// Victory/defeat stingers on the GameOver phase transition.
pub(crate) fn announce_phase_sfx(
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    mut queue: ResMut<SfxQueue>,
    mut announced: Local<Option<MatchPhase>>,
) {
    let Some(snapshot) = &state.snapshot else {
        *announced = None;
        return;
    };
    if *announced == Some(snapshot.phase) {
        return;
    }
    let transitioned_to_gameover = snapshot.phase == MatchPhase::GameOver;
    let previous = *announced;
    *announced = Some(snapshot.phase);
    if !transitioned_to_gameover || previous.is_none() {
        return;
    }
    let stinger = match (net.team, snapshot.winner) {
        (Some(team), Some(winner)) if winner == team => Sfx::Victory,
        (Some(_), Some(_)) => Sfx::Defeat,
        _ => return,
    };
    queue.push(stinger);
}
