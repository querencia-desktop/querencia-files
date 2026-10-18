// SPDX-License-Identifier: MIT

//! Classic window chrome in the GNOME 2 / Caja arrangement: a menu bar, a
//! navigation toolbar, a location row and a status bar. Every control routes
//! to an existing browser command; nothing here owns navigation state.

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use gtk::{gio, glib, prelude::*};

use crate::{
    app::Browser,
    assets::{self, icons},
    model::{Location, SortDirection, SortKey},
    ui::{
        browser::{BrowserView, PinStatus, clipboard_may_paste, network, paths::is_trash_location},
        browser_modes::BrowserMode,
        keys::{Binding, Keys, bindings::*},
        preferences::PreferenceManager,
        shortcut_footer::ShortcutFooter,
    },
};

use super::{
    super::{SidebarView, apply_browser_mode, home_directory, pin_status},
    layout::Header,
};

/// Prefix of the per-window action group the menus and toolbar activate.
const ACTIONS: &str = "classic";

/// How long "Connect to Server ▸ Add bookmark" waits for the connection.
const PENDING_BOOKMARK_TIMEOUT: Duration = Duration::from_secs(300);

/// A bookmark requested in the Connect to Server dialog; it is only written
/// once the browser shows the connected location, never for a failed attempt.
struct PendingBookmark {
    location: Location,
    name: String,
    until: Instant,
    pin: Rc<dyn Fn(Location, String)>,
}

pub(super) struct ClassicChrome {
    /// Menu bar, toolbar and location row; also the keyboard's "top bar".
    pub(super) chrome: gtk::Box,
    pub(super) status_bar: gtk::Box,
    menu_bar: gtk::PopoverMenuBar,
    status: gtk::Label,
    actions: gio::SimpleActionGroup,
    bookmarks: gio::Menu,
    rendered_bookmarks: RefCell<Vec<(Location, String)>>,
    pinned_places: Rc<RefCell<Vec<(Location, String)>>>,
    pending_bookmark: RefCell<Option<PendingBookmark>>,
    /// For the clipboard watcher, which outlives no window.
    browser: Weak<Browser>,
    clipboard_watch: RefCell<Option<(gtk::gdk::Clipboard, glib::SignalHandlerId)>>,
}

impl Drop for ClassicChrome {
    fn drop(&mut self) {
        if let Some((clipboard, handler)) = self.clipboard_watch.take() {
            clipboard.disconnect(handler);
        }
    }
}

impl ClassicChrome {
    pub(super) fn new(
        window: &gtk::ApplicationWindow,
        root: &gtk::Box,
        view: &BrowserView,
        sidebar: &SidebarView,
        header: &Header,
        preferences: &Rc<PreferenceManager>,
    ) -> Rc<Self> {
        let actions = gio::SimpleActionGroup::new();
        root.insert_action_group(ACTIONS, Some(&actions));
        let bookmarks = gio::Menu::new();
        let menu_bar = gtk::PopoverMenuBar::from_model(Some(&menu_model(&bookmarks)));
        menu_bar.add_css_class("classic-menu-bar");
        localize_accelerators(menu_bar.upcast_ref());
        let toolbar = build_toolbar();
        // Density, type grouping and the preview panel are only offered by
        // Strata's appearance menu, so it moves from the hidden header here.
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        toolbar.append(&spacer);
        if let Some(parent) = header.appearance.parent().and_downcast::<gtk::Box>() {
            parent.remove(&header.appearance);
        }
        toolbar.append(&header.appearance);
        let location_row = build_location_row(view);
        let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        status_bar.add_css_class("classic-status-bar");
        let status = gtk::Label::new(None);
        status.set_xalign(0.0);
        status.set_hexpand(true);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        status_bar.append(&status);
        let chrome_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        chrome_box.add_css_class("classic-chrome");
        chrome_box.append(&menu_bar);
        chrome_box.append(&toolbar);
        chrome_box.append(&location_row);
        crate::ui::tenxer_mode::hide_while_enabled(&chrome_box);
        crate::ui::tenxer_mode::hide_while_enabled(&status_bar);
        let chrome = Rc::new(Self {
            chrome: chrome_box,
            status_bar,
            menu_bar,
            status,
            actions,
            bookmarks,
            rendered_bookmarks: RefCell::new(Vec::new()),
            pinned_places: sidebar.state.pinned_places.clone(),
            pending_bookmark: RefCell::new(None),
            browser: Rc::downgrade(&view.browser()),
            clipboard_watch: RefCell::new(None),
        });
        chrome.install_actions(window, view, sidebar, header, preferences);
        chrome.observe(view);
        chrome.watch_clipboard();
        chrome
    }

