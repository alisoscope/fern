use std::ops;
use std::sync::Arc;
use std::fmt;
use std::ffi;
use std::time;
use thiserror::Error;

use crate::prelude::*;
use vulkan_backend::prelude::*;

// Backends
/// Vulkan graphics backend
pub mod vulkan_backend;
pub mod wgpu_backend;

pub trait GraphicsSystemImpl: Send + Sync {
    
}

pub trait PresentationSubSystemImpl {
    fn window(&self) -> &sdl3::video::Window;
    fn window_size(&self) -> (u32, u32);
    fn resize_surface(&mut self, width: u32, height: u32);
    fn present<F: FnMut(&mut wgpu::RenderPass)>(&mut self, f: F) -> anyhow::Result<()>;
}

#[repr(transparent)]
pub struct GraphicsSystem<T: GraphicsSystemImpl = wgpu_backend::WGpuGraphicsSystem>(T);

#[repr(transparent)]
pub struct PresentationSubSystem<T: PresentationSubSystemImpl = wgpu_backend::WGpuPresentationSubSystem>(T);

impl<T: GraphicsSystemImpl> ops::Deref for GraphicsSystem<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl <T: GraphicsSystemImpl> ops::DerefMut for GraphicsSystem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: GraphicsSystemImpl> From<T> for GraphicsSystem<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T: PresentationSubSystemImpl> ops::Deref for PresentationSubSystem<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: PresentationSubSystemImpl> ops::DerefMut for PresentationSubSystem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: PresentationSubSystemImpl> From<T> for PresentationSubSystem<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
