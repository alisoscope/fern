
use crate::prelude::*;
use super::prelude::*;
use super::Device;
use super::device::QueueType;

/// A command pool that manages long lived resettable command buffers
pub struct CircularCommandPool {
    device: Device,
    queue_type: QueueType,
    level: vk::CommandBufferLevel,
    current: usize,
    command_pool: vk::CommandPool,
    command_buffers: Box<[vk::CommandBuffer]>,
}

impl CircularCommandPool {
    pub fn new(
        device: Device,
        len: u32,
        level: vk::CommandBufferLevel,
        queue_type: QueueType,
    ) -> VkResult<Self> {
        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo {
                    flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                    queue_family_index: device.get_queue_family_index(queue_type),
                    ..Default::default()
                },
                super::ALLOCATION_CALLBACKS,
            )
        }?;

        let command_buffers = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo {
                    command_pool,
                    level,
                    command_buffer_count: len,
                    ..Default::default()
                }
            )
        }?.into_boxed_slice();

        Ok(Self {
            device,
            queue_type,
            level,
            current: 0,
            command_pool,
            command_buffers,
        })
    }
}

impl Drop for CircularCommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(
                self.command_pool,
                super::ALLOCATION_CALLBACKS,
            );
        }
    }
}