    /// Help ▸ About opens the Settings page; the launcher is created later.
    pub(super) fn bind_about(&self, open: impl Fn() + 'static) {
        let action = gio::SimpleAction::new("about", None);
        action.connect_activate(move |_, _| open());
        self.actions.add_action(&action);
    }

    /// The shortcut reference lives in the footer, which exists only after the
    /// layout is assembled.
    pub(super) fn bind_footer(&self, shortcuts: &ShortcutFooter) {
        let action = gio::SimpleAction::new("keyboard-shortcuts", None);
        let shortcuts = shortcuts.clone();
        action.connect_activate(move |_, _| shortcuts.toggle_reference());
        self.actions.add_action(&action);
    }

    fn install_actions(
        self: &Rc<Self>,
        window: &gtk::ApplicationWindow,
        view: &BrowserView,
        sidebar: &SidebarView,
        header: &Header,
        preferences: &Rc<PreferenceManager>,
    ) {
        let browser = view.browser();
        self.add_view_action(window, view, "back", {
            let browser = Rc::downgrade(&browser);
            move |_| {
                if let Some(browser) = browser.upgrade()
                    && browser.can_go_back()
                {
                    browser.back();
                }
            }
        });
        self.add_view_action(window, view, "forward", {
            let browser = Rc::downgrade(&browser);
            move |_| {
                if let Some(browser) = browser.upgrade()
                    && browser.can_go_forward()
                {
                    browser.forward();
                }
            }
        });
        self.add_view_action(window, view, "up", |view| view.navigate_up());
        self.add_view_action(window, view, "home", |view| {
            view.navigate_location(Location::local(home_directory()));
        });
        self.add_view_action(window, view, "trash", |view| {
            view.navigate_location(Location::uri("trash:///"));
        });
        self.add_view_action(window, view, "network", |view| {
            view.navigate_location(Location::uri(network::NETWORK_URI));
        });
        self.add_view_action(window, view, "connect-to-server", {
            let state = Rc::downgrade(&sidebar.state);
            let chrome = Rc::downgrade(self);
            move |view| {
                let connect_view = view.clone();
                let state = state.clone();
                let chrome = chrome.clone();
                network::show_connect_dialog(&view.widget(), move |address, bookmark| {
                    let Some(location) = connect_view.connect_to_server(&address) else {
                        return;
                    };
                    let Some(chrome) = chrome.upgrade() else {
                        return;
                    };
                    let state = state.clone();
                    chrome
                        .pending_bookmark
                        .replace(bookmark.map(|name| PendingBookmark {
                            name: if name.is_empty() {
                                location.display_name()
                            } else {
                                name
                            },
                            location,
                            until: Instant::now() + PENDING_BOOKMARK_TIMEOUT,
                            pin: Rc::new(move |location, name| {
                                if let Some(state) = state.upgrade() {
                                    state.pin_location(location, name);
                                }
                            }),
                        }));
                    if let Some(browser) = chrome.browser.upgrade() {
                        chrome.refresh(&browser);
                    }
                });
            }
        });
        self.add_view_action(window, view, "reload", |view| {
            if !view.reconnect_if_disconnected() {
                view.refresh();
            }
        });
        self.add_view_action(window, view, "edit-location", |view| {
            view.begin_location_edit();
        });
        self.add_view_action(window, view, "new-folder", |view| view.create_new_folder());
        self.add_view_action(window, view, "properties", |view| {
            if !view.show_focused_properties()
                && let Some(location) = view.browser().active_location()
            {
                view.show_location_properties(&location);
            }
        });
        self.add_view_action(window, view, "undo", |view| {
            view.undo_last_operation();
        });
        self.add_view_action(window, view, "redo", |view| {
            view.redo_last_operation();
        });
        self.add_view_action(window, view, "cut", |view| {
            view.cut_selection();
        });
        self.add_view_action(window, view, "copy", |view| {
            view.copy_selection();
        });
        self.add_view_action(window, view, "paste", |view| view.paste());
        self.add_view_action(window, view, "duplicate", |view| {
            view.duplicate_selection();
        });
        self.add_view_action(window, view, "select-all", |view| view.select_all());
        self.add_view_action(window, view, "rename", |view| {
            view.begin_rename();
        });
        self.add_view_action(window, view, "trash-selection", |view| {
            view.confirm_delete(false);
        });
        self.add_view_action(window, view, "delete-selection", |view| {
            view.confirm_delete(true);
        });
        self.add_view_action(window, view, "settings", {
            let settings = header.settings.clone();
            move |_| settings.emit_clicked()
        });
        self.add_view_action(window, view, "close-window", {
            let window = window.downgrade();
            move |_| {
                if let Some(window) = window.upgrade() {
                    window.close();
                }
            }
        });
        self.add_view_action(window, view, "add-bookmark", {
            let state = Rc::downgrade(&sidebar.state);
            let places = self.pinned_places.clone();
            let chrome = Rc::downgrade(self);
            move |view| {
                let Some(location) = view.browser().active_location() else {
                    return;
                };
                if pin_status(&places.borrow(), &location) != PinStatus::Available {
                    return;
                }
                if let Some(state) = state.upgrade() {
                    let name = location.display_name();
                    state.pin_location(location, name);
                }
                if let Some(chrome) = chrome.upgrade() {
                    chrome.refresh_bookmarks();
                }
            }
        });
        let open_bookmark = gio::SimpleAction::new("open-bookmark", Some(glib::VariantTy::UINT32));
        {
            let view = view.clone();
            let window = window.downgrade();
            let places = self.pinned_places.clone();
            open_bookmark.connect_activate(move |_, parameter| {
                let Some(index) = parameter.and_then(glib::Variant::get::<u32>) else {
                    return;
                };
                if !action_allowed(&window, &view) {
                    return;
                }
                let location = places
                    .borrow()
                    .get(index as usize)
                    .map(|(location, _)| location.clone());
                if let Some(location) = location {
                    view.navigate_location(location);
                }
            });
        }
        self.actions.add_action(&open_bookmark);

        let show_hidden = gio::SimpleAction::new_stateful(
            "show-hidden",
            None,
            &browser.preferences().show_hidden.to_variant(),
        );
        {
            let view = view.clone();
            let window = window.downgrade();
            show_hidden.connect_activate(move |_, _| {
                if action_allowed(&window, &view) {
                    view.browser().toggle_hidden();
                }
            });
        }
        {
            // Weak: the browser keeps preference observers for its lifetime, and
            // the action's closure holds the view, which holds the browser.
            let action = show_hidden.downgrade();
            browser.observe_preferences(move |preferences| {
                if let Some(action) = action.upgrade() {
                    action.set_state(&preferences.show_hidden.to_variant());
                }
            });
        }
        self.actions.add_action(&show_hidden);

        let view_mode = gio::SimpleAction::new_stateful(
            "view-mode",
            Some(glib::VariantTy::STRING),
            &mode_name(view.view_mode()).to_variant(),
        );
        {
            let view = view.clone();
            let window = window.downgrade();
            let preferences = preferences.clone();
            view_mode.connect_activate(move |_, parameter| {
                let Some(mode) = parameter
                    .and_then(|value| value.str())
                    .and_then(mode_for_name)
                else {
                    return;
                };
                if action_allowed(&window, &view) {
                    apply_browser_mode(&view, &preferences, mode);
                }
            });
        }
        {
            let action = view_mode.downgrade();
            view.connect_view_mode_changed(move |mode| {
                if let Some(action) = action.upgrade() {
                    action.set_state(&mode_name(mode).to_variant());
                }
            });
        }
        self.actions.add_action(&view_mode);

        // View ▸ Arrange Items: the active folder's sort, as the pane
        // header's sort menu sets it.
        let arrange = gio::SimpleAction::new_stateful(
            "arrange",
            Some(glib::VariantTy::STRING),
            &"name".to_variant(),
        );
        {
            let view = view.clone();
            let window = window.downgrade();
            arrange.connect_activate(move |_, parameter| {
                let Some(key) = parameter
                    .and_then(|value| value.str())
                    .and_then(SortKey::from_stored)
                else {
                    return;
                };
                let browser = view.browser();
                if action_allowed(&window, &view)
                    && let Some(depth) = browser.active_depth()
                {
                    browser.set_sort_key(depth, key);
                }
            });
        }
        self.actions.add_action(&arrange);
        let reversed =
            gio::SimpleAction::new_stateful("arrange-reversed", None, &false.to_variant());
        {
            let view = view.clone();
            let window = window.downgrade();
            reversed.connect_activate(move |_, _| {
                let browser = view.browser();
                if !action_allowed(&window, &view) {
                    return;
                }
                let Some(depth) = browser.active_depth() else {
                    return;
                };
                let Some(preferences) = browser.column_preferences(depth) else {
                    return;
                };
                browser.set_sort_direction(
                    depth,
                    match preferences.sort_direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    },
                );
            });
        }
        self.actions.add_action(&reversed);

