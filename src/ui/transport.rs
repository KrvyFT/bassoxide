//! 播放控制条 — 外部音频轨 + 节拍器 / 变速 / A-B 循环。

use egui::Ui;

use crate::state::AppState;
use crate::ui::material::MaterialPalette;
use bassoxide_audio::PlaybackStatus;

/// 绘制播放控制条
pub fn transport_bar(ui: &mut Ui, state: &mut AppState) {
    let has_audio = state.audio_track.is_some();
    let has_player = state.audio_player.is_some();
    let palette = MaterialPalette::for_mode(state.is_light_theme);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        let mut is_playing = false;
        if let Some(player) = &state.audio_player {
            is_playing = player.status() == PlaybackStatus::Playing;
        }

        let play_text = if is_playing { "|| 暂停" } else { "▶ 播放音频" };
        let play_btn = egui::Button::new(
            egui::RichText::new(play_text).color(palette.on_primary),
        )
        .fill(palette.primary);
        if ui
            .add_enabled(has_audio, play_btn)
            .on_hover_text("播放/暂停已加载的外部音频轨（空格）")
            .clicked()
        {
            state.sync_playback_tools_to_player();
            if let Some(player) = &state.audio_player {
                player.toggle_play_pause();
            }
        }

        let stop_btn = egui::Button::new("⏹ 停止").fill(palette.secondary_container);
        if ui.add_enabled(has_audio, stop_btn).clicked() {
            if let Some(player) = &state.audio_player {
                player.stop();
            }
        }

        ui.separator();
        if has_audio {
            if let Some(player) = &state.audio_player {
                let t = player.score_position_secs();
                ui.label(
                    egui::RichText::new(format!("谱面时间 {t:.2}s"))
                        .size(12.0)
                        .color(palette.on_surface),
                );
            }
        } else {
            ui.label(
                egui::RichText::new("请先在底部添加音频轨")
                    .size(12.0)
                    .color(palette.on_surface_variant),
            );
        }

        if has_player {
            ui.separator();

            // 节拍器
            let mut metro = state.metronome_enabled;
            if ui
                .checkbox(
                    &mut metro,
                    egui::RichText::new("节拍器").size(12.0).color(palette.on_surface),
                )
                .on_hover_text("按谱面拍点发声（快捷键 M）")
                .changed()
            {
                state.metronome_enabled = metro;
                state.sync_playback_tools_to_player();
                state.status_message = if metro {
                    "节拍器：开".into()
                } else {
                    "节拍器：关".into()
                };
            }

            // 变速
            ui.label(
                egui::RichText::new("变速")
                    .size(12.0)
                    .color(palette.on_surface_variant),
            );
            let mut rate_pct = (state.playback_rate * 100.0).round() as i32;
            let rate_drag = egui::DragValue::new(&mut rate_pct)
                .range(50..=150)
                .suffix("%")
                .speed(1.0);
            if ui
                .add(rate_drag)
                .on_hover_text("练习变速 50%–150%（音高随速率变化）")
                .changed()
            {
                state.set_playback_rate_ui(rate_pct as f32 / 100.0);
                state.status_message = format!("变速 {}%", rate_pct);
            }

            ui.separator();

            // A-B 循环
            if ui
                .button("设 A")
                .on_hover_text("以当前谱面时间为循环起点（[）")
                .clicked()
            {
                state.set_loop_a_here();
            }
            if ui
                .button("设 B")
                .on_hover_text("以当前谱面时间为循环终点（]）")
                .clicked()
            {
                state.set_loop_b_here();
            }
            let mut loop_on = state.loop_enabled;
            let loop_ready = state.loop_a.is_some() && state.loop_b.is_some();
            if ui
                .add_enabled(
                    loop_ready,
                    egui::Checkbox::new(
                        &mut loop_on,
                        egui::RichText::new("循环").size(12.0).color(palette.on_surface),
                    ),
                )
                .on_hover_text("在 A–B 间循环（快捷键 L）")
                .changed()
            {
                state.loop_enabled = loop_on;
                state.sync_playback_tools_to_player();
            }
            if ui.button("清除").on_hover_text("清除 A/B 点").clicked() {
                state.clear_loop_points();
            }

            let a_txt = state
                .loop_a
                .map(|t| format!("{t:.2}"))
                .unwrap_or_else(|| "—".into());
            let b_txt = state
                .loop_b
                .map(|t| format!("{t:.2}"))
                .unwrap_or_else(|| "—".into());
            ui.label(
                egui::RichText::new(format!("A={a_txt}s  B={b_txt}s"))
                    .size(11.0)
                    .color(palette.on_surface_variant),
            );
        }
    });
}
