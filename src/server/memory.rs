use crate::level::Level;
use bevy_ecs::prelude::{Local, Res};

// hmm, glibc raises its mmap threshold every time a big block is freed, so after a burst of
// generation all those ~100-200 KiB chunk buffers end up in the arenas and never go back to the OS.
// pinning it at 64 KiB keeps them on mmap, where free actually returns the pages
#[cfg(all(target_os = "linux", target_env = "gnu"))]
const MMAP_THRESHOLD: libc::c_int = 64 * 1024;

/// Tunes the system allocator for the server's allocation pattern. Only does anything on glibc.
pub fn tune_allocator() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        libc::mallopt(libc::M_MMAP_THRESHOLD, MMAP_THRESHOLD);
    }
}

/// Hands free heap pages back to the OS. Walks every arena, so call it when the server just
/// finished something heavy, not every tick.
pub fn release_free_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        libc::malloc_trim(0);
    }
}

/// Trims the heap once chunk generation goes idle, since that's where the big transient buffers come from.
pub fn release_after_generation(level: Option<Res<Level>>, mut generating: Local<bool>) {
    let Some(level) = level else { return };
    let pending = level.dimensions().any(|dimension| dimension.has_pending_generation());
    if *generating && !pending {
        release_free_memory();
    }
    *generating = pending;
}
