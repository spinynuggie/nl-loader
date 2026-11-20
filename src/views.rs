use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    black, deferred, div, ease_in_out, img, linear_color_stop, linear_gradient, percentage,
    prelude::FluentBuilder,
    pulsating_between, px, rgb, transparent_black, white, Animation, AnimationExt as _, AnyElement,
    BoxShadow, Context, Div, FontWeight, Hsla, ImageSource, ObjectFit, Render, SharedString, TextRun,
    Transformation, Window,
};

use crate::app::{Branch, Game, LauncherApp, View};
use crate::theme::{mix, Palette};
use crate::ui::{
    icon, no_drag_div, parse_changelog_line, plain, ChangelogSegment, API_DOCS_URL, DISCORD_URL,
    FONT_FAMILY, WEBSITE_URL,
};

impl Render for LauncherApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.step_hover_animations(window);

        div()
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(transparent_black())
            .font_family(FONT_FAMILY)
            .text_color(self.theme.text)
            .child(self.render_shell(window, cx))
    }
}

impl LauncherApp {
    fn render_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme.clone();

        if self.view == View::Boot {
            return div()
                .relative()
                .size(px(260.))
                .flex_none()
                .overflow_hidden()
                .rounded(px(9.))
                .bg(theme.main_bg_opaque)
                .shadow(shell_shadows(&theme))
                .child(self.render_boot(&theme))
                .into_any_element();
        }

        let launcher = self.render_launcher(window, cx);