        let side_pane = gio::SimpleAction::new_stateful(
            "side-pane",
            None,
            &preferences.sidebar_expanded().to_variant(),
        );
        {
            let preferences = preferences.clone();
            side_pane.connect_activate(move |_, _| {
                preferences.set_sidebar_expanded(!preferences.sidebar_expanded());
            });
        }
        {
            let action = side_pane.downgrade();
            preferences.bind_preference(
                &self.menu_bar,
                PreferenceManager::sidebar_expanded,
                move |_, expanded| {
                    if let Some(action) = action.upgrade() {
                        action.set_state(&expanded.to_variant());
                    }
                },
            );
        }
        self.actions.add_action(&side_pane);
        self.refresh(&browser);
    }

    /// Adds a `classic.<name>` action that, like the `win.*` actions, stays
    /// inert while a modal layer or an inline edit owns input.
    fn add_view_action(
        &self,
        window: &gtk::ApplicationWindow,
        view: &BrowserView,
        name: &str,
        run: impl Fn(&BrowserView) + 'static,
    ) {
        let action = gio::SimpleAction::new(name, None);
        let window = window.downgrade();
        let view = view.clone();
        action.connect_activate(move |_, _| {
            if action_allowed(&window, &view) {
                run(&view);
            }
        });
        self.actions.add_action(&action);
    }

    /// The observer owns the chrome: it lives as long as the browser's
    /// observer list, which the window clears on dispose.
    fn observe(self: &Rc<Self>, view: &BrowserView) {
        let browser = view.browser();
        let chrome = self.clone();
        let weak_browser = Rc::downgrade(&browser);
        browser.observe(move |_| {
            if let Some(browser) = weak_browser.upgrade() {
                chrome.refresh(&browser);
            }
        });
    }

    fn refresh(&self, browser: &Browser) {
        self.set_enabled("back", browser.can_go_back());
        self.set_enabled("forward", browser.can_go_forward());
        self.set_enabled("up", browser.can_go_parent());
        let active = browser.active_location();
        let pinnable = active.as_ref().is_some_and(|location| {
            pin_status(&self.pinned_places.borrow(), location) == PinStatus::Available
        });
        self.set_enabled("add-bookmark", pinnable);
        // Nautilus 2 greys out what has nothing to act on.
        let selected = browser.selected_count() > 0;
        for name in [
            "cut",
            "copy",
            "duplicate",
            "rename",
            "trash-selection",
            "delete-selection",
        ] {
            self.set_enabled(name, selected);
        }
        self.refresh_paste(browser);
        if let Some(depth) = browser.active_depth()
            && let Some(preferences) = browser.column_preferences(depth)
        {
            let key = preferences.sort_key.stored_name();
            self.set_enabled("arrange", key.is_some());
            self.set_enabled("arrange-reversed", key.is_some());
            self.set_state("arrange", &key.unwrap_or("name").to_variant());
            self.set_state(
                "arrange-reversed",
                &(preferences.sort_direction == SortDirection::Descending).to_variant(),
            );
        }
        self.complete_pending_bookmark(browser);
        self.refresh_bookmarks();
        let text = status_text(browser).unwrap_or_default();
        if self.status.text() != text {
            self.status.set_text(&text);
        }
    }

    fn complete_pending_bookmark(&self, browser: &Browser) {
        let due = {
            let mut pending = self.pending_bookmark.borrow_mut();
            let expired = pending
                .as_ref()
                .is_some_and(|bookmark| Instant::now() > bookmark.until);
            let reached = pending.as_ref().is_some_and(|bookmark| {
                browser
                    .active_location()
                    .is_some_and(|active| network::same_location(&active, &bookmark.location))
            });
            if expired {
                pending.take();
            }
            if reached { pending.take() } else { None }
        };
        if let Some(bookmark) = due {
            (bookmark.pin)(bookmark.location, bookmark.name);
        }
    }

    fn refresh_paste(&self, browser: &Browser) {
        let into_folder = browser.active_location().is_some_and(|location| {
            !is_trash_location(&location) && !location.is_recent_location()
        });
        self.set_enabled("paste", into_folder && clipboard_may_paste());
    }

    /// Paste follows the clipboard, which changes without a browser event.
    fn watch_clipboard(self: &Rc<Self>) {
        let Some(display) = gtk::gdk::Display::default() else {
            return;
        };
        let clipboard = display.clipboard();
        let chrome = Rc::downgrade(self);
        let handler = clipboard.connect_changed(move |_| {
            if let Some(chrome) = chrome.upgrade()
                && let Some(browser) = chrome.browser.upgrade()
            {
                chrome.refresh_paste(&browser);
            }
        });
        self.clipboard_watch.replace(Some((clipboard, handler)));
    }

    fn set_state(&self, name: &str, state: &glib::Variant) {
        if let Some(action) = self
            .actions
            .lookup_action(name)
            .and_downcast::<gio::SimpleAction>()
            && action.state().as_ref() != Some(state)
        {
            action.set_state(state);
        }
    }

    fn set_enabled(&self, name: &str, enabled: bool) {
        if let Some(action) = self
            .actions
            .lookup_action(name)
            .and_downcast::<gio::SimpleAction>()
            && action.is_enabled() != enabled
        {
            action.set_enabled(enabled);
        }
    }

    fn refresh_bookmarks(&self) {
        let places = self.pinned_places.borrow();
        if *self.rendered_bookmarks.borrow() == *places {
            return;
        }
        self.bookmarks.remove_all();
        for (index, (_, name)) in places.iter().enumerate() {
            let item = gio::MenuItem::new(Some(&name.replace('_', "__")), None);
            item.set_action_and_target_value(
                Some(&format!("{ACTIONS}.open-bookmark")),
                Some(&(index as u32).to_variant()),
            );
            self.bookmarks.append_item(&item);
        }
        self.rendered_bookmarks.replace(places.clone());
    }
}

