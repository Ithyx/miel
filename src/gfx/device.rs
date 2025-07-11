use std::{cmp::Ordering, collections::HashMap, ffi::CStr, ops::Deref};

use ash::vk::{self, QueueFlags};
use thiserror::Error;

use super::{instance::Instance, surface::Surface};

fn vendor_id_to_str(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x1002 => "AMD",
        0x1010 => "ImgTec",
        0x10DE => "NVIDIA",
        0x13B5 => "ARM",
        0x5143 => "Qualcomm",
        0x8086 => "Intel",
        _ => "unknown",
    }
}

fn device_type_to_str(device_type: vk::PhysicalDeviceType) -> &'static str {
    match device_type {
        vk::PhysicalDeviceType::INTEGRATED_GPU => "integrated GPU",
        vk::PhysicalDeviceType::DISCRETE_GPU => "discrete GPU",
        vk::PhysicalDeviceType::VIRTUAL_GPU => "virtual GPU",
        vk::PhysicalDeviceType::CPU => "CPU",
        _ => "other",
    }
}

pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub graphics_qf_index: u32,
}

#[derive(Debug, Error)]
pub enum PhysicalDeviceSelectError {
    #[error("device enumeration failed")]
    DeviceEnumeration(vk::Result),
    #[error("device name fetching failed")]
    DeviceNameFetching(#[from] std::ffi::FromBytesUntilNulError),
    #[error("device name convertion failed")]
    DeviceNameConversion(#[from] std::str::Utf8Error),
    #[error("no valid device detected")]
    NoDevice,
}

impl PhysicalDevice {
    pub(crate) fn select(
        instance: &Instance,
        minimum_vk_version: u32,
        target_surface: &Surface,
    ) -> Result<Self, PhysicalDeviceSelectError> {
        log::debug!("Started physical device selection");
        // SAFETY: This is safe as long as the entry used to create the instance is still alive.
        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .map_err(PhysicalDeviceSelectError::DeviceEnumeration)?;

        // Get initial list of devices
        let physical_devices: Vec<_> = physical_devices
            .into_iter()
            .map(|handle| {
                // SAFETY: This is safe as long as the entry used to create the instance is still alive.
                (handle, unsafe {
                    instance.get_physical_device_properties(handle)
                })
            })
            .collect();

        log::debug!("Initial device list:");
        for (_, device_info) in &physical_devices {
            let device_name = device_info.device_name_as_c_str()?.to_str()?;
            let device_type = device_type_to_str(device_info.device_type);
            let device_vendor = vendor_id_to_str(device_info.vendor_id);
            log::debug!("\t{} [{}]: {}", device_name, device_vendor, device_type);
        }

        // Filter what we can even without queue families
        let compatible_devices: Vec<_> = physical_devices
            .into_iter()
            .filter(|&(device_handle, device_info)| {
                // VK API version check
                if device_info.api_version < minimum_vk_version {
                    return false;
                }

                // Device extension check
                let mut required_extensions: HashMap<&CStr, bool> = [
                    (ash::khr::swapchain::NAME, false),
                    (ash::khr::dynamic_rendering::NAME, false),
                    // Other required device extensions go here
                ]
                .into();
                // SAFETY: This is safe as long as the entry used to create the instance is still alive.
                let supported_extensions = unsafe {
                    instance.enumerate_device_extension_properties(device_handle)
                }
                .inspect_err(|err| {
                    log::warn!(
                        "Failed to query device extensions for device {} ({err}), ignoring.",
                        device_info
                            .device_name_as_c_str()
                            .unwrap_or(c"INVALID")
                            .to_str()
                            .unwrap_or("INVALID")
                    );
                })
                .unwrap_or(vec![]);

                for extension in &supported_extensions {
                    let extension_name = extension.extension_name_as_c_str().unwrap_or(c"");
                    if let Some(extension_check) = required_extensions.get_mut(extension_name) {
                        *extension_check = true;
                    }
                }

                for &extension_check in required_extensions.values() {
                    if !(extension_check) {
                        return false;
                    }
                }

                true
            })
            .collect();

        log::debug!("Device list after initial compatibility check:");
        for (_, device_info) in &compatible_devices {
            let device_name = device_info.device_name_as_c_str()?.to_str()?;
            let device_type = device_type_to_str(device_info.device_type);
            let device_vendor = vendor_id_to_str(device_info.vendor_id);
            log::debug!("\t{} [{}]: {}", device_name, device_vendor, device_type);
        }

        // Filter devices withtout the queue families we need
        let mut compatible_queue_families: Vec<_> = compatible_devices
            .into_iter()
            .filter_map(|(device_handle, device_info)| {
                // SAFETY: This is safe as long as the entry used to create the instance is still alive.
                let qf_properties =
                    unsafe { instance.get_physical_device_queue_family_properties(device_handle) };
                for (qf_index, queue_family) in qf_properties.iter().enumerate() {
                    let qf_index = qf_index as u32;
                    if !queue_family.queue_flags.contains(QueueFlags::GRAPHICS) {
                        continue;
                    }
                    if !queue_family.queue_flags.contains(QueueFlags::COMPUTE) {
                        continue;
                    }

                    let device = Self {
                        handle: device_handle,
                        properties: device_info,
                        graphics_qf_index: qf_index,
                    };

                    // SAFETY: This is safe as long as the entry used to create this loader is still alive.
                    let is_surface_compatible = unsafe {
                        target_surface.loader.get_physical_device_surface_support(
                            device_handle,
                            qf_index,
                            target_surface.handle,
                        )
                    }
                    .inspect_err(|err| {
                        log::warn!(
                            "Failed to get surface compatibility for device {} ({err}), ignoring.",
                            device.debug_string()
                        );
                    })
                    .unwrap_or(false);
                    if !is_surface_compatible {
                        continue;
                    }

                    return Some(device);
                }

                None
            })
            .collect();

        log::debug!("Device list after queue family compatibility check:");
        for device in &compatible_queue_families {
            log::debug!("\t{}", device.debug_string());
        }

        compatible_queue_families.sort_by(|a, b| {
            let mut ordering = Ordering::Equal;
            if a.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                && b.properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU
            {
                ordering = Ordering::Greater;
            }
            if a.properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU
                && b.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
            {
                ordering = Ordering::Less;
            }

            ordering
        });

        log::debug!("Device list after ordering:");
        for device in &compatible_queue_families {
            log::debug!("\t{}", device.debug_string());
        }

        let selected_device = compatible_queue_families
            .into_iter()
            .next()
            .ok_or(PhysicalDeviceSelectError::NoDevice)?;

        log::info!("Physical device selection result:");
        log::info!("{}", selected_device.debug_string());

        Ok(selected_device)
    }

    pub fn debug_string(&self) -> String {
        let device_name = self
            .properties
            .device_name_as_c_str()
            .unwrap_or(c"INVALID")
            .to_str()
            .unwrap_or("INVALID");
        let device_type = device_type_to_str(self.properties.device_type);
        let device_vendor = vendor_id_to_str(self.properties.vendor_id);
        format!("{} [{}]: {}", device_name, device_vendor, device_type)
    }
}

pub struct DeviceQueue {
    pub handle: vk::Queue,
    pub family_index: u32,
}

impl Deref for DeviceQueue {
    type Target = vk::Queue;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

pub struct Device {
    pub loader: ash::Device,
    pub graphics_queue: DeviceQueue,
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.loader
    }
}

#[derive(Debug, Error)]
pub enum DeviceCreateError {
    #[error("vulkan call to create the device failed")]
    VulkanCreation(vk::Result),
}

impl Device {
    pub(crate) fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<Self, DeviceCreateError> {
        let features = vk::PhysicalDeviceFeatures::default();
        let mut dynamic_rendering_feature =
            vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);

        let extensions = [
            ash::khr::swapchain::NAME.as_ptr(),
            ash::khr::dynamic_rendering::NAME.as_ptr(),
        ];

        let queue_priorities = [1.0];
        let queue_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(physical_device.graphics_qf_index)
            .queue_priorities(&queue_priorities)];

        let create_info = vk::DeviceCreateInfo::default()
            .enabled_features(&features)
            .enabled_extension_names(&extensions)
            .queue_create_infos(&queue_infos)
            .push_next(&mut dynamic_rendering_feature);

        // SAFETY: This is safe as long as the entry used to create the instance is still alive.
        let loader = unsafe { instance.create_device(physical_device.handle, &create_info, None) }
            .map_err(DeviceCreateError::VulkanCreation)?;

        // SAFETY: This is safe as long as the entry used to create this loader is still alive.
        let graphics_queue_handle =
            unsafe { loader.get_device_queue(physical_device.graphics_qf_index, 0) };
        let graphics_queue = DeviceQueue {
            handle: graphics_queue_handle,
            family_index: physical_device.graphics_qf_index,
        };

        Ok(Self {
            loader,
            graphics_queue,
        })
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        log::debug!("destroying logical device");
        // SAFETY: This is safe as long as the entry used to create this loader is still alive.
        unsafe { self.destroy_device(None) };
    }
}
