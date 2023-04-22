use crate::dynamic_string::DynamicString;
use crate::script::{Script, ScriptInput};
use crate::send;
use gtk::gdk::ScrollDirection;
use gtk::prelude::*;
use gtk::EventBox;
use serde::Deserialize;
use tokio::spawn;
use tracing::trace;

/// Common configuration options
/// which can be set on every module.
#[derive(Debug, Deserialize, Clone)]
pub struct CommonConfig {
    pub show_if: Option<ScriptInput>,

    pub on_click_left: Option<ScriptInput>,
    pub on_click_right: Option<ScriptInput>,
    pub on_click_middle: Option<ScriptInput>,
    pub on_scroll_up: Option<ScriptInput>,
    pub on_scroll_down: Option<ScriptInput>,
    pub on_mouse_enter: Option<ScriptInput>,
    pub on_mouse_exit: Option<ScriptInput>,

    pub tooltip: Option<String>,
}

impl CommonConfig {
    /// Configures the module's container according to the common config options.
    pub fn install(mut self, container: &EventBox) {
        self.install_show_if(container);

        let left_click_script = self.on_click_left.map(Script::new_polling);
        let middle_click_script = self.on_click_middle.map(Script::new_polling);
        let right_click_script = self.on_click_right.map(Script::new_polling);

        container.connect_button_press_event(move |_, event| {
            let script = match event.button() {
                1 => left_click_script.as_ref(),
                2 => middle_click_script.as_ref(),
                3 => right_click_script.as_ref(),
                _ => None,
            };

            if let Some(script) = script {
                trace!("Running on-click script: {}", event.button());
                script.run_as_oneshot(None);
            }

            Inhibit(false)
        });

        let scroll_up_script = self.on_scroll_up.map(Script::new_polling);
        let scroll_down_script = self.on_scroll_down.map(Script::new_polling);

        container.connect_scroll_event(move |_, event| {
            let script = match event.direction() {
                ScrollDirection::Up => scroll_up_script.as_ref(),
                ScrollDirection::Down => scroll_down_script.as_ref(),
                _ => None,
            };

            if let Some(script) = script {
                trace!("Running on-scroll script: {}", event.direction());
                script.run_as_oneshot(None);
            }

            Inhibit(false)
        });

        macro_rules! install_oneshot {
            ($option:expr, $method:ident) => {
                $option.map(Script::new_polling).map(|script| {
                    container.$method(move |_, _| {
                        script.run_as_oneshot(None);
                        Inhibit(false)
                    });
                })
            };
        }

        install_oneshot!(self.on_mouse_enter, connect_enter_notify_event);
        install_oneshot!(self.on_mouse_exit, connect_leave_notify_event);

        if let Some(tooltip) = self.tooltip {
            let container = container.clone();
            DynamicString::new(&tooltip, move |string| {
                container.set_tooltip_text(Some(&string));
                Continue(true)
            });
        }
    }

    fn install_show_if(&mut self, container: &EventBox) {
        self.show_if.take().map_or_else(
            || {
                container.show_all();
            },
            |show_if| {
                let script = Script::new_polling(show_if);
                let container = container.clone();
                let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
                spawn(async move {
                    script
                        .run(None, |_, success| {
                            send!(tx, success);
                        })
                        .await;
                });
                rx.attach(None, move |success| {
                    if success {
                        container.show_all();
                    } else {
                        container.hide();
                    };
                    Continue(true)
                });
            },
        );
    }
}