        div()
            .relative()
            .flex_none()
            .overflow_hidden()
            .rounded(px(9.))
            .bg(theme.main_bg_opaque)
            .shadow(shell_shadows(&theme))
            .text_color(theme.text)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if this.any_menu_open() {
                        this.close_menus(cx);
                        cx.notify();
                    } else {
                        start_window_drag(window);
                    }
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(px(530.))
                    .h(px(400.))
                    .child(launcher)
                    .with_animation(
                        "launcher-fade",
                        Animation::new(Duration::from_millis(450)),
                        |element, delta| element.opacity(delayed(delta, 170.0, 450.0, ease_in_out)),
                    ),
            )
            .with_animation(
                "shell-expand",
                Animation::new(Duration::from_millis(430)).with_easing(ease_smooth()),
                |element, delta| {
                    element
                        .w(px(260.0 + 270.0 * delta))
                        .h(px(260.0 + 140.0 * delta))
                },
            )
            .into_any_element()
    }

    fn render_boot(&self, theme: &Palette) -> AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                gpui::svg()
                    .path("icons/spinner.svg")
                    .size(px(34.))
                    .text_color(theme.spinner)
                    .with_animation(
                        "boot-spin",
                        Animation::new(Duration::from_millis(1400)).repeat(),
                        |svg, delta| {
                            svg.with_transformation(Transformation::rotate(percentage(delta)))
                        },
                    ),
            )
            .into_any_element()
    }

    fn render_launcher(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme.clone();
        let hover_text = theme.text_hover();

        let mut root = div()
            .relative()
            .size_full()
            .child(
                plain("BS", 38.0, 900.0, theme.logo)
                    .absolute()
                    .left(px(22.))
                    .top(px(22.))
                    .line_height(px(38.)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(18.))
                    .top(px(105.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(self.render_nav_link("nav-website", "Website", theme.text, hover_text, Some(WEBSITE_URL), cx))
                    .child(self.render_nav_link("nav-support", "Support", theme.text, hover_text, Some(DISCORD_URL), cx))
                    .child(self.render_nav_link("nav-market", "Market", theme.text, hover_text, None, cx)),
            )
            .child(self.render_profile_trigger(&theme, cx))
            .child(self.render_window_controls(&theme, cx))
            .child(self.render_subscriptions(&theme, cx));

        if matches!(
            self.view,
            View::Details | View::ClosingDetails | View::Launching
        ) {
            root = root.child(self.render_details(cx));
        }

        if self.profile_open {
            let dim = div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.close_menus(cx);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
            let dim = if self.profile_closing {
                dim.with_animation(
                    "profile-dim-out",
                    Animation::new(Duration::from_millis(210)),
                    |element, delta| element.bg(black().opacity(0.66 * (1.0 - delta))),
                )
            } else {
                dim.with_animation(
                    "profile-dim",
                    Animation::new(Duration::from_millis(210)),
                    |element, delta| element.bg(black().opacity(0.66 * delta)),
                )
            };
            root = root
                .child(dim)
                .child(self.render_profile_popout(window, cx));
        }

        root.into_any_element()
    }

    fn render_profile_trigger(
        &mut self,
        theme: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let avatar = self.render_avatar(40.0);
        let t = if self.profile_open {
            1.0
        } else {
            self.hover_t("profile-trigger")
        };
        no_drag_div(
            div()
                .id("profile-trigger")
                .absolute()
                .left(px(20.))
                .bottom(px(20.))
                .w(px(132.))
                .flex()
                .items_center()
                .gap(px(8.))
                .opacity(1.0 - 0.18 * t)
                .on_hover(cx.listener(|this, hovered, _, cx| {
                    this.set_hover("profile-trigger", *hovered, 130.0, cx);
                }))
                .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.open_profile(cx)))
                .child(avatar)
                .child(
                    div()
                        .min_w_0().truncate()
                        .font_family(FONT_FAMILY)
                        .text_size(px(14.))
                        .font_weight(FontWeight(100.0))
                        .line_height(px(28.))
                        .text_color(theme.active_text)
                        .child(self.username.clone()),
                ),
        )
        .into_any_element()
    }

    fn render_window_controls(&self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let theme = *theme;
        let min_color = mix(theme.text, theme.active_text, self.hover_t("win-min"));
        let close_color = mix(theme.text, theme.active_text, self.hover_t("win-close"));
        div()
            .absolute()
            .right(px(17.))
            .top(px(15.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                no_drag_div(
                    div()
                        .id("win-min")
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_hover(cx.listener(|this, hovered, _, cx| {
                            this.set_hover("win-min", *hovered, 120.0, cx);
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, |_, window, _| window.minimize_window())
                        .child(
                            div()
                                .w(px(12.))
                                .h(px(1.5))
                                .rounded_full()
                                .bg(min_color),
                        ),
                )
                .into_any_element(),
            )
            .child(
                no_drag_div(
                    div()
                        .id("win-close")
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_hover(cx.listener(|this, hovered, _, cx| {
                            this.set_hover("win-close", *hovered, 120.0, cx);
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, |_, _, cx| cx.quit())
                        .child(icon("icons/close.svg", 19.0, close_color)),
                )
                .into_any_element(),
            )
            .into_any_element()
    }

    fn render_subscriptions(&mut self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .absolute()
            .left(px(176.))
            .top(px(16.))
            .w(px(323.))
            .flex()
            .flex_col()
            .child(
                div().ml(px(7.)).child(
                    plain("Subscription", 19.0, 500.0, theme.active_text).line_height(px(32.3)),
                ),
            )
            .child(
                div().ml(px(7.)).mt(px(6.)).mb(px(45.)).child(
                    plain("Available subscriptions", 14.5, 300.0, theme.text)
                        .line_height(px(20.)),
                ),
            )
            .child(self.render_subscription_card(
                Game::Cs2Legacy,
                self.installed.cs2_legacy_branch,
                theme,
                cx,
            ))
            .child(self.render_subscription_card(
                Game::CsgoStandalone,
                self.installed.csgo_standalone,
                theme,
                cx,
            ))
            .into_any_element()
    }

    fn render_subscription_card(
        &mut self,
        game: Game,
        enabled: bool,
        theme: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let card = div()
            .relative()
            .w_full()
            .h(px(73.))
            .rounded(px(12.))
            .overflow_hidden()
            .border(px(0.5))
            .border_color(theme.border)
            .bg(theme.block_bg)
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when(enabled, |element| {
                element.on_mouse_up(gpui::MouseButton::Left, cx.listener(
                    move |this, _, _, cx| {
                        this.open_details(game, cx);
                    },
                ))
            })
            .when(!enabled, |element| {
                element
                    .cursor(gpui::CursorStyle::OperationNotAllowed)
                    .opacity(0.4)
            })
            .child(
                div()
                    .absolute()
                    .left(px(12.))
                    .top(px(8.))
                    .child(
                        plain(game.label(), 13.5, 500.0, theme.active_text).line_height(px(16.)),
                    )
                    .child(
                        div().mt(px(9.)).child(
                            plain(
                                if enabled { "Expires Never" } else { "⚠️ Not Installed" },
                                13.0,
                                100.0,
                                if enabled {
                                    theme.text_preview
                                } else {
                                    Hsla::from(rgb(0xff6b6b))
                                },
                            )
                            .line_height(px(17.)),
                        ),
                    ),
            )
            .child(game_icon(22.0, 5.0, !enabled).absolute().right(px(10.)).top(px(11.)));

        div()
            .relative()
            .w_full()
            .mb(px(11.))
            .child(card)
            .with_animation(
                match game {
                    Game::Cs2Legacy => "card-in-legacy",
                    Game::CsgoStandalone => "card-in-standalone",
                },
                Animation::new(Duration::from_millis(620)),
                |element, delta| {
                    let t = delayed(delta, 260.0, 620.0, ease_in_out);
                    element.opacity(t).top(px(4.0 * (1.0 - t)))
                },
            )
            .into_any_element()
    }

    fn render_details(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme.clone();
        let launching = self.view == View::Launching;
        let closing = self.view == View::ClosingDetails;

        let dim = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.any_menu_open() {
                        this.close_menus(cx);
                    } else if this.view == View::Details {
                        this.close_details(cx);
                    }
                    cx.stop_propagation();
                }),
            );
        let dim = if closing {
            dim.with_animation(
                "details-dim-out",
                Animation::new(Duration::from_millis(210)).with_easing(ease_soft()),
                |element, delta| element.bg(black().opacity(0.66 * (1.0 - delta))),
            )
            .into_any_element()
        } else {
            dim.with_animation(
                "details-dim",
                Animation::new(Duration::from_millis(360)).with_easing(ease_soft()),
                |element, delta| element.bg(black().opacity(0.66 * delta)),
            )
            .into_any_element()
        };

        let content: AnyElement = if launching {
            self.render_launch_content(&theme)
        } else {
            self.render_details_content(&theme, cx)
        };

        let panel = div()
            .absolute()
            .overflow_hidden()
            .rounded(px(10.))
            .border_1()
            .border_color(theme.white_border())
            .bg(theme.block_bg_opaque)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.close_menus(cx);
                    cx.stop_propagation();
                }),
            )
            .shadow(vec![BoxShadow {
                color: theme.shadow_soft,
                offset: gpui::point(px(0.), px(14.)),
                blur_radius: px(21.),
                spread_radius: px(0.),
            }])
            .child(content);

        let panel = if launching {
            panel
                .with_animation(
                    "details-launch",
                    Animation::new(Duration::from_millis(520)).with_easing(ease_smooth()),
                    |element, delta| {
                        element
                            .left(px(35.0 + (77.5 - 35.0) * delta))
                            .top(px(30.0 + (75.0 - 30.0) * delta))
                            .w(px(460.0 + (375.0 - 460.0) * delta))
                            .h(px(340.0 + (250.0 - 340.0) * delta))
                            .rounded(px(10.0 - 6.0 * delta))
                    },
                )
                .into_any_element()
        } else if closing {
            panel
                .left(px(35.))
                .top(px(30.))
                .w(px(460.))
                .h(px(340.))
                .with_animation(
                    "details-close",
                    Animation::new(Duration::from_millis(210)).with_easing(ease_soft()),
                    |element, delta| element.opacity(1.0 - delta),
                )
                .into_any_element()
        } else {
            panel
                .left(px(35.))
                .top(px(30.))
                .w(px(460.))
                .h(px(340.))
                .with_animation(
                    "details-in",
                    Animation::new(Duration::from_millis(320)).with_easing(ease_smooth()),
                    |element, delta| {
                        element
                            .top(px(30.0 + (1.0 - delta) * 3.4))
                            .opacity(delta)
                    },
                )
                .into_any_element()
        };

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(dim)
            .child(panel)
            .into_any_element()
    }

    fn render_details_content(&mut self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut body = div()
            .absolute()
            .top(px(70.))
            .left_0()
            .w_full()
            .h(px(215.))
            .px(px(20.))
            .pt(px(16.))
            .flex();

        if self.launch_error.is_empty() {
            body = body
                .child(self.render_metadata(theme, cx))
                .child(self.render_changelog(theme, cx));
        } else {
            body = body.child(self.render_launch_error(theme, cx));
        }

        let content = div()
            .size_full()
            .child(self.render_details_header(theme, cx))
            .child(body)
            .child(self.render_details_footer(theme, cx));

        if self.launch_pending {
            content
                .relative()
                .with_animation(
                    "details-fade-out",
                    Animation::new(Duration::from_millis(280)).with_easing(ease_soft()),
                    |element, delta| {
                        element
                            .top(px(2.0 * delta))
                            .opacity(1.0 - delta)
                    },
                )
                .into_any_element()
        } else {
            content.into_any_element()
        }
    }

    fn render_details_header(&self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let theme = *theme;
        let close_color = mix(theme.text, theme.active_text, self.hover_t("detail-close"));
        div()
            .relative()
            .h(px(70.))
            .border_b_1()
            .border_color(theme.separator)
            .child(game_icon(30.0, 8.0, false).absolute().left(px(17.)).top(px(19.)))
            .child(
                div()
                    .absolute()
                    .left(px(58.))
                    .top_0()
                    .h(px(70.))
                    .flex()
                    .items_center()
                    .child(plain(self.game.label(), 18.0, 500.0, theme.active_text)),
            )
            .child(
                no_drag_div(
                    div()
                        .id("detail-close")
                        .absolute()
                        .right(px(15.))
                        .top(px(25.))
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_hover(cx.listener(|this, hovered, _, cx| {
                            this.set_hover("detail-close", *hovered, 120.0, cx);
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.close_details(cx)))
                        .child(icon("icons/close.svg", 19.0, close_color)),
                )
                .into_any_element(),
            )
            .into_any_element()
    }

    fn render_metadata(&mut self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut column = div()
            .w(px(197.))
            .flex_none()
            .flex()
            .flex_col()
            .child(self.render_branch_row(theme, cx))
            .child(
                div()
                    .h(px(24.))
                    .mb(px(3.))
                    .flex()
                    .items_center()
                    .child(plain("Updated:", 13.0, 300.0, theme.active_text))
                    .child(div().ml(px(4.)).child(plain(
                        self.updated_at_label(),
                        13.0,
                        500.0,
                        theme.link_active,
                    ))),
            )
            .child(self.render_config_row(theme, cx))
            .child(
                div()
                    .h(px(24.))
                    .mb(px(3.))
                    .flex()
                    .items_center()
                    .child(plain("Last Launch:", 13.0, 300.0, theme.active_text))
                    .child(div().ml(px(4.)).child(plain(
                        "Just Now",
                        13.0,
                        500.0,
                        theme.link_active,
                    ))),
            );
        column = column.text_size(px(13.)).line_height(px(24.));
        column.into_any_element()
    }

    fn render_metadata_row(
        &self,
        id: &'static str,
        label: &'static str,
        icon_path: &'static str,
        value: String,
        max_w: f32,
        open: bool,
        theme: &Palette,
        cx: &mut Context<Self>,
        on_toggle: impl Fn(&mut Self, &gpui::MouseUpEvent, &mut Window, &mut Context<Self>) + 'static,
        menu: Option<AnyElement>,
    ) -> AnyElement {
        let theme = *theme;
        let t = self.hover_t(id);
        let (bg, border, icon_color) = if open {
            (theme.button, transparent_black(), theme.button_active_text)
        } else {
            (
                theme.button_half().opacity(t),
                theme.strong_border().opacity(1.0 - t),
                mix(theme.link_active, theme.active_text, t),
            )
        };

        div()
            .relative()
            .h(px(24.))
            .mb(px(3.))
            .flex()
            .items_center()
            .gap(px(4.))
            .child(plain(label, 13.0, 300.0, theme.active_text))
            .child(
                no_drag_div(
                    div()
                        .id(id)
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .h(px(22.))
                        .max_w(px(max_w))
                        .px(px(7.))
                        .rounded(px(5.))
                        .border(px(1.))
                        .border_color(border)
                        .bg(bg)
                        .text_size(px(13.))
                        .font_weight(FontWeight(500.0))
                        .text_color(icon_color)
                        .on_hover(cx.listener(move |this, hovered, _, cx| {
                            this.set_hover(id, *hovered, 130.0, cx);
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(on_toggle))
                        .child(icon(icon_path, 15.0, icon_color))
                        .child(div().min_w_0().truncate().child(value)),
                )
                .into_any_element(),
            )
            .when_some(menu, |row, menu| row.child(deferred(menu)))
            .into_any_element()
    }

    fn render_branch_row(&mut self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let menu = self
            .version_open
            .then(|| self.render_version_menu(theme, cx));
        self.render_metadata_row(
            "version-trigger",
            "Branch:",
            "icons/git-branch.svg",
            self.selected_branch_label(),
            132.0,
            self.version_open,
            theme,
            cx,
            |this, _, _, cx| this.toggle_version(cx),
            menu,
        )
    }

    fn render_version_menu(&self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut menu = div()
            .absolute()
            .occlude()
            .left(px(68.))
            .top(px(28.))
            .min_w(px(132.))
            .max_w(px(196.))
            .max_h(px(116.))
            .id("version-menu")
            .overflow_y_scroll()
            .scrollbar_width(px(4.))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.menu_border())
            .bg(theme.menu_bg())
            .py(px(6.))
            .shadow(menu_shadows(&theme))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());

        let versions: Vec<(String, bool)> = self
            .versions()
            .iter()
            .map(|version| (version.tag.clone(), version.tag == self.selected_version))
            .collect();

        if versions.is_empty() {
            menu = menu.child(self.render_menu_item(
                "menu:ver:empty",
                "No builds",
                true,
                false,
                theme,
                cx,
                |_, _, cx| cx.stop_propagation(),
            ));
        } else {
            for (tag, selected) in versions {
                let menu_tag = tag.clone();
                let key = format!("menu:ver:{tag}");
                let listener = cx.listener(
                    move |this: &mut LauncherApp, _: &gpui::MouseUpEvent, _, cx| {
                        this.select_version(menu_tag.clone(), cx);
                    },
                );
                menu = menu.child(self.render_menu_item(key, tag, selected, false, theme, cx, listener));
            }
        }

        menu_fade_in(menu, "version-menu-in")
    }

    fn render_config_row(&mut self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let menu = self
            .config_open
            .then(|| self.render_config_menu(theme, cx));
        self.render_metadata_row(
            "config-trigger",
            "Config:",
            "icons/cog.svg",
            self.selected_config_name(),
            128.0,
            self.config_open,
            theme,
            cx,
            |this, _, _, cx| this.toggle_config(cx),
            menu,
        )
    }

    fn render_config_menu(&self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut menu = div()
            .absolute()
            .occlude()
            .left(px(68.))
            .top(px(28.))
            .min_w(px(132.))
            .max_w(px(164.))
            .max_h(px(116.))
            .id("config-menu")
            .overflow_y_scroll()
            .scrollbar_width(px(4.))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.menu_border())
            .bg(theme.menu_bg())
            .py(px(6.))
            .shadow(menu_shadows(&theme))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());

        for config in &self.configs {
            let name = config.name.clone();
            let entry_id = config.entry_id;
            let selected = Some(entry_id) == self.selected_config_id;
            let key = format!("menu:cfg:{entry_id}");
            let listener = cx.listener(
                move |this: &mut LauncherApp, _: &gpui::MouseUpEvent, _, cx| {
                    this.select_config(entry_id, cx);
                },
            );
            menu = menu.child(self.render_menu_item(key, name, selected, false, theme, cx, listener));
        }

        if self.configs.is_empty() {
            menu = menu.child(self.render_menu_item(
                "menu:cfg:loading",
                "Loading...",
                true,
                true,
                theme,
                cx,
                |_, _, cx| cx.stop_propagation(),
            ));
        }

        menu_fade_in(menu, "config-menu-in")
    }

    fn render_changelog(&self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let theme = *theme;
        let version_label = self.selected_version_label();
        let release_url = self.selected_release_url();
        let title_link_color = mix(theme.link, theme.link_active, self.hover_t("changelog-title-link"));

        let title = div()
            .mb(px(15.))
            .flex()
            .items_center()
            .text_size(px(13.))
            .font_weight(FontWeight(300.0))
            .line_height(px(24.))
            .text_color(theme.active_text)
            .child("Changelogs")
            .child(if release_url.is_empty() {
                div().ml(px(4.)).child(version_label).into_any_element()
            } else {
                let url = release_url.clone();
                div()
                    .id("changelog-title-link")
                    .ml(px(4.))
                    .text_color(title_link_color)
                    .on_hover(cx.listener(|this, hovered, _, cx| {
                        this.set_hover("changelog-title-link", *hovered, 130.0, cx);
                    }))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation()
                    })
                    .on_mouse_up(gpui::MouseButton::Left, move |_, _, cx| cx.open_url(&url))
                    .child(version_label)
                    .into_any_element()
            });

        let mut entries = div()
            .id("changelog")
            .flex()
            .flex_col()
            .max_h(px(154.))
            .overflow_y_scroll()
            .scrollbar_width(px(4.))
            .pr(px(8.))
            .pb(px(18.))
            .text_size(px(13.))
            .font_weight(FontWeight(300.0))
            .line_height(px(24.))
            .text_color(theme.text)
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());

        for (idx, line) in self.changelog_entries().iter().enumerate() {
            entries = entries.child(self.render_changelog_line(idx, line, &theme, cx));
        }

        let fade_mask = div()
            .absolute()
            .left_0()
            .right_0()
            .bottom_0()
            .h(px(22.))
            .bg(linear_gradient(
                180.0,
                linear_color_stop(theme.block_bg_opaque.opacity(0.0), 0.0),
                linear_color_stop(theme.block_bg_opaque, 1.0),
            ));

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .child(title)
            .child(
                div()
                    .relative()
                    .max_h(px(154.))
                    .overflow_hidden()
                    .child(entries)
                    .child(fade_mask),
            )
            .into_any_element()
    }

    fn render_launch_error(&self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let theme = *theme;
        let red = Hsla::from(rgb(0xff8d8d));
        let message = self.launch_error.clone();
        let closing = self.launch_error_closing;
        let close_t = self.hover_t("launch-error-close");
        let close_bg = mix(theme.frame_bg, theme.selection, close_t);
        let close_text = mix(theme.text, theme.active_text, close_t);
        let support_color = mix(theme.link, theme.link_active, self.hover_t("launch-error-support"));

        div()
            .relative()
            .flex_1()
            .w_full()
            .min_w_0()
            .h_full()
            .px(px(16.))
            .py(px(8.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .text_center()
            .child(
                div()
                    .flex_none()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.0))
                    .text_color(red)
                    .mb(px(6.))
                    .child("LAUNCH FAILED"),
            )
            .child(
                div()
                    .id("launch-error-message")
                    .w_full()
                    .min_w_0()
                    .max_h(px(96.))
                    .overflow_y_scroll()
                    .scrollbar_width(px(4.))
                    .px(px(8.))
                    .text_size(px(12.))
                    .font_weight(FontWeight(300.0))
                    .line_height(px(17.))
                    .text_color(red.opacity(0.85))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().w_full().whitespace_normal().child(message)),
            )
            .child(
                div()
                    .mt(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(4.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight(300.0))
                    .text_color(theme.small_text)
                    .child("Need help?")
                    .child(
                        no_drag_div(
                            div()
                                .id("launch-error-support")
                                .text_color(support_color)
                                .on_hover(cx.listener(|this, hovered, _, cx| {
                                    this.set_hover("launch-error-support", *hovered, 130.0, cx);
                                }))
                                .on_mouse_up(gpui::MouseButton::Left, |_, _, cx| {
                                    cx.open_url(DISCORD_URL);
                                })
                                .child("Contact Support"),
                        ),
                    ),
            )
            .child(
                no_drag_div(
                    div()
                        .id("launch-error-close")
                        .mt(px(10.))
                        .h(px(24.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(close_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(5.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight(300.0))
                        .text_color(close_text)
                        .on_hover(cx.listener(|this, hovered, _, cx| {
                            this.set_hover("launch-error-close", *hovered, 130.0, cx);
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.dismiss_launch_error(cx)))
                        .child(icon("icons/close.svg", 13.0, close_text))
                        .child("Dismiss"),
                )
                .into_any_element(),
            )
            .with_animation(
                if closing {
                    "launch-error-out"
                } else {
                    "launch-error-in"
                },
                Animation::new(Duration::from_millis(if closing { 160 } else { 150 }))
                    .with_easing(ease_soft()),
                move |element, delta| {
                    if closing {
                        element.top(px(3.0 * delta)).opacity(1.0 - delta)
                    } else {
                        element.top(px(3.0 * (1.0 - delta))).opacity(delta)
                    }
                },
            )
            .into_any_element()
    }

    fn render_details_footer(&mut self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let theme = *theme;
        let play_icon = icon("icons/play.svg", 18.0, theme.button_active_text);
        let chevron = icon(
            "icons/chevron-down.svg",
            19.0,
            theme.button_active_text,
        );
        let branch_t = self.hover_t("branch-trigger");
        let (branch_bg, branch_border) = if self.branch_open {
            (theme.button, transparent_black())
        } else {
            (
                theme.button_half().opacity(branch_t),
                theme.strong_border().opacity(1.0 - branch_t),
            )
        };
        let load_disabled = self.launch_pending || self.branch_open;
        let load_bg = if load_disabled {
            theme.button
        } else {
            mix(theme.button, theme.button_active, self.hover_t("load-btn"))
        };

        div()
            .absolute()
            .left(px(17.))
            .right(px(17.))
            .bottom(px(15.))
            .h(px(28.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .gap(px(15.))
                    .child(self.render_nav_link("footer-community", "Community", theme.text, theme.text_hover(), Some(DISCORD_URL), cx))
                    .child(self.render_nav_link("footer-api", "API Documentation", theme.text, theme.text_hover(), Some(API_DOCS_URL), cx)),
            )
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        no_drag_div(
                            div()
                                .id("branch-trigger")
                                .size(px(32.))
                                .h(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .border(px(1.))
                                .border_color(branch_border)
                                .bg(branch_bg)
                                .text_color(theme.button_active_text)
                                .on_hover(cx.listener(|this, hovered, _, cx| {
                                    this.set_hover("branch-trigger", *hovered, 130.0, cx);
                                }))
                                .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.toggle_branch(cx)))
                                .child(chevron),
                        )
                        .into_any_element(),
                    )
                    .child(
                        no_drag_div(
                            div()
                                .id("load-btn")
                                .w(px(102.))
                                .h(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .bg(load_bg)
                                .text_size(px(13.))
                                .font_weight(FontWeight(300.0))
                                .text_color(theme.button_active_text)
                                .when(!load_disabled, |element| {
                                    element
                                        .on_hover(cx.listener(|this, hovered, _, cx| {
                                            this.set_hover("load-btn", *hovered, 130.0, cx);
                                        }))
                                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.launch(cx)))
                                })
                                .when(load_disabled, |element| element.opacity(0.55))
                                .child(
                                    div()
                                        .mr(px(12.))
                                        .ml(px(-4.))
                                        .flex()
                                        .items_center()
                                        .child(play_icon),
                                )
                                .child("Load"),
                        )
                        .into_any_element(),
                    )
                    .when(self.branch_open, |actions| {
                        actions.child(deferred(self.render_branch_menu(&theme, cx)))
                    }),
            )
            .into_any_element()
    }

    fn render_branch_menu(&self, theme: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let menu = div()
            .absolute()
            .occlude()
            .left(px(34.))
            .top(px(-46.))
            .w(px(122.))
            .h(px(74.))
            .pt(px(8.))
            .overflow_hidden()
            .rounded(px(6.))
            .border_1()
            .border_color(theme.menu_border())
            .bg(theme.menu_bg())
            .shadow(menu_shadows(&theme))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(self.render_branch_menu_item(
                Branch::Release,
                self.branch == Branch::Release,
                theme,
                cx,
            ))
            .child(self.render_branch_menu_item(
                Branch::Nightly,
                self.branch == Branch::Nightly,
                theme,
                cx,
            ));
        menu_fade_in(menu, "branch-menu-in")
    }

    fn render_launch_content(&self, theme: &Palette) -> AnyElement {
        let icon = game_icon(46.0, 7.0, false)
            .with_animation(
                "launch-icon-pulse",
                Animation::new(Duration::from_millis(2000))
                    .repeat()
                    .with_easing(pulsating_between(0.68, 1.0)),
                |element, delta| element.opacity(delta),
            )
            .into_any_element();

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(icon)
            .child(
                div()
                    .mt(px(30.))
                    .mb(px(16.))
                    .w(px(249.))
                    .text_center()
                    .text_size(px(13.))
                    .font_weight(FontWeight(300.0))
                    .line_height(px(20.))
                    .text_color(theme.text)
                    .whitespace_nowrap()
                    .child("The game will be launched automatically"),
            )
            .child(
                div()
                    .w(px(249.))
                    .h(px(4.))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(theme.frame_bg)
                    .child(
                        div()
                            .h_full()
                            .w(px(249.0 * self.progress / 100.0))
                            .rounded_full()
                            .bg(theme.button),
                    ),
            )
            .with_animation(
                "launch-content-in",
                Animation::new(Duration::from_millis(520)),
                |element, delta| {
                    element.opacity(delayed(delta, 260.0, 520.0, ease_soft()))
                },
            )
            .into_any_element()
    }

    fn render_profile_popout(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme.clone();
        let closing = self.profile_closing;
        let avatar_t = self.hover_t("profile-avatar");
        let avatar_slide_t = self.hover_t("profile-avatar-slide");
        let cancel_color = mix(theme.text, theme.text_hover(), self.hover_t("profile-cancel"));
        let save_bg = mix(theme.button, theme.button_active, self.hover_t("profile-save"));
        let close_color = mix(theme.text, theme.active_text, self.hover_t("profile-close"));

        let content = div()
            .absolute()
            .left(px(20.))
            .right(px(20.))
            .top(px(46.))
            .bottom(px(17.))
            .child(
                no_drag_div(
                    div()
                        .id("profile-avatar")
                        .relative()
                        .mx_auto()
                        .size(px(84.))
                        .rounded_full()
                        .overflow_hidden()
                        .shadow(vec![BoxShadow {
                            color: black().opacity(0.4),
                            offset: gpui::point(px(0.), px(8.)),
                            blur_radius: px(14.),
                            spread_radius: px(0.),
                        }])
                        .on_hover(cx.listener(|this, hovered, _, cx| {
                            this.set_hover("profile-avatar", *hovered, 150.0, cx);
                            this.set_hover("profile-avatar-slide", *hovered, 170.0, cx);
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.choose_avatar(cx)))
                        .child(self.render_avatar(84.0))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full()
                                .bg(black().opacity(0.28 * avatar_t)),
                        )
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom(px(-24.0 * (1.0 - avatar_slide_t)))
                                .h(px(24.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(black().opacity(0.72))
                                .text_size(px(10.))
                                .font_weight(FontWeight(500.0))
                                .text_color(white())
                                .child(if self.avatar_loading {
                                    "Loading..."
                                } else {
                                    "Change"
                                }),
                        ),
                )
                .into_any_element(),
            )
            .child(
                div()
                    .mt(px(12.))
                    .flex()
                    .justify_center()
                    .child(self.render_profile_input(window, cx)),
            )
            .when(!self.profile_error.is_empty(), |element| {
                element.child(
                    div()
                        .mt(px(7.))
                        .text_center()
                        .text_size(px(11.))
                        .line_height(px(15.))
                        .text_color(rgb(0xff8d8d))
                        .child(self.profile_error.clone()),
                )
            })
            .when(self.pending_avatar_bytes.is_some(), |element| {
                element.child(
                    div()
                        .mt(px(10.))
                        .flex()
                        .justify_center()
                        .gap(px(8.))
                        .child(
                            no_drag_div(
                                div()
                                    .id("profile-cancel")
                                    .h(px(28.))
                                    .min_w(px(62.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(300.0))
                                    .text_color(cancel_color)
                                    .on_hover(cx.listener(|this, hovered, _, cx| {
                                        this.set_hover("profile-cancel", *hovered, 130.0, cx);
                                    }))
                                    .when(self.profile_saving, |element| element.opacity(0.55))
                                    .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.cancel_avatar_edit(cx)
                                    }))
                                    .child("Cancel"),
                            )
                            .into_any_element(),
                        )
                        .child(
                            no_drag_div(
                                div()
                                    .id("profile-save")
                                    .h(px(28.))
                                    .min_w(px(62.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.))
                                    .bg(save_bg)
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(300.0))
                                    .text_color(theme.button_active_text)
                                    .on_hover(cx.listener(|this, hovered, _, cx| {
                                        this.set_hover("profile-save", *hovered, 130.0, cx);
                                    }))
                                    .when(self.profile_saving, |element| element.opacity(0.55))
                                    .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.save_profile(cx)))
                                    .child(if self.profile_saving {
                                        "Saving..."
                                    } else {
                                        "Save"
                                    }),
                            )
                            .into_any_element(),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(px(2.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .font_weight(FontWeight(300.0))
                    .text_color(theme.small_text)
                    .child(icon("icons/construction.svg", 15.0, theme.small_text))
                    .child("Work In Progress"),
            );

        div()
            .absolute()
            .left(px(35.))
            .top(px(30.))
            .w(px(460.))
            .h(px(340.))
            .overflow_hidden()
            .rounded(px(10.))
            .border_1()
            .border_color(theme.white_border())
            .bg(theme.block_bg_opaque)
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .shadow(vec![BoxShadow {
                color: theme.shadow_soft,
                offset: gpui::point(px(0.), px(14.)),
                blur_radius: px(21.),
                spread_radius: px(0.),
            }])
            .child(
                no_drag_div(
                    div()
                        .id("profile-close")
                        .absolute()
                        .right(px(15.))
                        .top(px(18.))
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_hover(cx.listener(|this, hovered, _, cx| {
                            this.set_hover("profile-close", *hovered, 120.0, cx);
                        }))
                        .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| this.close_menus(cx)))
                        .child(icon("icons/close.svg", 19.0, close_color)),
                )
                .into_any_element(),
            )
            .child(content)
            .with_animation(
                if closing {
                    "profile-popout-out"
                } else {
                    "profile-popout-in"
                },
                Animation::new(Duration::from_millis(220)).with_easing(ease_in_out),
                move |element, delta| {
                    if closing {
                        element.top(px(30.0 + delta * 1.7)).opacity(1.0 - delta)
                    } else {
                        element
                            .top(px(30.0 + (1.0 - delta) * 1.7))
                            .opacity(delta)
                    }
                },
            )
            .into_any_element()
    }

    fn render_profile_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme.clone();
        let focused = self.profile_focus.is_focused(window);

        let mut font = gpui::font(FONT_FAMILY);
        font.weight = FontWeight(500.0);
        let run = TextRun {
            len: self.profile_name.len(),
            font,
            color: theme.active_text,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = window.text_system().shape_line(
            SharedString::from(self.profile_name.clone()),
            px(15.),
            &[run],
            None,
        );
        let text_width = shaped.width;
        let cursor_offset = shaped.x_for_index(self.profile_cursor);
        let cursor_x = px(89.0) - text_width * 0.5 + cursor_offset;

        let hover_bg = mix(theme.main_bg_opaque, white(), 0.04);
        let hover_border = mix(theme.link, white(), 0.18);
        let input_t = if focused {
            1.0
        } else {
            self.hover_t("profile-input")
        };

        let input = div()
            .id("profile-input")
            .relative()
            .w(px(178.))
            .h(px(32.))
            .overflow_hidden()
            .rounded(px(5.))
            .border_1()
            .bg(hover_bg.opacity(input_t))
            .border_color(hover_border.opacity(input_t))
            .track_focus(&self.profile_focus)
            .cursor(gpui::CursorStyle::IBeam)
            .on_hover(cx.listener(|this, hovered, _, cx| {
                this.set_hover("profile-input", *hovered, 130.0, cx);
            }))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.profile_focus);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(15.))
                    .font_weight(FontWeight(500.0))
                    .line_height(px(20.))
                    .text_color(theme.active_text)
                    .child(self.profile_name.clone()),
            )
            .when(focused, |element| {
                element.child(
                    div()
                        .absolute()
                        .left(cursor_x)
                        .top(px(8.))
                        .w(px(1.5))
                        .h(px(16.))
                        .bg(theme.active_text)
                        .with_animation(
                            "profile-caret",
                            Animation::new(Duration::from_millis(1000)).repeat(),
                            |caret, delta| {
                                if delta < 0.5 {
                                    caret.opacity(1.0)
                                } else {
                                    caret.opacity(0.0)
                                }
                            },
                        ),
                )
            });

        no_drag_div(input).into_any_element()
    }

    fn render_avatar(&self, size: f32) -> AnyElement {
        let source = self
            .avatar
            .clone()
            .map(ImageSource::Render)
            .unwrap_or_else(|| ImageSource::from("images/avatar-placeholder.svg"));

        div()
            .flex_none()
            .size(px(size))
            .rounded_full()
            .overflow_hidden()
            .border_1()
            .border_color(white().opacity(0.18))
            .bg(rgb(0x4b4d57))
            .opacity(0.9)
            .child(
                img(source)
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
            .into_any_element()
    }

    fn render_nav_link(
        &self,
        id: &'static str,
        label: &'static str,
        color: Hsla,
        hover: Hsla,
        url: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let text_color = mix(color, hover, self.hover_t(id));
        let item = div()
            .id(id)
            .text_size(px(13.))
            .font_weight(FontWeight(300.0))
            .line_height(px(18.))
            .text_color(text_color)
            .on_hover(cx.listener(move |this, hovered, _, cx| {
                this.set_hover(id, *hovered, 130.0, cx);
            }))
            .child(label);

        if let Some(url) = url {
            no_drag_div(item.on_mouse_up(gpui::MouseButton::Left, move |_, _, cx| cx.open_url(url)))
                .into_any_element()
        } else {
            item.into_any_element()
        }
    }

    fn render_changelog_line(
        &self,
        line_idx: usize,
        line: &str,
        theme: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = *theme;
        let segments = parse_changelog_line(line);
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .min_h(px(20.))
            .text_size(px(13.))
            .font_weight(FontWeight(300.0))
            .line_height(px(24.))
            .text_color(theme.text);

        for (seg_idx, segment) in segments.into_iter().enumerate() {
            row = match segment {
                ChangelogSegment::Text(value) => row.child(value),
                ChangelogSegment::Link { label, url } => {
                    let key: SharedString = format!("cl-link:{line_idx}:{seg_idx}").into();
                    let hover_key = key.clone();
                    let link_color = mix(theme.link, theme.link_active, self.hover_t(&key));
                    let link_url = url.clone();
                    row.child(
                        div()
                            .id(key)
                            .text_color(link_color)
                            .on_hover(cx.listener(move |this, hovered, _, cx| {
                                this.set_hover(hover_key.clone(), *hovered, 130.0, cx);
                            }))
                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation()
                            })
                            .on_mouse_up(gpui::MouseButton::Left, move |_, _, cx| cx.open_url(&link_url))
                            .child(label),
                    )
                }
            };
        }

        row.into_any_element()
    }

    fn render_menu_item(
        &self,
        key: impl Into<SharedString>,
        label: impl Into<SharedString>,
        selected: bool,
        disabled: bool,
        theme: &Palette,
        cx: &mut Context<Self>,
        on_click: impl Fn(&gpui::MouseUpEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> AnyElement {
        let theme = *theme;
        let key: SharedString = key.into();
        let hover_key = key.clone();
        let label = label.into();
        let t = if disabled { 0.0 } else { self.hover_t(&key) };
        let base_text = if selected {
            theme.active_text
        } else {
            theme.text_preview
        };
        let hover_text = if selected {
            theme.active_text
        } else {
            mix(theme.active_text, theme.text_preview, 0.5)
        };

        let mut item = div()
            .id(key)
            .relative()
            .h(px(26.))
            .w_full()
            .px(px(11.))
            .flex()
            .items_center()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(13.))
            .font_weight(FontWeight(300.0))
            .text_color(mix(base_text, hover_text, t));

        if disabled {
            item = item.opacity(0.58);
        } else {
            item = item
                .on_hover(cx.listener(move |this, hovered, _, cx| {
                    this.set_hover(hover_key.clone(), *hovered, 130.0, cx);
                }))
                .on_mouse_up(gpui::MouseButton::Left, on_click);
        }

        item = item
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(8.))
                    .right(px(8.))
                    .rounded(px(5.))
                    .bg(theme.selection)
                    .opacity(0.5 * t),
            )
            .child(div().min_w_0().flex_1().truncate().child(label));

        no_drag_div(item).into_any_element()
    }

    fn render_branch_menu_item(
        &self,
        branch: Branch,
        selected: bool,
        theme: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = *theme;
        let key: &'static str = match branch {
            Branch::Release => "menu:branch:release",
            Branch::Nightly => "menu:branch:nightly",
        };
        let t = self.hover_t(key);
        let text_color = if selected {
            theme.active_text
        } else {
            mix(theme.text_preview, theme.active_text, t)
        };
        no_drag_div(
            div()
                .id(key)
                .relative()
                .h(px(29.))
                .w_full()
                .flex()
                .items_center()
                .gap(px(11.))
                .pl(px(14.))
                .text_size(px(14.))
                .font_weight(FontWeight(100.0))
                .text_color(text_color)
                .on_hover(cx.listener(move |this, hovered, _, cx| {
                    this.set_hover(key, *hovered, 130.0, cx);
                }))
                .on_mouse_up(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| this.select_branch(branch, cx)))
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(8.))
                        .right(px(8.))
                        .rounded(px(5.))
                        .bg(theme.selection)
                        .opacity(0.5 * t),
                )
                .child(
                    div()
                        .w(px(24.))
                        .flex()
                        .justify_center()
                        .child(icon("icons/git-branch.svg", 17.0, text_color)),
                )
                .child(branch.label()),
        )
        .into_any_element()
    }
}

