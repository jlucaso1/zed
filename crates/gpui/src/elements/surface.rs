#[cfg(not(target_os = "macos"))]
use crate::YuvFrameData;
use crate::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    ObjectFit, Pixels, Style, StyleRefinement, Styled, Window,
};
#[cfg(target_os = "macos")]
use core_video::pixel_buffer::CVPixelBuffer;
use refineable::Refineable;

/// A source of a surface's content.
#[derive(Clone, Debug)]
pub enum SurfaceSource {
    /// A macOS image buffer from CoreVideo.
    #[cfg(target_os = "macos")]
    Surface(CVPixelBuffer),
    /// YUV frame data for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    Yuv(YuvFrameData),
}

#[cfg(target_os = "macos")]
impl From<CVPixelBuffer> for SurfaceSource {
    fn from(value: CVPixelBuffer) -> Self {
        SurfaceSource::Surface(value)
    }
}

#[cfg(not(target_os = "macos"))]
impl From<YuvFrameData> for SurfaceSource {
    fn from(value: YuvFrameData) -> Self {
        SurfaceSource::Yuv(value)
    }
}

/// A surface element.
pub struct Surface {
    source: SurfaceSource,
    object_fit: ObjectFit,
    style: StyleRefinement,
}

/// Create a new surface element.
#[cfg(target_os = "macos")]
pub fn surface(source: impl Into<SurfaceSource>) -> Surface {
    Surface {
        source: source.into(),
        object_fit: ObjectFit::Contain,
        style: Default::default(),
    }
}

/// Create a new surface element from YUV frame data.
#[cfg(not(target_os = "macos"))]
pub fn surface(source: impl Into<SurfaceSource>) -> Surface {
    Surface {
        source: source.into(),
        object_fit: ObjectFit::Contain,
        style: Default::default(),
    }
}

impl Surface {
    /// Set the object fit for the image.
    pub fn object_fit(mut self, object_fit: ObjectFit) -> Self {
        self.object_fit = object_fit;
        self
    }
}

impl Element for Surface {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        let layout_id = window.request_layout(style, [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        match &self.source {
            #[cfg(target_os = "macos")]
            SurfaceSource::Surface(surface) => {
                let size = crate::size(surface.get_width().into(), surface.get_height().into());
                let new_bounds = self.object_fit.get_bounds(bounds, size);
                window.paint_surface(new_bounds, surface.clone());
            }
            #[cfg(not(target_os = "macos"))]
            SurfaceSource::Yuv(frame_data) => {
                let width = i32::try_from(frame_data.width).unwrap_or(i32::MAX);
                let height = i32::try_from(frame_data.height).unwrap_or(i32::MAX);
                let size = crate::size(crate::DevicePixels(width), crate::DevicePixels(height));
                let new_bounds = self.object_fit.get_bounds(bounds, size);
                window.paint_surface(new_bounds, frame_data.clone());
            }
        }
    }
}

impl IntoElement for Surface {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for Surface {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "macos"))]
    use crate::YuvFormat;
    #[cfg(not(target_os = "macos"))]
    use std::sync::Arc;

    #[cfg(not(target_os = "macos"))]
    fn create_test_nv12_frame(width: u32, height: u32) -> YuvFrameData {
        let y_size = (width * height) as usize;
        let uv_size = (width * height / 2) as usize;

        YuvFrameData {
            format: YuvFormat::Nv12,
            width,
            height,
            y_plane: vec![128u8; y_size].into(),
            u_plane: vec![128u8; uv_size].into(),
            v_plane: None,
            y_stride: width,
            u_stride: width,
            v_stride: None,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn create_test_i420_frame(width: u32, height: u32) -> YuvFrameData {
        let y_size = (width * height) as usize;
        let uv_size = (width * height / 4) as usize;

        YuvFrameData {
            format: YuvFormat::I420,
            width,
            height,
            y_plane: vec![128u8; y_size].into(),
            u_plane: vec![128u8; uv_size].into(),
            v_plane: Some(vec![128u8; uv_size].into()),
            y_stride: width,
            u_stride: width / 2,
            v_stride: Some(width / 2),
        }
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_nv12_frame_data_creation() {
        let frame = create_test_nv12_frame(640, 480);
        assert_eq!(frame.format, YuvFormat::Nv12);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 480);
        assert_eq!(frame.y_plane.len(), 640 * 480);
        assert_eq!(frame.u_plane.len(), 640 * 480 / 2);
        assert!(frame.v_plane.is_none());
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_i420_frame_data_creation() {
        let frame = create_test_i420_frame(640, 480);
        assert_eq!(frame.format, YuvFormat::I420);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 480);
        assert_eq!(frame.y_plane.len(), 640 * 480);
        assert_eq!(frame.u_plane.len(), 640 * 480 / 4);
        assert!(frame.v_plane.is_some());
        assert_eq!(frame.v_plane.as_ref().unwrap().len(), 640 * 480 / 4);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_surface_source_from_yuv() {
        let frame = create_test_nv12_frame(320, 240);
        let source: SurfaceSource = frame.into();
        match source {
            SurfaceSource::Yuv(f) => {
                assert_eq!(f.width, 320);
                assert_eq!(f.height, 240);
            }
        }
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_surface_element_creation() {
        let frame = create_test_nv12_frame(1920, 1080);
        let _surface_element = surface(frame);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_surface_element_object_fit() {
        let frame = create_test_nv12_frame(1920, 1080);
        let surface_element = surface(frame).object_fit(ObjectFit::Cover);
        assert!(matches!(surface_element.object_fit, ObjectFit::Cover));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_yuv_frame_clone() {
        let frame = create_test_i420_frame(640, 480);
        let cloned = frame.clone();
        assert_eq!(cloned.width, frame.width);
        assert_eq!(cloned.height, frame.height);
        assert_eq!(cloned.format, frame.format);
        assert!(Arc::ptr_eq(&cloned.y_plane, &frame.y_plane));
    }
}
