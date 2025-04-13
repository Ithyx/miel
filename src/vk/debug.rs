use std::ffi::CStr;

use ash::{ext, vk};
use thiserror::Error;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> u32 {
    let callback_data_deref = unsafe { *callback_data };
    let message_id_str = callback_data_deref.message_id_number.to_string();
    let message = if callback_data_deref.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { CStr::from_ptr(callback_data_deref.p_message) }.to_string_lossy()
    };

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            log::debug!("{message_severity:?} ({message_type:?}): [ID: {message_id_str}] {message}")
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("{message_severity:?} ({message_type:?}): [ID: {message_id_str}] {message}")
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("{message_severity:?} ({message_type:?}): [ID: {message_id_str}] {message}")
        }
        _ => {
            log::error!("{message_severity:?} ({message_type:?}): [ID: {message_id_str}] {message}")
        }
    }

    vk::FALSE
}

#[derive(Debug, Error)]
pub enum DUMCreationError {
    #[error("vulkan call to create the messenger failed")]
    VulkanError(vk::Result),
}

pub(crate) type DUMHandle = Option<(vk::DebugUtilsMessengerEXT, ext::debug_utils::Instance)>;

pub(crate) fn create_dum(
    entry: &ash::Entry,
    instance: &ash::Instance,
) -> Result<DUMHandle, DUMCreationError> {
    match cfg!(debug_assertions) {
        true => {
            let debug_utils_instance = ext::debug_utils::Instance::new(entry, instance);

            let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                        | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                        | vk::DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));
            let messenger =
                unsafe { debug_utils_instance.create_debug_utils_messenger(&create_info, None) }
                    .map_err(DUMCreationError::VulkanError)?;

            Ok(Some((messenger, debug_utils_instance)))
        }
        false => Ok(None),
    }
}
