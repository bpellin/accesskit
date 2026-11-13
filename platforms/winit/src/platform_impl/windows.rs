// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file).
// Crystal Launcher modified this adapter to skip Windows UI Automation under Wine.
#[cfg(feature = "rwh_05")]
use crate::raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
#[cfg(feature = "rwh_06")]
use crate::raw_window_handle::{HasWindowHandle, RawWindowHandle};

use accesskit::{ActionHandler, ActivationHandler, DeactivationHandler, TreeUpdate};
use accesskit_windows::{HWND, SubclassingAdapter};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

pub struct Adapter {
    adapter: Option<SubclassingAdapter>,
}

impl Adapter {
    pub fn new(
        _event_loop: &ActiveEventLoop,
        window: &Window,
        activation_handler: impl 'static + ActivationHandler,
        action_handler: impl 'static + ActionHandler + Send,
        _deactivation_handler: impl 'static + DeactivationHandler,
    ) -> Self {
        // Wine's UI Automation event delivery can panic in AccessKit. Skip the
        // subclass before it handles any messages or registers a provider.
        let ntdll = unsafe { GetModuleHandleW(windows_sys::w!("ntdll.dll")) };
        if !ntdll.is_null()
            && unsafe { GetProcAddress(ntdll, c"wine_get_version".as_ptr().cast()) }.is_some()
        {
            return Self { adapter: None };
        }

        #[cfg(feature = "rwh_05")]
        let hwnd = match window.raw_window_handle() {
            RawWindowHandle::Win32(handle) => handle.hwnd,
            RawWindowHandle::WinRt(_) => unimplemented!(),
            _ => unreachable!(),
        };
        #[cfg(feature = "rwh_06")]
        let hwnd = match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get() as *mut _,
            RawWindowHandle::WinRt(_) => unimplemented!(),
            _ => unreachable!(),
        };

        let adapter = SubclassingAdapter::new(HWND(hwnd), activation_handler, action_handler);
        Self {
            adapter: Some(adapter),
        }
    }

    pub fn update_if_active(&mut self, updater: impl FnOnce() -> TreeUpdate) {
        let events = self
            .adapter
            .as_mut()
            .and_then(|adapter| adapter.update_if_active(updater));
        if let Some(events) = events {
            events.raise();
        }
    }

    pub fn process_event(&mut self, _window: &Window, _event: &WindowEvent) {}
}
