use std::sync::Arc;
use std::ffi;
use std::ops;

use crate::prelude::*;
use super::prelude::*;

#[derive(Clone)]
pub struct Instance(Arc<InstanceInner>);

impl Instance {
    pub fn new<'a>(
        static_fn: ash::StaticFn,
        additional_required_extensions: impl IntoIterator<Item = &'a ffi::CStr>,
    ) -> VkResult<Self> {
        let entry = unsafe {
            ash::Entry::from_static_fn(static_fn)
        };

        let application_info = vk::ApplicationInfo {
            api_version: vk::API_VERSION_1_3,
            ..Default::default()
        };

        let instance_layers: &[*const ffi::c_char] = if cfg!(feature = "vulkan-validation-layers") {
            &[super::VALIDATION_LAYER.as_ptr()]
        } else {
            &[]
        };

        let required_extensions = super::REQUIRED_INSTANCE_EXTENSIONS.iter().cloned()
            .chain(additional_required_extensions)
            .collect::<Vec<_>>();

        let required_extension_ptrs = required_extensions.iter()
            .map(|&s| s.as_ptr())
            .collect::<Vec<_>>();

        let instance_create_info = vk::InstanceCreateInfo {
            p_application_info: &application_info,
            enabled_layer_count: instance_layers.len() as u32,
            pp_enabled_layer_names: instance_layers.as_ptr(),
            enabled_extension_count: required_extension_ptrs.len() as u32,
            pp_enabled_extension_names: required_extension_ptrs.as_ptr(),
            ..Default::default()
        };

        let instance = unsafe {
            entry.create_instance(
                &instance_create_info,
                super::ALLOCATION_CALLBACKS,
            )
        }?;

        let surface = khr::surface::Instance::new(&entry, &instance);
        let debug_utils = ext::debug_utils::Instance::new(&entry, &instance);

        Ok(Self(Arc::new(InstanceInner {
            debug_utils,
            surface,
            inner: instance,
            entry,
        })))
    }
    pub fn first_device(&self) -> VkResult<vk::PhysicalDevice> {
        let devices = unsafe {
            self.enumerate_physical_devices()
        }?;
        Ok(devices.into_iter().next().unwrap())
    }
}

impl Drop for InstanceInner {
    fn drop(&mut self) {
        log_verbose_debug!(LogType::Info, "Destroying Vulkan Instance.");
        unsafe {
            self.inner.destroy_instance(
                super::ALLOCATION_CALLBACKS,
            );
        }
    }
}

#[non_exhaustive]
pub struct InstanceInner {
    pub debug_utils: ext::debug_utils::Instance,
    pub surface: khr::surface::Instance,
    pub inner: ash::Instance,
    pub entry: ash::Entry,
}

impl ops::Deref for Instance {
    type Target = InstanceInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::Deref for InstanceInner {
    type Target = ash::Instance;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
