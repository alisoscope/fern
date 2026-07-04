use std::sync::Arc;
use std::ffi;
use std::ops;

use crate::prelude::*;
use super::prelude::*;
use super::Device;
use super::device::QueueType;
use super::Surface;

pub struct Swapchain {
    image_views: Vec<vk::ImageView>,
    images: Vec<vk::Image>,
    extent: vk::Extent2D,
    inner: vk::SwapchainKHR, 
    surface: Surface,
    device: Device,
}

impl Swapchain {
    pub const DESIRED_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;
    pub const DESIRED_COLOR_SPACE: vk::ColorSpaceKHR = vk::ColorSpaceKHR::SRGB_NONLINEAR;
    pub fn new(device: Device, surface: Surface) -> VkResult<Self> {
        let surface_capabilities = surface.capabilities(device.physical_device)?;
        let surface_formats = surface.formats(device.physical_device)?;

        if surface_formats.iter().find(|&format| format.format == Self::DESIRED_FORMAT && format.color_space == Self::DESIRED_COLOR_SPACE).is_none() {
            panic!("Surface does not support require surface format.");
        }

        let (pixel_width, pixel_height) = surface.window().size_in_pixels();

        let swapchain_extent = vk::Extent2D {
            width: pixel_width.clamp(surface_capabilities.min_image_extent.width, surface_capabilities.max_image_extent.width),
            height: pixel_height.clamp(surface_capabilities.min_image_extent.height, surface_capabilities.max_image_extent.height),
        };

        let min_image_count = (surface_capabilities.min_image_count + 1).min(
            if surface_capabilities.max_image_count == 0 { u32::MAX } else { surface_capabilities.max_image_count }
        );

        if device.get_queue_family_index(QueueType::Presentation) != device.get_queue_family_index(QueueType::Graphics){
            panic!("Cannot use exclusive sharing mode for swapchain.");
        }

        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            surface: surface.handle(),
            min_image_count,
            image_format: Self::DESIRED_FORMAT,
            image_color_space: Self::DESIRED_COLOR_SPACE,
            image_extent: swapchain_extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform: surface_capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: vk::PresentModeKHR::FIFO,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            ..Default::default()
        };

        let swapchain = unsafe {
            device.swapchain.create_swapchain(
                &swapchain_create_info,
                super::ALLOCATION_CALLBACKS,
            )
        }?;

        let images = unsafe {
            device.swapchain.get_swapchain_images(swapchain)
        }?;

        let image_views = images
            .iter()
            .map(|&image| {
                let image_view_create_info = vk::ImageViewCreateInfo {
                    image,
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: Self::DESIRED_FORMAT,
                    components: vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    },
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    ..Default::default()
                };
                unsafe {
                    device.create_image_view(
                        &image_view_create_info,
                        super::ALLOCATION_CALLBACKS,
                    )
                }
            })
            .collect::<VkResult<Vec<_>>>()?;

        Ok(Self {
            image_views,
            images,
            extent: swapchain_extent,
            inner: swapchain,
            surface,
            device,
        })
    }
    pub fn handle(&self) -> vk::SwapchainKHR {
        self.inner
    }
    pub fn acquire_next_image(&mut self, semaphore: vk::Semaphore) -> VkResult<(vk::Image, vk::ImageView, u32)> {
        let (image_index, suboptimal) = unsafe {
            self.device.swapchain.acquire_next_image(
                self.inner,
                u64::MAX,
                semaphore,
                vk::Fence::null(),
            )
        }?;

        if suboptimal {
            log_debug!(LogType::Warning, "Swapchain Suboptimal.");
        }

        Ok((
            self.images[image_index as usize],
            self.image_views[image_index as usize],
            image_index,
        ))
    }
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        log_verbose_debug!(LogType::Info, "Destroying Vulkan Swapchain.");
        unsafe {
            for image_view in &self.image_views {
                self.device.destroy_image_view(
                    *image_view,
                    super::ALLOCATION_CALLBACKS,
                );
            }
            self.device.swapchain.destroy_swapchain(
                self.inner,
                super::ALLOCATION_CALLBACKS,
            );
        }
    }
}
