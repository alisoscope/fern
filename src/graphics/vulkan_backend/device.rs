use std::sync::Arc;
use std::ffi;
use std::ops;

use super::Instance;
use super::Surface;
use crate::prelude::*;
use super::prelude::*;

#[derive(Clone)]
pub struct Device(Arc<DeviceInner>);

#[derive(Clone, Copy, Debug)]
pub enum QueueType {
    Graphics,
    Presentation,
    Transfer,
}

impl Device {
    /// TODO: make a physical device wrapper
    pub fn new(
        instance: Instance,
        physical_device: vk::PhysicalDevice,
        surface: &Surface,
    ) -> VkResult<Self> {
        let queue_properties = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
        };

        let queue_index = queue_properties.iter()
            .enumerate()
            .find_map(|(i, properties)| {
                let present_support = surface.support(
                    physical_device,
                    i as u32,
                ).ok()?;
                let graphics_support = properties.queue_flags
                    .contains(vk::QueueFlags::GRAPHICS);

                if present_support && graphics_support {
                    Some(i as u32)
                } else { None }
            }).unwrap();

        let queue_create_infos = &[
            vk::DeviceQueueCreateInfo {
                queue_family_index: queue_index,
                queue_count: 1,
                p_queue_priorities: &0f32,
                ..Default::default()
            }
        ];

        let required_extensions_ptrs = super::REQUIRED_DEVICE_EXTENSIONS.iter()
            .map(|&s| s.as_ptr())
            .collect::<Vec<_>>();

        let vulkan13_features = vk::PhysicalDeviceVulkan13Features {
            synchronization2: vk::TRUE,
            dynamic_rendering: vk::TRUE,
            ..Default::default()
        };

        let vulkan12_features = vk::PhysicalDeviceVulkan12Features {
            p_next: &raw const vulkan13_features as _,
            ..Default::default()
        };

        let vulkan11_features = vk::PhysicalDeviceVulkan11Features {
            p_next: &raw const vulkan12_features as _,
            ..Default::default()
        };

        let features = vk::PhysicalDeviceFeatures2 {
            p_next: &raw const vulkan11_features as _,
            features: vk::PhysicalDeviceFeatures {
                wide_lines: vk::TRUE,
                ..Default::default()
            },
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo {
            p_next: &raw const features as _,
            queue_create_info_count: queue_create_infos.len() as u32,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_extension_count: required_extensions_ptrs.len() as u32,
            pp_enabled_extension_names: required_extensions_ptrs.as_ptr(),
            ..Default::default()
        };

        let device = unsafe {
            instance.create_device(
                physical_device,
                &device_create_info,
                super::ALLOCATION_CALLBACKS,
            )
        }?;

        let swapchain = khr::swapchain::Device::new(&instance, &device);

        let graphics_queue = unsafe {
            device.get_device_queue(queue_index, 0)
        };

        let present_queue = unsafe {
            device.get_device_queue(queue_index, 0)
        };

        Ok(Self(Arc::new(DeviceInner {
            present_index: queue_index,
            present_queue,
            graphics_index: queue_index,
            graphics_queue,
            swapchain,
            inner: device,
            physical_device,
            instance,
        })))
    }
    pub fn get_queue_handle(&self, queue_type: QueueType) -> vk::Queue {
        match queue_type {
            QueueType::Graphics => self.graphics_queue,
            QueueType::Presentation => self.present_queue,
            QueueType::Transfer => unimplemented!(),
        }
    }
    pub fn get_queue_family_index(&self, queue_type: QueueType) -> u32 {
        match queue_type {
            QueueType::Graphics => self.graphics_index,
            QueueType::Presentation => self.present_index,
            QueueType::Transfer => unimplemented!(),
        }
    }
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        self.handle() == other.handle()
    }
}

impl Eq for Device { }

impl Drop for DeviceInner {
    fn drop(&mut self) {
        log_verbose_debug!(LogType::Info, "Destroying Vulkan Device.");
        unsafe {
            self.destroy_device(
                super::ALLOCATION_CALLBACKS,
            )
        }
    }
}

#[non_exhaustive]
pub struct DeviceInner {
    present_index: u32,
    present_queue: vk::Queue,
    graphics_index: u32,
    graphics_queue: vk::Queue,
    pub swapchain: khr::swapchain::Device,
    pub inner: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub instance: Instance,
}

impl ops::Deref for Device {
    type Target = DeviceInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::Deref for DeviceInner {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
