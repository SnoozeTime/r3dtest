//! Safe wrapper around imgui-sys for tab menu.

use imgui::*;
use std::{ptr, thread};

bitflags! {
    #[repr(transparent)]
    pub struct TabBarFlags: u32 {
        const REORDERABLE = sys::ImGuiTabBarFlags_Reorderable;
        const AUTO_SELECT_NEW_TABS = sys::ImGuiTabBarFlags_AutoSelectNewTabs;
        const TAB_LIST_POPUP_BUTTON = sys::ImGuiTabBarFlags_TabListPopupButton;
        const NO_CLOSE_WITH_MIDDLE_MOUSE_BUTTON = sys::ImGuiTabBarFlags_NoCloseWithMiddleMouseButton;
        const NO_TAB_LIST_SCROLLING_BUTTONS = sys::ImGuiTabBarFlags_NoTabListScrollingButtons;
        const NO_TOOLTIP = sys::ImGuiTabBarFlags_NoTooltip;
        const FITTING_POLICY_RESIZE_DOWN = sys::ImGuiTabBarFlags_FittingPolicyResizeDown;
        const FITTING_POLICY_SCROLL = sys::ImGuiTabBarFlags_FittingPolicyScroll;
        const FITTING_POLICY_MASK = sys::ImGuiTabBarFlags_FittingPolicyMask_;
        const FITTING_POLICY_DEFAULT = sys::ImGuiTabBarFlags_FittingPolicyDefault_;
    }
}

pub struct TabBar<'a> {
    id: &'a ImStr,
    flags: TabBarFlags,
}

impl<'a> TabBar<'a> {
    pub fn new(id: &ImStr) -> Self {
        Self {
            id,
            flags: TabBarFlags::empty(),
        }
    }

    #[must_use]
    pub fn begin(&self, ui: &Ui) -> Option<TabBarToken> {
        let shoud_render =
            unsafe { sys::igBeginTabBar(im_str!("bar").as_ptr(), 0 as ::std::os::raw::c_int) };

        if shoud_render {
            Some(TabBarToken { ctx: ui.ctx })
        } else {
            unsafe { sys::igEndTabBar() };
            None
        }
    }
}

/// Tracks a window that must be ended by calling `.end()`
pub struct TabBarToken {
    ctx: *const Context,
}

impl TabBarToken {
    /// Ends a tab bar
    #[must_use]
    pub fn end(mut self, _: &Ui) {
        self.ctx = ptr::null();
        unsafe { sys::igEndTabBar() };
    }
}

impl Drop for TabBarToken {
    fn drop(&mut self) {
        if !self.ctx.is_null() && !thread::panicking() {
            panic!("A TabBarToken was leaked. Did you call .end()?");
        }
    }
}