fn shell_shadows(theme: &Palette) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: theme.shadow.opacity(0.32),
            offset: gpui::point(px(0.), px(0.)),
            blur_radius: px(8.),
            spread_radius: px(0.),
        },
        BoxShadow {
            color: black().opacity(0.26),
            offset: gpui::point(px(0.), px(6.)),
            blur_radius: px(10.),
            spread_radius: px(0.),
        },
    ]
}

fn game_icon(size: f32, radius: f32, grayscale: bool) -> Div {
    div()
        .flex_none()
        .size(px(size))
        .rounded(px(radius))
        .overflow_hidden()
        .shadow(vec![BoxShadow {
            color: Hsla::from(rgb(0xf08b0b)).opacity(0.2),
            offset: gpui::point(px(0.), px(0.)),
            blur_radius: px(9.),
            spread_radius: px(0.),
        }])
        .child(
            img("images/csgo.png")
                .size_full()
                .object_fit(ObjectFit::Cover)
                .grayscale(grayscale),
        )
}

/// Delays an animation curve by `delay_ms` within `total_ms`.
fn delayed(delta: f32, delay_ms: f32, total_ms: f32, easing: impl Fn(f32) -> f32) -> f32 {
    let start = delay_ms / total_ms;
    if delta <= start {
        0.0
    } else {
        easing(((delta - start) / (1.0 - start)).clamp(0.0, 1.0))
    }
}

