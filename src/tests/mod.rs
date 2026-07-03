use super::*;

#[cfg(feature = "slow-tests")]
mod fixtures;

#[cfg(feature = "slow-tests")]
mod candidate_selection;
#[cfg(feature = "slow-tests")]
mod capsule_templates;
mod cleanup;
#[cfg(feature = "slow-tests")]
mod fine_opt_candidate;
#[cfg(feature = "slow-tests")]
mod loose_opt_candidate;
mod optimizer;
mod path_data;
#[cfg(feature = "slow-tests")]
mod pixel_templates;
mod potrace_core;
#[cfg(feature = "slow-tests")]
mod quadratic_vertex_candidate;
#[cfg(feature = "slow-tests")]
mod ring_sector_templates;
#[cfg(feature = "slow-tests")]
mod rounded_rect_templates;
#[cfg(feature = "slow-tests")]
mod sharp_v_templates;
#[cfg(feature = "slow-tests")]
mod stepped_e_templates;
#[cfg(feature = "slow-tests")]
mod u_shape_templates;

#[cfg(feature = "slow-tests")]
use fixtures::*;
