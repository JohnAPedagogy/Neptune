//! Copying a finished frame back out of the GPU and onto disk as a PNG.
//!
//! This is the mirror image of [`super::texture`]: that module stages CPU
//! pixels into a device image with `copy_buffer_to_image`, this one drains a
//! device image into a host-visible buffer with `copy_image_to_buffer`. Both
//! block on a fence, and for the same reason — neither is a hot path.
//!
//! The copy command is recorded into the *same* command buffer as the frame's
//! draw calls, right after the render pass ends, so it reads the finished image
//! before it is handed to the presentation engine. Vulkano's automatic
//! synchronisation supplies the layout transitions on both sides: the render
//! pass leaves the swapchain image in `PresentSrc`, the copy needs
//! `TransferSrcOptimal`, and the final barrier Vulkano appends puts it back in
//! `PresentSrc` for the present that follows.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::format::Format;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};

/// Why a screenshot could not be produced.
#[derive(Debug)]
pub(crate) enum ScreenshotError {
    /// The surface never offered `ImageUsage::TRANSFER_SRC`, so nothing can be
    /// copied out of the swapchain.
    NotSupported,
    /// The swapchain's colour format is not one of the 8-bit-per-channel
    /// layouts this module knows how to unpack.
    UnsupportedFormat(Format),
    /// The read-back buffer could not be allocated.
    Allocate(String),
    /// The PNG could not be encoded or written.
    Write(image::ImageError),
    /// The parent directory of the requested path could not be created.
    Io(std::io::Error),
}

impl fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScreenshotError::NotSupported => {
                write!(f, "this surface's swapchain images cannot be copied from")
            }
            ScreenshotError::UnsupportedFormat(format) => {
                write!(f, "swapchain format {format:?} is not an 8-bit RGBA/BGRA layout")
            }
            ScreenshotError::Allocate(err) => write!(f, "failed to allocate a read-back buffer: {err}"),
            ScreenshotError::Write(err) => write!(f, "failed to write the PNG: {err}"),
            ScreenshotError::Io(err) => write!(f, "failed to prepare the output directory: {err}"),
        }
    }
}

impl std::error::Error for ScreenshotError {}

/// Whether a swapchain format stores its first channel as blue rather than red.
///
/// `Ok(true)` means the bytes come back as B, G, R, A and have to be swizzled;
/// `Ok(false)` means they are already R, G, B, A. Only the 8-bit-per-channel
/// four-component formats are handled, because those are what a desktop surface
/// actually reports — anything else is refused rather than silently mangled.
pub(crate) fn swaps_red_and_blue(format: Format) -> Result<bool, ScreenshotError> {
    match format {
        Format::B8G8R8A8_UNORM | Format::B8G8R8A8_SRGB => Ok(true),
        Format::R8G8B8A8_UNORM | Format::R8G8B8A8_SRGB => Ok(false),
        other => Err(ScreenshotError::UnsupportedFormat(other)),
    }
}

/// Rewrites raw captured bytes into the tightly packed, fully opaque RGBA8 that
/// the `image` crate wants for a PNG.
///
/// Two fixups happen here, both pure and both testable without a GPU:
///
/// - If `swap_red_blue`, the first and third bytes of every pixel are exchanged
///   (BGRA -> RGBA).
/// - The alpha byte is forced to 255. A swapchain image's alpha is whatever the
///   blending left behind and the compositor was told to ignore
///   (`CompositeAlpha::Opaque`), so carrying it into the PNG would produce
///   randomly see-through screenshots rather than the picture on screen.
pub(crate) fn to_opaque_rgba8(raw: &[u8], swap_red_blue: bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(raw.len());
    for pixel in raw.chunks_exact(4) {
        if swap_red_blue {
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
        } else {
            rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
        }
    }
    rgba
}

/// Allocates the host-visible buffer one captured frame is copied into.
///
/// Sized for a tightly packed 4-bytes-per-pixel image, which is what
/// `CopyImageToBufferInfo::image_buffer`'s default region writes.
pub(crate) fn readback_buffer(
    allocator: &Arc<StandardMemoryAllocator>,
    extent: [u32; 2],
) -> Result<Subbuffer<[u8]>, ScreenshotError> {
    let byte_len = extent[0] as u64 * extent[1] as u64 * 4;
    Buffer::new_slice::<u8>(
        allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
        byte_len,
    )
    .map_err(|err| ScreenshotError::Allocate(err.to_string()))
}

/// Encodes an already-captured frame and writes it out.
///
/// Call only after the fence for the submission holding the copy has been
/// waited on: until then the buffer's contents are undefined.
pub(crate) fn write_png(
    buffer: &Subbuffer<[u8]>,
    extent: [u32; 2],
    format: Format,
    path: &Path,
) -> Result<(), ScreenshotError> {
    let swap = swaps_red_and_blue(format)?;

    let rgba = {
        let mapped = buffer
            .read()
            .map_err(|err| ScreenshotError::Allocate(err.to_string()))?;
        to_opaque_rgba8(&mapped, swap)
    };

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(ScreenshotError::Io)?;
    }

    image::save_buffer(
        path,
        &rgba,
        extent[0],
        extent[1],
        image::ExtendedColorType::Rgba8,
    )
    .map_err(ScreenshotError::Write)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_formats_are_swizzled_and_rgba_ones_are_not() {
        assert!(swaps_red_and_blue(Format::B8G8R8A8_UNORM).unwrap());
        assert!(swaps_red_and_blue(Format::B8G8R8A8_SRGB).unwrap());
        assert!(!swaps_red_and_blue(Format::R8G8B8A8_UNORM).unwrap());
        assert!(!swaps_red_and_blue(Format::R8G8B8A8_SRGB).unwrap());
    }

    #[test]
    fn an_unhandled_format_is_refused_rather_than_guessed_at() {
        assert!(matches!(
            swaps_red_and_blue(Format::R16G16B16A16_SFLOAT),
            Err(ScreenshotError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn swizzling_exchanges_red_and_blue_and_leaves_green_alone() {
        // One pixel, stored blue-first: B=1, G=2, R=3, A=4.
        let raw = [1u8, 2, 3, 4];
        assert_eq!(to_opaque_rgba8(&raw, true), vec![3, 2, 1, 255]);
    }

    #[test]
    fn a_format_already_in_rgba_order_keeps_its_channels() {
        let raw = [1u8, 2, 3, 4];
        assert_eq!(to_opaque_rgba8(&raw, false), vec![1, 2, 3, 255]);
    }

    #[test]
    fn alpha_is_forced_opaque_whichever_way_the_channels_run() {
        let raw = [10u8, 20, 30, 0, 40, 50, 60, 128];
        assert_eq!(to_opaque_rgba8(&raw, false)[3], 255);
        assert_eq!(to_opaque_rgba8(&raw, false)[7], 255);
        assert_eq!(to_opaque_rgba8(&raw, true)[3], 255);
        assert_eq!(to_opaque_rgba8(&raw, true)[7], 255);
    }

    #[test]
    fn every_pixel_of_a_multi_pixel_frame_is_converted() {
        // 2x1 pixels, BGRA: red then green.
        let raw = [0u8, 0, 255, 255, 0, 255, 0, 255];
        assert_eq!(
            to_opaque_rgba8(&raw, true),
            vec![255, 0, 0, 255, 0, 255, 0, 255]
        );
    }

    #[test]
    fn the_converted_buffer_is_the_same_length_as_the_capture() {
        let raw = vec![7u8; 4 * 16];
        assert_eq!(to_opaque_rgba8(&raw, true).len(), raw.len());
    }
}
