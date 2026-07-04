use std::ffi;

pub mod instance;
pub use instance::Instance;

pub mod device;
pub use device::Device;

pub mod command_buffer;

pub mod surface;
pub use surface::Surface;

pub mod swapchain;
pub use swapchain::Swapchain;

pub mod prelude {
    pub use ash::{vk, khr, ext, prelude::VkResult};
}
use prelude::*;
use crate::prelude::*;

pub const FRAMES_IN_FLIGHT: usize = 3;
pub const ALLOCATION_CALLBACKS: Option<&'static vk::AllocationCallbacks<'static>> = None;
const VALIDATION_LAYER: &ffi::CStr = c"VK_LAYER_KHRONOS_validation";
const REQUIRED_INSTANCE_EXTENSIONS: &[&ffi::CStr] = &[
    khr::surface::NAME,
    ext::debug_utils::NAME,
];
const REQUIRED_DEVICE_EXTENSIONS: &[&ffi::CStr] = &[
    khr::swapchain::NAME,
];

use std::time;

pub struct GraphicsSystemInner {
    device: Device,
    instance: Instance,
}

impl GraphicsSystemInner {
    pub fn init(
        sdl_video: &sdl3::VideoSubsystem,
        title: &str,
    ) -> anyhow::Result<((), PresentationSubSystem)> {
        log_release!(LogType::Info, "Initialising GraphicsSystem and PresentationSubSystem...");
        let now = time::Instant::now();
        
        sdl_video.vulkan_load_library_default()?;
        
        let window = sdl_video.window(title, 800, 640)
            .resizable()
            .vulkan()
            .build()?;
        
        let get_instance_proc_addr = sdl_video.vulkan_get_proc_address_function()
            .ok_or(VulkanProcAddressError)?;
        
        let static_fn = ash::StaticFn {
            get_instance_proc_addr,
        };

        let sdl_instance_extensions = window.vulkan_instance_extensions()?
            .into_iter()
                .map(|s| ffi::CString::new(s).unwrap())
            .collect::<Vec<_>>();

        let instance = Instance::new(
            static_fn,
            sdl_instance_extensions.iter().map(|s| s.as_c_str()),
        )?;

        let surface = Surface::new(
            instance.clone(),
            window.clone()
        )?;

        let device = Device::new(
            instance.clone(),
            instance.first_device()?,
            &surface,
        )?;

        let swapchain = Swapchain::new(
            device.clone(),
            surface,
        )?;


        let elapsed_ms = now.elapsed().as_millis();
        log_release!(LogType::Info, "Finished initialising GraphicsSystem and PresentationSubSystem ({}ms).", elapsed_ms);

        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo {
                    flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                    queue_family_index: device.get_queue_family_index(device::QueueType::Graphics),
                    ..Default::default()
                },
                ALLOCATION_CALLBACKS,
            )
        }?;

        let command_buffers = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo {
                    command_pool,
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: FRAMES_IN_FLIGHT as u32,
                    ..Default::default()
                },
            )
        }?;

        let frames = command_buffers.into_iter()
            .map(|command_buffer| unsafe {
                Ok(Frame {
                    command_buffer,
                    fence: device.create_fence(
                        &vk::FenceCreateInfo {
                            flags: vk::FenceCreateFlags::SIGNALED,
                            ..Default::default()
                        },
                        ALLOCATION_CALLBACKS,
                    )?,
                    image_aquired: device.create_semaphore(
                        &vk::SemaphoreCreateInfo {
                            ..Default::default()
                        },
                        ALLOCATION_CALLBACKS,
                    )?,
                    render_finished: device.create_semaphore(
                        &vk::SemaphoreCreateInfo {
                            ..Default::default()
                        },
                        ALLOCATION_CALLBACKS,
                    )?,
                })
            })
            .collect::<VkResult<Box<[_]>>>()?;

        Ok((
            (),
            PresentationSubSystem {
                command_pool,
                frames,
                current: 0,
                swapchain,
                window,
            }
        ))
    }
}

use std::ops;

/// Split into its own struct to allow the main graphics struct to be Send + Sync
pub struct PresentationSubSystem {
    command_pool: vk::CommandPool,
    frames: Box<[Frame]>,
    current: usize,
    //graphics: GraphicsSystem,
    swapchain: Swapchain,
    window: sdl3::video::Window,
}