fn action_allowed(window: &glib::WeakRef<gtk::ApplicationWindow>, view: &BrowserView) -> bool {
    window.upgrade().is_some_and(|window| {
        !super::super::foreign_modal_visible(&window, None) && !view.inline_edit_is_open()
    })
}

fn mode_name(mode: BrowserMode) -> &'static str {
    match mode {
        BrowserMode::Columns => "columns",
        BrowserMode::Icons => "icons",
        BrowserMode::List => "list",
    }
}

fn mode_for_name(name: &str) -> Option<BrowserMode> {
    match name {
        "columns" => Some(BrowserMode::Columns),
        "icons" => Some(BrowserMode::Icons),
        "list" => Some(BrowserMode::List),
        _ => None,
    }
}

fn status_text(browser: &Browser) -> Option<String> {
    let depth = browser.active_depth()?;
    let counts = browser.column_entry_counts(depth)?;
    let selected = browser.selected_count();
    if selected == 0 {
        return Some(crate::i18n::count("items", counts.total));
    }
    // The plural form follows the total; the selected count fills in afterwards.
    Some(rust_i18n::replace_patterns(
        &crate::i18n::count("selected_of_items", counts.total),
        &["selected"],
        &[crate::i18n::integer(selected as u64)],
    ))
}

/// GTK spells accelerators with its own translations, which this system
/// lacks ("Ctrl+X" in a German menu). Each time a menu opens, its rows show
/// Strata's label for the same key instead ("Strg+X", "Entf").
fn localize_accelerators(widget: &gtk::Widget) {
    if let Some(popover) = widget.downcast_ref::<gtk::Popover>() {
        popover.connect_map(|popover| relabel_accelerators(popover.upcast_ref()));
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        localize_accelerators(&current);
    }
}

