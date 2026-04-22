//! Color palette — matches the original Python app's browser.tcss.

use ratatui::style::Color;

pub const BG: Color      = Color::Reset; // defer to terminal background
pub const SURFACE: Color = Color::Rgb(0x35, 0x3a, 0x3e); // panel backgrounds / modal
pub const BORDER: Color  = Color::Rgb(0x7a, 0xb5, 0xd8); // active tree border, directory names
pub const PINK: Color    = Color::Rgb(0xc0, 0x7e, 0xc5); // active preview border, cursor highlight
pub const GREEN: Color   = Color::Rgb(0x6a, 0xaa, 0x72); // search match highlight
pub const TEXT: Color    = Color::Rgb(0xcd, 0xd1, 0xd5); // normal file names
pub const MUTED: Color   = Color::Rgb(0x6a, 0x70, 0x75); // dimmed text, inactive border
pub const RED: Color     = Color::Rgb(0xbf, 0x61, 0x6a); // delete confirmation button
