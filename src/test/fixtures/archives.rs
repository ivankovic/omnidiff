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

//! Archive fixtures (`src/test/data/archives/`): one stub per fixture, `verdicts()`, written by
//! `human_solver` when an archive sample is promoted. See `test::helper::human_content`.
#[cfg(test)]
mod bzip2_x_jonathansalwan_ropgadget_7c4f6ecf_ref_output;
#[cfg(test)]
mod bzip2_x_jonathansalwan_ropgadget_9f20a4be_ref_output;
#[cfg(test)]
mod bzip2_x_rocm_miopen_801eab7f_gfx942130_db_txt;
#[cfg(test)]
mod bzip2_x_rocm_miopen_801eab7f_gfx942130_hip_fdb_txt;
#[cfg(test)]
mod gzip_x_atsb_dav_text_1396c098_dav_1;
#[cfg(test)]
mod gzip_x_atsb_dav_text_1a7fef01_dav_1;
#[cfg(test)]
mod gzip_x_cesanta_docker_auth_c67fa202_docker_auth_1_5_0;
#[cfg(test)]
mod gzip_x_fbb_git_icmake_650f438d_bobcat;
#[cfg(test)]
mod gzip_x_poetaman_arttime_3f0e855e_artprint_1;
#[cfg(test)]
mod zip_x_silnrsi_teckit_3d06b4f4_teckit_tools;