fn relabel_accelerators(widget: &gtk::Widget) {
    if widget.find_property("accel").is_some()
        && let Some(accel) = widget.property::<Option<String>>("accel")
        && let Some((key, modifiers)) = gtk::accelerator_parse(&accel)
    {
        let text = crate::ui::keys::label(&Keys::One(Binding::new(key, modifiers)));
        let mut child = widget.first_child();
        while let Some(current) = child {
            child = current.next_sibling();
            if current.css_name() == "accelerator"
                && let Some(label) = current.downcast_ref::<gtk::Label>()
                && label.label() != text
            {
                label.set_label(&text);
            }
        }
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        relabel_accelerators(&current);
    }
}

fn menu_model(bookmarks: &gio::Menu) -> gio::Menu {
    let tr = crate::i18n::tr;
    let action = |name: &str| format!("{ACTIONS}.{name}");
    // GNOME 2 shows each command's shortcut beside it. The `accel` attribute
    // is display-only: the window's key dispatcher keeps handling the keys,
    // so no shortcut is bound twice. The bindings are the dispatcher's own.
    let item = |section: &gio::Menu, label: &str, action: &str, accel: Option<Binding>| {
        let item = gio::MenuItem::new(Some(&tr(label)), Some(action));
        if let Some(accel) = accel {
            item.set_attribute_value("accel", Some(&accel.accelerator().to_variant()));
        }
        section.append_item(&item);
    };
    let model = gio::Menu::new();

    let file = gio::Menu::new();
    let section = gio::Menu::new();
    item(&section, "New Window", "win.new-window", Some(NEW_WINDOW));
    item(&section, "New tab", "win.new-tab", Some(TAB_NEW));
    item(
        &section,
        "New Folder",
        &action("new-folder"),
        Some(NEW_FOLDER),
    );
    file.append_section(None, &section);
    let section = gio::Menu::new();
    section.append(
        Some(&tr("Connect to Server…")),
        Some(&action("connect-to-server")),
    );
    file.append_section(None, &section);
    let section = gio::Menu::new();
    item(
        &section,
        "Properties",
        &action("properties"),
        Some(PROPERTIES),
    );
    file.append_section(None, &section);
    let section = gio::Menu::new();
    item(&section, "Close tab", "win.close-tab", Some(TAB_CLOSE));
    item(&section, "Close window", &action("close-window"), None);
    file.append_section(None, &section);
    model.append_submenu(Some(&tr("File")), &file);

    let edit = gio::Menu::new();
    let section = gio::Menu::new();
    item(&section, "Undo", &action("undo"), Some(UNDO));
    item(&section, "Redo", &action("redo"), Some(REDO));
    edit.append_section(None, &section);
    let section = gio::Menu::new();
    item(&section, "Cut", &action("cut"), Some(CUT));
    item(&section, "Copy", &action("copy"), Some(COPY));
    item(&section, "Paste", &action("paste"), Some(PASTE));
    item(&section, "Duplicate", &action("duplicate"), Some(DUPLICATE));
    edit.append_section(None, &section);
    let section = gio::Menu::new();
    item(
        &section,
        "Select all",
        &action("select-all"),
        Some(SELECT_ALL),
    );
    edit.append_section(None, &section);
    let section = gio::Menu::new();
    item(&section, "Rename…", &action("rename"), Some(RENAME));
    item(
        &section,
        "Move to Trash",
        &action("trash-selection"),
        Some(DELETE),
    );
    item(
        &section,
        "Permanently delete",
        &action("delete-selection"),
        Some(DELETE_PERMANENTLY),
    );
    edit.append_section(None, &section);
    let section = gio::Menu::new();
    item(&section, "Settings", &action("settings"), Some(SETTINGS));
    edit.append_section(None, &section);
    model.append_submenu(Some(&tr("Edit")), &edit);

    let view = gio::Menu::new();
    let section = gio::Menu::new();
    item(&section, "Reload", &action("reload"), Some(REFRESH_R));
    view.append_section(None, &section);
    let section = gio::Menu::new();
    item(
        &section,
        "Show hidden files",
        &action("show-hidden"),
        Some(HIDDEN_FILES),
    );
    item(&section, "Side pane", &action("side-pane"), Some(SIDEBAR));
    view.append_section(None, &section);
    let arrange = gio::Menu::new();
    let keys = gio::Menu::new();
    for (label, key) in [
        ("Name", SortKey::Name),
        ("Size", SortKey::Size),
        ("Type", SortKey::Type),
        ("Modified", SortKey::Modified),
    ] {
        let name = key.stored_name().unwrap_or("name");
        keys.append(
            Some(&tr(label)),
            Some(&format!("{}::{name}", action("arrange"))),
        );
    }
    arrange.append_section(None, &keys);
    let order = gio::Menu::new();
    order.append(Some(&tr("Reverse")), Some(&action("arrange-reversed")));
    arrange.append_section(None, &order);
    let section = gio::Menu::new();
    section.append_submenu(Some(&tr("Arrange Items")), &arrange);
    view.append_section(None, &section);
    let section = gio::Menu::new();
    for (label, mode, digit) in [
        ("Icons", BrowserMode::Icons, gtk::gdk::Key::_2),
        ("List", BrowserMode::List, gtk::gdk::Key::_3),
        ("Columns", BrowserMode::Columns, gtk::gdk::Key::_1),
    ] {
        item(
            &section,
            label,
            &format!("{}::{}", action("view-mode"), mode_name(mode)),
            Some(Binding::ctrl(digit)),
        );
    }
    view.append_section(None, &section);
    model.append_submenu(Some(&tr("View")), &view);

    let go = gio::Menu::new();
    let section = gio::Menu::new();
    item(&section, "Back", &action("back"), Some(HISTORY_BACK));
    item(
        &section,
        "Forward",
        &action("forward"),
        Some(HISTORY_FORWARD),
    );
    item(&section, "Up", &action("up"), Some(PARENT_FOLDER));
    item(&section, "Home", &action("home"), Some(HOME_FOLDER));
    go.append_section(None, &section);
    let section = gio::Menu::new();
    section.append(Some(&tr("Trash")), Some(&action("trash")));
    section.append(Some(&tr("Network")), Some(&action("network")));
    section.append(
        Some(&tr("Connect to Server…")),
        Some(&action("connect-to-server")),
    );
    go.append_section(None, &section);
    let section = gio::Menu::new();
    item(
        &section,
        "Location",
        &action("edit-location"),
        Some(LOCATION),
    );
    item(&section, "Search", "win.search", Some(SEARCH));
    go.append_section(None, &section);
    model.append_submenu(Some(&tr("Go")), &go);

    let bookmarks_menu = gio::Menu::new();
    let section = gio::Menu::new();
    section.append(Some(&tr("Add bookmark")), Some(&action("add-bookmark")));
    bookmarks_menu.append_section(None, &section);
    bookmarks_menu.append_section(None, bookmarks);
    model.append_submenu(Some(&tr("Bookmarks")), &bookmarks_menu);

    let help = gio::Menu::new();
    item(
        &help,
        "Keyboard shortcuts",
        &action("keyboard-shortcuts"),
        Some(REFERENCE),
    );
    help.append(Some(&tr("About")), Some(&action("about")));
    model.append_submenu(Some(&tr("Help")), &help);
    model
}

