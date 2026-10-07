/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;

use accesskit::Affine;
use egui::Event::Key;
use egui::{Button, EventFilter, Panel, Rect, Sense, Widget, WidgetInfo, WidgetType};
use euclid::Scale;
use servo::{DeviceIndependentPixel, DevicePixel, WebView, WebViewId};
use url::Url;

use crate::desktop::gui::Gui;
use crate::running_app_state::UserInterfaceCommand;
use crate::window::{ServoShellWindow, TopLevelWebViewCreationRequest};

pub(crate) fn tab_strip(
    window: &ServoShellWindow,
    favicon_textures: &mut HashMap<WebViewId, (egui::TextureHandle, egui::load::SizedTexture)>,
    ctx: &mut egui::Ui,
) -> egui::Response {
    Panel::top("tabs")
        .show_inside(ctx, |ui| {
            // Add scroll for overflowing tabs
            egui::ScrollArea::horizontal()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        ui.available_size(),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            for (id, webview) in window.webviews().into_iter() {
                                let favicon = favicon_textures
                                    .get(&id)
                                    .map(|(_, favicon)| favicon)
                                    .copied();
                                browser_tab(ui, window, webview, favicon);
                            }

                            let new_tab_button = ui.add(Gui::toolbar_button("+"));
                            new_tab_button.widget_info(|| {
                                let mut info = WidgetInfo::new(WidgetType::Button);
                                info.label = Some("New tab".into());
                                info
                            });
                            if new_tab_button.clicked() {
                                window
                                    .queue_user_interface_command(UserInterfaceCommand::NewWebView);
                            }

                            let new_window_button = ui.add(Gui::toolbar_button("⊞"));
                            new_window_button.widget_info(|| {
                                let mut info = WidgetInfo::new(WidgetType::Button);
                                info.label = Some("New window".into());
                                info
                            });
                            if new_window_button.clicked() {
                                let url = Url::parse("servo:newtab").expect(
                                    "Should be able to unconditionally parse 'servo:newtab' as URL",
                                );
                                window.queue_user_interface_command(
                                    UserInterfaceCommand::NewWindow(
                                        TopLevelWebViewCreationRequest::WithUrl(url),
                                    ),
                                );
                            }
                        },
                    );
                })
        })
        .response
}

/// Draws a browser tab, checking for clicks and queues appropriate [`UserInterfaceCommand`]s.
/// Using a custom widget here would've been nice, but it doesn't seem as though egui
/// supports that, so we arrange multiple Widgets in a way that they look connected.
pub(crate) fn browser_tab(
    ui: &mut egui::Ui,
    window: &ServoShellWindow,
    webview: WebView,
    favicon_texture: Option<egui::load::SizedTexture>,
) {
    let label = match (webview.page_title(), webview.url()) {
        (Some(title), _) if !title.is_empty() => title,
        (_, Some(url)) => url.to_string(),
        _ => "New Tab".into(),
    };

    let inactive_bg_color = ui.visuals().window_fill;
    let active_bg_color = ui.visuals().widgets.active.weak_bg_fill;
    let active = window.active_webview().map(|webview| webview.id()) == Some(webview.id());

    // Setup a tab frame that will contain the favicon, title and close button
    let mut tab_frame = egui::Frame::NONE.corner_radius(4).begin(ui);
    {
        tab_frame.content_ui.add_space(5.0);

        let visuals = tab_frame.content_ui.visuals_mut();
        // Remove the stroke so we don't see the border between the close button and the label
        visuals.widgets.active.bg_stroke.width = 0.0;
        visuals.widgets.hovered.bg_stroke.width = 0.0;
        // Now we make sure the fill color is always the same, irrespective of state, that way
        // we can make sure that both the label and close button have the same background color
        visuals.widgets.noninteractive.weak_bg_fill = inactive_bg_color;
        visuals.widgets.inactive.weak_bg_fill = inactive_bg_color;
        visuals.widgets.hovered.weak_bg_fill = active_bg_color;
        visuals.widgets.active.weak_bg_fill = active_bg_color;
        visuals.selection.bg_fill = active_bg_color;
        visuals.selection.stroke.color = visuals.widgets.active.fg_stroke.color;
        visuals.widgets.hovered.fg_stroke.color = visuals.widgets.active.fg_stroke.color;

        // Expansion would also show that they are 2 separate widgets
        visuals.widgets.active.expansion = 0.0;
        visuals.widgets.hovered.expansion = 0.0;

        if let Some(favicon) = favicon_texture {
            tab_frame.content_ui.add(
                egui::Image::from_texture(favicon)
                    .fit_to_exact_size(egui::vec2(16.0, 16.0))
                    .bg_fill(egui::Color32::TRANSPARENT),
            );
        }

        let tab = tab_frame
            .content_ui
            .add(Button::selectable(
                active,
                truncate_with_ellipsis(&label, 20),
            ))
            .on_hover_ui(|ui| {
                ui.label(&label);
            });

        let close_button = tab_frame
            .content_ui
            .add(egui::Button::new("X").fill(egui::Color32::TRANSPARENT));
        close_button.widget_info(|| {
            let mut info = WidgetInfo::new(WidgetType::Button);
            info.label = Some("Close".into());
            info
        });
        if close_button.clicked() || close_button.middle_clicked() || tab.middle_clicked() {
            window.queue_user_interface_command(UserInterfaceCommand::CloseWebView(webview.id()));
        } else if !active && tab.clicked() {
            window.activate_webview(webview.id());
        }
    }

    let response = tab_frame.allocate_space(ui);
    let fill_color = if active || response.hovered() {
        active_bg_color
    } else {
        inactive_bg_color
    };
    tab_frame.frame.fill = fill_color;
    tab_frame.end(ui);
}

