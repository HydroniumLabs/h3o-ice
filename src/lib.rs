//! Frozen (aka read-only) sets and maps for H3 cell index.
//!
//! Those data structures are built on top of finite state transducers, which
//! allows them to be extremely compact while offering fast lookup.
//!
//! This is especially for read-only indexes (both in-memory or on-disk).

// Lints {{{

#![deny(
    nonstandard_style,
    rust_2018_idioms,
    rust_2021_compatibility,
    future_incompatible,
    rustdoc::all,
    rustdoc::missing_crate_level_docs,
    missing_docs,
    unsafe_code,
    unused,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    variant_size_differences,
    warnings,
    clippy::all,
    clippy::cargo,
    clippy::pedantic,
    clippy::allow_attributes_without_reason,
    clippy::as_underscore,
    clippy::branches_sharing_code,
    clippy::clone_on_ref_ptr,
    clippy::cognitive_complexity,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::decimal_literal_representation,
    clippy::default_union_representation,
    clippy::derive_partial_eq_without_eq,
    clippy::empty_drop,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::equatable_if_let,
    clippy::exhaustive_enums,
    clippy::exit,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast_any,
    clippy::format_push_string,
    clippy::future_not_send,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::imprecise_flops,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::iter_with_drain,
    clippy::large_include_file,
    clippy::let_underscore_must_use,
    clippy::lossy_float_literal,
    clippy::mem_forget,
    clippy::missing_const_for_fn,
    clippy::mixed_read_write_in_expression,
    clippy::multiple_inherent_impl,
    clippy::mutex_atomic,
    clippy::mutex_integer,
    clippy::needless_collect,
    clippy::non_send_fields_in_send_ty,
    clippy::nonstandard_macro_braces,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::path_buf_push_overwrite,
    clippy::pattern_type_mismatch,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::redundant_pub_crate,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::self_named_module_files,
    clippy::significant_drop_in_scrutinee,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::trailing_empty_array,
    clippy::trait_duplication_in_bounds,
    clippy::transmute_undefined_repr,
    clippy::trivial_regex,
    clippy::try_err,
    clippy::type_repetition_in_bounds,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::unnecessary_self_imports,
    clippy::unneeded_field_pattern,
    clippy::unseparated_literal_suffix,
    clippy::unused_peekable,
    clippy::unused_rounding,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::use_self,
    clippy::useless_let_if_seq,
    clippy::verbose_file_reads
)]
#![allow(
    // The 90’s called and wanted their charset back.
    clippy::non_ascii_literal,
    // "It requires the user to type the module name twice."
    // => not true here since internal modules are hidden from the users.
    clippy::module_name_repetitions,
    // Usually yes, but not really applicable for most literals in this crate.
    clippy::unreadable_literal,
    reason = "allow some exceptions"
)]

// }}}

mod error;
mod key;
mod map;
mod set;

pub use error::BuildError;
pub use map::{
    FrozenMap, FrozenMapBuilder, FrozenMapIterator, FrozenMapKeys,
    FrozenMapValues,
};
pub use set::{FrozenSet, FrozenSetBuilder, FrozenSetIterator};

use key::Key;