/// Classic toolbar icon size: GTK 2's "small toolbar" (QF-0f; the former
/// 24 px "large toolbar" icons dominated the compact bar).
const TOOLBAR_ICON_PX: i32 = 16;

/// How the toolbar buttons look (QF-0f). GNOME 2 offered both in its
/// toolbar preferences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolbarStyle {
    /// Variant A, the default: a 16 px icon beside the label.
    IconsAndText,
    /// Variant B: labels only, the most compact and calmest bar.
    TextOnly,
}

impl ToolbarStyle {
    /// From `QUERENCIA_FILES_TOOLBAR` (`text` → B, anything else → A).
    pub(super) fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("text") => Self::TextOnly,
            _ => Self::IconsAndText,
        }
    }

    fn current() -> Self {
        Self::from_setting(std::env::var("QUERENCIA_FILES_TOOLBAR").ok().as_deref())
    }
}

/// A toolbar icon from the desktop's icon theme: the full-colour name first,
/// then its `-symbolic` variant, then Strata's bundled chrome icon.
fn toolbar_icon(theme_name: &str, fallback: &str) -> gtk::Image {
    let symbolic = format!("{theme_name}-symbolic");
    let available =
        gtk::gdk::Display::default().map(|display| gtk::IconTheme::for_display(&display));
    let themed = available.and_then(|theme| {
        [theme_name, symbolic.as_str()]
            .into_iter()
            .find(|name| theme.has_icon(name))
            .map(str::to_owned)
    });
    match themed {
        Some(name) => {
            let image = gtk::Image::from_icon_name(&name);
            image.set_pixel_size(TOOLBAR_ICON_PX);
            image.add_css_class("classic-tool-icon");
            image
        }
        None => assets::chrome_icon(fallback),
    }
}