struct Frame {
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    image_aquired: vk::Semaphore,
    render_finished: vk::Semaphore,
}

/*
impl PresentationSubSystem {
    pub fn present(&mut self) -> VkResult<()> {
        let frame = &self.frames[self.current];
        let device = &self.graphics.device;
        
        unsafe {
            device.wait_for_fences(
                &[frame.fence],
                false,
                u64::MAX,
            )?;

            device.reset_fences(&[frame.fence])?;

            let (image, image_view, image_index) = self.swapchain.acquire_next_image(
                frame.image_aquired,
            )?;

            device.begin_command_buffer(
                frame.command_buffer,
                &vk::CommandBufferBeginInfo {
                    flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                },
            )?;

            let barrier = vk::ImageMemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };

            device.cmd_pipeline_barrier2(
                frame.command_buffer,
                &vk::DependencyInfo {
                    image_memory_barrier_count: 1,
                    p_image_memory_barriers: &raw const barrier,
                    ..Default::default()
                },
            );

            let color_attachment = vk::RenderingAttachmentInfo {
                image_view,
                image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                clear_value: vk::ClearValue { color: vk::ClearColorValue::default() },
                ..Default::default()
            };

            let rendering_info = vk::RenderingInfo {
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent(),
                },
                layer_count: 1,
                color_attachment_count: 1,
                p_color_attachments: &raw const color_attachment,
                ..Default::default()
            };

            device.cmd_begin_rendering(
                frame.command_buffer,
                &rendering_info,
            );

            device.cmd_end_rendering(frame.command_buffer);

            let barrier = vk::ImageMemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
                dst_access_mask: vk::AccessFlags2::NONE,
                old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };

            device.cmd_pipeline_barrier2(
                frame.command_buffer,
                &vk::DependencyInfo {
                    image_memory_barrier_count: 1,
                    p_image_memory_barriers: &raw const barrier,
                    ..Default::default()
                },
            );
             
            device.end_command_buffer(frame.command_buffer)?;

            let wait_dst = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;

            device.queue_submit(
                device.get_queue_handle(vulkan_backend::device::QueueType::Graphics),
                &[vk::SubmitInfo {
                    wait_semaphore_count: 1,
                    p_wait_semaphores: &raw const frame.image_aquired,
                    p_wait_dst_stage_mask: &raw const wait_dst,
                    command_buffer_count: 1,
                    p_command_buffers: &raw const frame.command_buffer,
                    signal_semaphore_count: 1,
                    p_signal_semaphores: &raw const frame.render_finished,
                    ..Default::default()
                }],
                frame.fence,
            )?;

            let swapchain_handle = self.swapchain.handle();

            device.swapchain.queue_present(
                device.get_queue_handle(vulkan_backend::device::QueueType::Presentation),
                &vk::PresentInfoKHR {
                    wait_semaphore_count: 1,
                    p_wait_semaphores: &raw const frame.render_finished,
                    swapchain_count: 1,
                    p_swapchains: &raw const swapchain_handle,
                    p_image_indices: &raw const image_index,
                    ..Default::default()
                },
            )?;
        }

        Ok(())
    }
}

impl Drop for PresentationSubSystem {
    fn drop(&mut self) {
        let device = &self.graphics.device;
        unsafe {
            device.queue_wait_idle(device.get_queue_handle(vulkan_backend::device::QueueType::Graphics)).unwrap();
            device.destroy_command_pool(
                self.command_pool,
                vulkan_backend::ALLOCATION_CALLBACKS,
            );
            for frame in &self.frames {
                device.destroy_fence(
                    frame.fence,
                    vulkan_backend::ALLOCATION_CALLBACKS,
                );
                device.destroy_semaphore(
                    frame.image_aquired,
                    vulkan_backend::ALLOCATION_CALLBACKS,
                );
                device.destroy_semaphore(
                    frame.render_finished,
                    vulkan_backend::ALLOCATION_CALLBACKS,
                );
            }
        }
    }
}

*/

use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct VulkanProcAddressError;

impl fmt::Display for VulkanProcAddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to retrieve Vulkan instance proc address")
    }
}
