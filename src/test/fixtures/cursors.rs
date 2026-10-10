/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! Cursor fixtures (`src/test/data/cursors/`): one stub per fixture, `verdicts()`, written by
//! `human_solver` when a cursor sample is promoted - one verdict for a still or animated Windows
//! cursor, one per changed member for an X or Hyprland cursor. See `test::helper::human_content`.
#[cfg(test)]
mod ani_x_pop_os_icon_theme_4c2c4af8_00000000000000020006000e7e9ffc3f;
#[cfg(test)]
mod ani_x_pop_os_icon_theme_4c2c4af8_wait;
#[cfg(test)]
mod ani_x_vinceliuice_vimix_cursors_de8100f3_busy;
#[cfg(test)]
mod ani_x_vinceliuice_vimix_cursors_de8100f3_working_in_background;
#[cfg(test)]
mod cur_x_pop_os_icon_theme_c8b14907_right_tee;
#[cfg(test)]
mod cur_x_vinceliuice_vimix_cursors_de8100f3_diagonal_resize_1;
#[cfg(test)]
mod hyprcursor_x_guillaumeboehm_nordzy_cursors_39478f04_color_picker;
#[cfg(test)]
mod hyprcursor_x_guillaumeboehm_nordzy_cursors_39478f04_progress;
#[cfg(test)]
mod hyprcursor_x_guillaumeboehm_nordzy_cursors_5bd0b1ba_wait;
#[cfg(test)]
mod xcursor_x_alvatip_neonly_8085a361_left_tee;