fn build_toolbar() -> gtk::Box {
    let style = ToolbarStyle::current();
    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    toolbar.add_css_class("classic-toolbar");
    if style == ToolbarStyle::TextOnly {
        toolbar.add_css_class("text-only");
    }
    for (label, theme_icon, fallback, name) in [
        ("Back", "go-previous", icons::ARROW_LEFT, "back"),
        ("Forward", "go-next", icons::ARROW_RIGHT, "forward"),
        ("Up", "go-up", icons::ARROW_UP, "up"),
        ("Home", "go-home", icons::HOME, "home"),
        ("Reload", "view-refresh", icons::REFRESH, "reload"),
    ] {
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        if style == ToolbarStyle::IconsAndText {
            content.append(&toolbar_icon(theme_icon, fallback));
        }
        content.append(&gtk::Label::new(Some(&crate::i18n::tr(label))));
        let button = gtk::Button::new();
        button.set_child(Some(&content));
        button.add_css_class("classic-tool-button");
        button.set_action_name(Some(&format!("{ACTIONS}.{name}")));
        toolbar.append(&button);
    }
    toolbar
}

/// Moves the browser's location control out of Strata's header into its own row.
fn build_location_row(view: &BrowserView) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.add_css_class("classic-location-row");
    let caption = gtk::Label::new(Some(&format!("{}:", crate::i18n::tr("Location"))));
    caption.add_css_class("classic-location-caption");
    row.append(&caption);
    let location = view.location_widget();
    if let Some(parent) = location.parent().and_downcast::<gtk::Box>() {
        parent.remove(&location);
    }
    location.set_hexpand(true);
    row.append(&location);
    row
}

#[cfg(test)]
mod toolbar_tests {
    use super::{TOOLBAR_ICON_PX, ToolbarStyle};

    #[test]
    fn small_icons_with_text_are_the_default_and_text_only_is_chosen_explicitly() {
        assert_eq!(TOOLBAR_ICON_PX, 16);
        assert_eq!(ToolbarStyle::from_setting(None), ToolbarStyle::IconsAndText);
        assert_eq!(
            ToolbarStyle::from_setting(Some("")),
            ToolbarStyle::IconsAndText
        );
        assert_eq!(
            ToolbarStyle::from_setting(Some("symbole")),
            ToolbarStyle::IconsAndText
        );
        assert_eq!(
            ToolbarStyle::from_setting(Some("text")),
            ToolbarStyle::TextOnly
        );
        assert_eq!(
            ToolbarStyle::from_setting(Some(" text ")),
            ToolbarStyle::TextOnly
        );
    }
}