fn truncate_with_ellipsis(input: &str, max_length: usize) -> String {
    if input.chars().count() > max_length {
        let truncated: String = input.chars().take(max_length.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        input.to_string()
    }
}

pub(crate) struct WebViewPanel {
    id: egui::Id,
    webview: WebView,
    scale_factor: Scale<f32, DeviceIndependentPixel, DevicePixel>,
    available_rect: Rect,
}

impl WebViewPanel {
    pub fn new(
        webview_id: WebViewId,
        webview: WebView,
        scale_factor: Scale<f32, DeviceIndependentPixel, DevicePixel>,
        available_rect: Rect,
    ) -> Self {
        Self {
            id: egui::Id::new(webview_id),
            webview,
            scale_factor,
            available_rect,
        }
    }

    // TODO: copy what CentralPanel does in `show_inside_dyn()`
    // to take up all the remaining space

    // TODO: when to call `interested_in_focus()`?
    // Have a look at DragValue::ui()??

    // responsibilities:
    // - create AccessKit graft node
    // - participate in egui tab order (either by being a Widget or by calling interested_in_focus)
    // - draw webview if active?
    // - observe focus and ensure AccessKit graft node is the focused node in egui's AccessKit tree
    //   (potentially happens automatically when focused)
    // - maybe handle keyboard events from egui when focused?
    //

    // Check for focus:
    // ui.memory(|mem| mem.has_focus(id))
}

impl Widget for WebViewPanel {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // if focusing GenericContainer doesn't work, consider creating a TreeUpdate which can be
        // retrieved at the end of Gui::update() setting focus to the graft node
        if let Some(tree_id) = self.webview.accesskit_tree_id() {
            let affine = {
                // The grafted WebView tree reports bounds in device pixels relative to the
                // WebView's own origin, so this node supplies the offset of the WebView within
                // the window, and scales it to the same scale as the rest of the nodes in egui's
                // AccessKit tree.
                let scale = (1.0 / self.scale_factor.get()) as f64;
                dbg!(scale);
                let x = self.available_rect.min.x as f64;
                let y = self.available_rect.min.y as f64;
                Affine::new([scale, 0.0, 0.0, scale, x, y])
            };
            ui.accesskit_node_builder(self.id, |node| {
                // TODO: does this work?
                // need to look at egui_winit::PlatformOutput to see where the focused
                // node for the egui winit tree gets set
                node.set_tree_id(tree_id);
                // Only the transform is set: AccessKit consumers exclude graft nodes from
                // the presented tree, so bounds on this node would never be read.
                node.set_transform(affine);
            });
            dbg!(self.id.accesskit_id());
        } else {
            dbg!("no accesskit_tree_id");
        }

        ui.memory_mut(|mem| mem.interested_in_focus(self.id, ui.layer_id()));
        let event_filter = EventFilter {
            tab: true,
            horizontal_arrows: true,
            vertical_arrows: true,
            escape: true,
        };
        ui.memory_mut(|mem| mem.set_focus_lock_filter(self.id, event_filter));
        let focused = ui.memory(|mem| mem.has_focus(self.id));
        // dbg!(focused);

        let events = ui.input(|i| i.filtered_events(&event_filter));
        for event in events {
            if focused && matches!(event, Key { .. }) {
                // TODO: forward keyboard events to webview
                dbg!(event);
            }
        }

        // ui.response() ?
        ui.interact(self.available_rect, self.id, Sense::FOCUSABLE)
    }
}
