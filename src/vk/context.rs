use std::ffi::CString;

use ash::vk;
use thiserror::Error;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::utils::ThreadSafeRef;

use super::{
    allocator::{Allocator, AllocatorCreateError},
    debug::{DUMCreationError, DUMessenger},
    device::{Device, DeviceCreateError, PhysicalDevice, PhysicalDeviceSelectError},
    instance::{Instance, InstanceCreateError},
    surface::{DeviceSetupError, Surface, SurfaceCreateError},
    swapchain::{Swapchain, SwapchainCreateError},
};

pub struct ContextCreateInfo {
    pub application_name: CString,
    pub application_version: u32,
}

pub(crate) struct Context {
    pub swapchain: Swapchain,

    pub allocator_ref: ThreadSafeRef<Allocator>,

    pub device_ref: ThreadSafeRef<Device>,
    pub physical_device: PhysicalDevice,
    pub surface: Surface,
    pub du_messenger: Option<DUMessenger>,
    pub instance: Instance,
    pub entry: ash::Entry,
}

#[derive(Debug, Error)]
pub enum ContextCreateError {
    #[error("unable to get necessary handles from window")]
    InvalidWindow(#[from] winit::raw_window_handle::HandleError),

    #[error("vulkan library loading failed")]
    VulkanLoad(#[from] ash::LoadingError),

    #[error("instance creation failed")]
    InstanceCreation(#[from] InstanceCreateError),

    #[error("debug utils messenger creation failed")]
    DUMCreation(#[from] DUMCreationError),

    #[error("surface creation failed")]
    SurfaceCreation(#[from] SurfaceCreateError),

    #[error("physical device selection failed")]
    PhysicalDeviceSelection(#[from] PhysicalDeviceSelectError),

    #[error("physical device selection failed")]
    DeviceCreation(#[from] DeviceCreateError),

    #[error("surface format selection failed")]
    SurfaceFormatSelection(#[from] DeviceSetupError),

    #[error("GPU allocator creation failed")]
    AllocatorCreation(#[from] AllocatorCreateError),

    #[error("swapchain creation failed")]
    SwapchainCreation(#[from] SwapchainCreateError),
}

impl Context {
    pub fn create(
        window: &Window,
        create_info: &ContextCreateInfo,
    ) -> Result<Self, ContextCreateError> {
        let window_handle = window.window_handle()?.as_raw();
        let display_handle = window.display_handle()?.as_raw();

        let vk_version = vk::make_api_version(0, 1, 3, 0);

        // SAFETY: This is basically foreign code execution, and there is not way to properly ensure safety
        // here. It is unfortunately an uncontrollable risk we must accept.
        let entry = unsafe { ash::Entry::load() }?;
        let instance = Instance::create(
            &entry,
            &create_info.application_name,
            create_info.application_version,
            vk_version,
            display_handle,
        )?;
        let du_messenger = DUMessenger::create(&entry, &instance)?;
        let mut surface = Surface::create(&entry, &instance, display_handle, window_handle)?;
        let physical_device = PhysicalDevice::select(&instance, vk_version, &surface)?;
        surface.setup_from_device(&physical_device)?;

        // These reesources need to be stored as shared reeferences as they are often needed for
        // destruction anbd thus have to be stored in every sub-resource.
        let device_ref = ThreadSafeRef::new(Device::create(&instance, &physical_device)?);
        let allocator_ref = ThreadSafeRef::new(Allocator::create(
            &instance,
            &physical_device,
            &device_ref.lock(),
        )?);

        let swapchain = Swapchain::create(
            &instance,
            device_ref.clone(),
            &surface,
            vk::Extent2D {
                width: 1280,
                height: 720,
            },
        )?;

        Ok(Self {
            swapchain,

            allocator_ref,

            device_ref,
            physical_device,
            surface,
            du_messenger,
            instance,
            entry,
        })
    }
}
