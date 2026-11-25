//! Imports communs à tous les modules.
pub(crate) use std::cell::RefCell;
pub(crate) use std::collections::HashMap;
pub(crate) use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub(crate) use windows::core::{w, Interface, Result};
pub(crate) use windows::Win32::Foundation::*;
pub(crate) use windows::Win32::Graphics::Direct2D::Common::*;
pub(crate) use windows::Win32::Graphics::Direct2D::*;
pub(crate) use windows::Win32::Graphics::Direct3D::*;
pub(crate) use windows::Win32::Graphics::Direct3D11::*;
pub(crate) use windows::Win32::Graphics::Dwm::*;
pub(crate) use windows::Win32::Graphics::DirectComposition::*;
pub(crate) use windows::Win32::Graphics::Dxgi::Common::*;
pub(crate) use windows::Win32::Graphics::Dxgi::*;
pub(crate) use windows::Win32::Graphics::Gdi::*;
pub(crate) use windows::Win32::Graphics::Imaging::*;
pub(crate) use windows::Win32::System::Com::*;
pub(crate) use windows::Win32::System::LibraryLoader::GetModuleHandleW;
pub(crate) use windows::Win32::System::Registry::*;
pub(crate) use windows::Win32::System::Threading::CreateMutexW;
pub(crate) use windows::Win32::UI::HiDpi::*;
pub(crate) use windows::Win32::UI::Shell::*;
pub(crate) use windows::Win32::UI::WindowsAndMessaging::*;

pub(crate) use crate::app::*;
pub(crate) use crate::characters::*;
pub(crate) use crate::desktop::*;
pub(crate) use crate::scenes::*;
pub(crate) use crate::settings::*;
pub(crate) use crate::snow::*;
pub(crate) use crate::sprites::*;
pub(crate) use crate::tray::*;
pub(crate) use crate::util::*;
