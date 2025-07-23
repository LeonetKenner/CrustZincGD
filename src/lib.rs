pub mod emulator;
pub(crate) mod gdassembler;
pub mod gdemulator;
use godot::prelude::*;
pub mod neozasm;
struct CrustZinc;

#[gdextension]
unsafe impl ExtensionLibrary for CrustZinc {}
