pub mod emulator;
pub mod gdemulator;
pub(crate) mod gdassembler;
use godot::prelude::*;
pub mod neozasm;
struct CrustZinc;

#[gdextension]
unsafe impl ExtensionLibrary for CrustZinc {}