fn menu_shadows(theme: &Palette) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: theme.button_active.opacity(0.08),
            offset: gpui::point(px(0.), px(0.)),
            blur_radius: px(7.),
            spread_radius: px(0.),
        },
        BoxShadow {
            color: theme.shadow_soft,
            offset: gpui::point(px(0.), px(10.)),
            blur_radius: px(15.),
            spread_radius: px(0.),
        },
    ]
}

fn menu_fade_in<S>(menu: S, id: &'static str) -> AnyElement
where
    S: gpui::Styled + IntoElement + 'static,
{
    menu.with_animation(
        id,
        Animation::new(Duration::from_millis(190)).with_easing(ease_in_out),
        |element, delta| element.opacity(delta),
    )
    .into_any_element()
}

/// CSS-style cubic-bezier easing.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    fn eval(t: f32, p1: f32, p2: f32) -> f32 {
        let it = 1.0 - t;
        3.0 * it * it * t * p1 + 3.0 * it * t * t * p2 + t * t * t
    }
    move |t| {
        let mut lo = 0.0_f32;
        let mut hi = 1.0_f32;
        let mut u = t;
        for _ in 0..20 {
            let x = eval(u, x1, x2);
            if (x - t).abs() < 1e-5 {
                break;
            }
            if x < t {
                lo = u;
            } else {
                hi = u;
            }
            u = (lo + hi) / 2.0;
        }
        eval(u, y1, y2)
    }
}

/// `cubic-bezier(.22, 1, .36, 1)`
fn ease_smooth() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.22, 1.0, 0.36, 1.0)
}

/// `cubic-bezier(.25, .1, .25, 1)`
fn ease_soft() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.0)
}

/// GPUI's `start_window_move` is a no-op on Windows; send `HTCAPTION` instead.
#[cfg(windows)]
fn start_window_drag(window: &mut Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, POINT, WPARAM},
        UI::{
            Input::KeyboardAndMouse::ReleaseCapture,
            WindowsAndMessaging::{GetCursorPos, SendMessageW, HTCAPTION, WM_NCLBUTTONDOWN},
        },
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };

    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point);
        ReleaseCapture();
        SendMessageW(
            win32.hwnd.get() as HWND,
            WM_NCLBUTTONDOWN,
            HTCAPTION as WPARAM,
            ((point.x & 0xFFFF) | (point.y << 16)) as LPARAM,
        );
    }
}

#[cfg(not(windows))]
fn start_window_drag(window: &mut Window) {
    window.start_window_move();
}





