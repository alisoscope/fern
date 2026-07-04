
use crate::prelude::*;
use super::prelude::*;
use super::Instance;

pub struct Surface {
    inner: vk::SurfaceKHR,
    instance: Instance,
    window: sdl3::video::Window,
}

impl Surface {
    pub fn new(
        instance: Instance,
        window: sdl3::video::Window,
    ) -> Result<Self, sdl3::Error> {
        let surface = unsafe {
            window.vulkan_create_surface(
                instance.handle(),
            )
        }?;

        Ok(Self {
            inner: surface,
            window,
            instance,
        })
    }
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.inner
    }
    pub fn window(&self) -> &sdl3::video::Window {
        &self.window
    }
    pub fn capabilities(&self, physical_device: vk::PhysicalDevice) -> VkResult<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            self.instance.surface.get_physical_device_surface_capabilities(
                physical_device,
                self.inner,
            )
        }
    }
    pub fn formats(&self, physical_device: vk::PhysicalDevice) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
        unsafe {
            self.instance.surface.get_physical_device_surface_formats(
                physical_device,
                self.inner,
            )
        }
    }
    pub fn support(&self, physical_device: vk::PhysicalDevice, queue_family_index: u32) -> VkResult<bool> {
        unsafe {
            self.instance.surface.get_physical_device_surface_support(
                physical_device,
                queue_family_index,
                self.inner,
            )
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        log_verbose_debug!(LogType::Info, "Destroying Vulkan Surface.");
        unsafe {
            self.instance.surface.destroy_surface(
                self.inner,
                super::ALLOCATION_CALLBACKS,
            );
        }
    }
}
